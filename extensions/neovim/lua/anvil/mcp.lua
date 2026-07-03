--- MCP client for Neovim.
---
--- Spawns `anvil mcp` as a stdio subprocess via `vim.fn.jobstart`,
--- sends JSON-RPC 2.0 line-delimited messages, and routes responses
--- back to callers via a pending-request map.
---
--- Notifications (anvil/warning, anvil/error, anvil/state_changed) are
--- forwarded to `vim.diagnostic` and emitted via the module's callback API.
---
--- Usage:
---   local mcp = require("anvil.mcp")
---   mcp.start()  -- spawn + initialize handshake
---   mcp.request("tools/call", { name = "anvil_doctor" }, function(err, result)
---     if err then vim.notify(err, vim.log.levels.ERROR) end
---     -- handle result...
---   end)
---   mcp.stop()   -- shutdown + kill
---
---@module anvil.mcp
local M = {}

---@type number|nil Job ID from vim.fn.jobstart
local job_id = nil

---@type vim.api.keyset.channel|nil
local channel = nil

---@type table<number|string, {resolve: function, reject: function, timer: table|nil}>
local pending = {}

---@type number
local next_id = 1

---@type string 'connecting'|'ready'|'disconnected'|'error'
local state = "disconnected"

---@type table<string, function[]>
local notification_handlers = {}

---@type number
local REQUEST_TIMEOUT_MS = 30000

---@type number
local MAX_PENDING = 128

---@type number|nil
local bufnr = nil

---@type number|nil
local stderr_data = nil

-- ── Public API ─────────────────────────────────────────────────────────────

--- Get the current connection state.
---@return string
function M.state()
  return state
---

--- Set up a notification handler.
---@param method string Notification method name (e.g. "anvil/warning")
---@param handler function(params: table)
function M.on_notification(method, handler)
  if not notification_handlers[method] then
    notification_handlers[method] = {}
  end
  table.insert(notification_handlers[method], handler)
end

--- Remove a previously registered notification handler.
---@param method string
---@param handler function
function M.off_notification(method, handler)
  if not notification_handlers[method] then
    return
  end
  for i, h in ipairs(notification_handlers[method]) do
    if h == handler then
      table.remove(notification_handlers[method], i)
      return
    end
  end
end

--- Spawn `anvil mcp` and perform the MCP initialize handshake.
---
--- The callback receives `(err)` — nil on success, error message on failure.
---@param callback? function(err: string|nil)
function M.start(callback)
  if state == "connecting" or state == "ready" then
    if callback then
      callback("MCP client is already connected or connecting")
    end
    return
  end

  state = "connecting"

  -- Create a scratch buffer for stdout processing
  bufnr = vim.api.nvim_create_buf(false, true)

  -- Verify anvil exists on PATH before starting the job
  local anvil_check = vim.fn.executable("anvil")
  if anvil_check == 0 then
    state = "error"
    vim.notify(
      "anvil binary not found on PATH. Install anvil and try again.",
      vim.log.levels.ERROR
    )
    if callback then
      callback("anvil binary not found on PATH. Install anvil and try again.")
    end
    return
  end

  local stdout_data = {}

  job_id = vim.fn.jobstart("anvil mcp", {
    -- stdout: line-delimited JSON-RPC
    on_stdout = function(_, data)
      if not data then
        return
      end
      for _, line in ipairs(data) do
        if line and line ~= "" then
          M._handle_message(line)
        end
      end
    end,
    -- stderr: collect for error reporting
    on_stderr = function(_, data)
      if data then
        for _, line in ipairs(data) do
          if line and line ~= "" then
            stderr_data = line
          end
        end
      end
    end,
    -- exit handler
    on_exit = function(_, exit_code)
      state = "disconnected"

      -- Reject pending requests
      if exit_code ~= 0 and next(pending) ~= nil then
        local err_msg
        if stderr_data and (stderr_data:find("not found") or stderr_data:find("No such file")) then
          err_msg = "anvil binary not found on PATH. Install anvil and try again."
        else
          err_msg = "anvil mcp exited unexpectedly (code=" .. exit_code .. "): " .. (stderr_data or "unknown error")
        end
        for id, entry in pairs(pending) do
          if entry.timer then
            vim.fn.timer_stop(entry.timer)
          end
          entry.reject(err_msg)
        end
        pending = {}
      end

      job_id = nil
      channel = nil
    end,
  })

  if not job_id or job_id <= 0 then
    state = "error"
    vim.notify("Failed to start anvil mcp process", vim.log.levels.ERROR)
    if callback then
      callback("Failed to start anvil mcp process")
    end
    return
  end

  -- Send initialize request
  M._raw_request("initialize", {
    protocol_version = "2024-11-05",
    capabilities = {},
    client_info = { name = "anvil-neovim", version = "0.1.0" },
  }, function(err, _)
    if err then
      state = "error"
      vim.notify("Anvil MCP initialize failed: " .. err, vim.log.levels.ERROR)
      if callback then
        callback(err)
      end
      return
    end

    -- Send initialized notification (fire-and-forget)
    M._send_notification("notifications/initialized", {})

    state = "ready"
    if callback then
      callback(nil)
    end
  end)
end

--- Send a JSON-RPC request and invoke the callback with the response.
---
--- The callback receives `(err, result)` where result is the decoded JSON
--- response table (containing `result` or `error` fields).
---@param method string MCP method name
---@param params? table Request parameters
---@param callback function(err: string|nil, result: table|nil)
function M.request(method, params, callback)
  M._raw_request(method, params, function(err, resp)
    if err then
      callback(err, nil)
      return
    end
    if resp.error then
      callback(resp.error.message or "MCP error", nil)
      return
    end
    callback(nil, resp.result)
  end)
end

--- Convenience wrapper: call an MCP tool.
---@param name string Tool name
---@param args? table Tool arguments
---@param callback function(err: string|nil, result: table|nil)
function M.call_tool(name, args, callback)
  M.request("tools/call", { name = name, arguments = args or {} }, callback)
end

--- Convenience wrapper: get a prompt.
---@param name string Prompt name
---@param args? table Prompt arguments
---@param callback function(err: string|nil, result: table|nil)
function M.get_prompt(name, args, callback)
  M.request("prompts/get", { name = name, arguments = args or {} }, callback)
end

--- Gracefully shut down the MCP connection.
---
--- Sends shutdown notification, closes stdin, waits 3 seconds, then force-kills.
function M.stop()
  if not job_id or state == "disconnected" then
    return
  end

  state = "disconnected"

  -- Send shutdown notification
  M._send_notification("shutdown", {})

  -- Close stdin (sends EOF to anvil mcp)
  vim.fn.jobclose(job_id, "stdin")

  -- Wait 3 seconds then SIGKILL
  vim.defer_fn(function()
    if job_id then
      local ok = pcall(vim.fn.jobstop, job_id)
      if not ok then
        -- Already cleaned up
      end
      job_id = nil
      channel = nil
    end
  end, 3000)

  -- Also try SIGTERM immediately
  vim.fn.jobstop(job_id)
  job_id = nil
  channel = nil
end

--- Check if the MCP client is in a ready state.
---@return boolean
function M.is_ready()
  return state == "ready"
end

-- ── Internal ───────────────────────────────────────────────────────────────

--- Send a JSON-RPC request with raw response (error object included if present).
---@param method string
---@param params? table
---@param callback function(err: string|nil, resp: table|nil)
function M._raw_request(method, params, callback)
  if state ~= "ready" and state ~= "connecting" then
    callback("MCP client is not connected", nil)
    return
  end

  if #pending >= MAX_PENDING then
    callback("Too many pending requests", nil)
    return
  end

  local id = next_id
  next_id = next_id + 1

  local timer = vim.fn.timer_start(REQUEST_TIMEOUT_MS, function()
    pending[id] = nil
    callback("Request timed out: " .. method, nil)
  end)

  pending[id] = {
    resolve = function(resp)
      if timer then
        vim.fn.timer_stop(timer)
      end
      callback(nil, resp)
    end,
    reject = function(err_msg)
      if timer then
        vim.fn.timer_stop(timer)
      end
      callback(err_msg, nil)
    end,
    timer = timer,
  }

  local request = vim.fn.json_encode({
    jsonrpc = "2.0",
    id = id,
    method = method,
    params = params or {},
  })

  local ok = vim.fn.chansend(job_id, request .. "\n")
  if not ok or ok <= 0 then
    pending[id] = nil
    if timer then
      vim.fn.timer_stop(timer)
    end
    callback("Failed to write to anvil mcp stdin", nil)
  end
end

--- Send a notification (no id, no response expected).
---@param method string
---@param params? table
function M._send_notification(method, params)
  if not job_id then
    return
  end

  local notification = vim.fn.json_encode({
    jsonrpc = "2.0",
    method = method,
    params = params or {},
  })

  pcall(vim.fn.chansend, job_id, notification .. "\n")
end

--- Handle an incoming JSON-RPC message (response or notification).
---@param raw string
function M._handle_message(raw)
  local ok, msg = pcall(vim.fn.json_decode, raw)
  if not ok or type(msg) ~= "table" then
    return
  end

  if msg.id ~= nil then
    M._handle_response(msg)
  else
    M._handle_notification(msg)
  end
end

--- Route a response to the matching pending request.
---@param msg table
function M._handle_response(msg)
  local id = msg.id
  local entry = pending[id]
  if not entry then
    return
  end

  pending[id] = nil
  entry.resolve(msg)
end

--- Handle a server-sent notification.
---@param msg table
function M._handle_notification(msg)
  local method = msg.method
  if not method then
    return
  end

  -- Forward to registered handlers
  if notification_handlers[method] then
    for _, handler in ipairs(notification_handlers[method]) do
      pcall(handler, msg.params)
    end
  end

  -- Default handling for anvil notifications
  if method == "anvil/warning" and msg.params then
    local p = msg.params
    if p.finding then
      vim.diagnostic.add(vim.diagnostic.severity.WARN, p.finding, {
        bufnr = 0,
        lnum = 0,
        col = 0,
        end_lnum = 0,
        end_col = 0,
      })
    end
  elseif method == "anvil/error" and msg.params then
    local p = msg.params
    if p.error then
      vim.notify(
        "[Anvil Error] " .. (p.operation or "unknown") .. ": " .. p.error,
        vim.log.levels.ERROR
      )
    end
  end
end

return M
