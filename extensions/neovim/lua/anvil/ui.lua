--- Anvil UI module.
---
--- Renders MCP responses in native Neovim UI surfaces:
---   - Floating windows for status and explain
---   - Quickfix list for diagnostics
---   - Terminal buffer for run output
---   - Telescope picker for runtimes/config (optional)
---
---@module anvil.ui
local M = {}

--- Default floating window dimensions (relative to editor).
local FLOAT_OPTS = {
  style = "minimal",
  border = "rounded",
  width = 0.8,
  height = 0.8,
  row = 0.1,
  col = 0.1,
  relative = "editor",
}

-- ── Commands ───────────────────────────────────────────────────────────────

--- Show anvil environment status in a floating window.
---
--- Sends `prompts/get` with `anvil:status` and renders markdown in a float.
---@param mcp table The MCP client module
function M.show_status(mcp)
  if not mcp or not mcp.is_ready() then
    vim.notify("Anvil is not connected. Open a project with anvil.toml.", vim.log.levels.WARN)
    return
  end

  mcp.get_prompt("anvil:status", {}, function(err, result)
    if err then
      vim.notify("Anvil: " .. err, vim.log.levels.ERROR)
      return
    end

    local text = "No status available."
    if result and result.messages and result.messages[1] and result.messages[1].content then
      text = result.messages[1].content.text or text
    end

    _open_float("Anvil Status", text)
  end)
end

--- Run anvil diagnostics and populate the quickfix list.
---
--- Sends `prompts/get` with `anvil:diagnose` and parses findings into quickfix entries.
---@param mcp table The MCP client module
function M.show_diagnose(mcp)
  if not mcp or not mcp.is_ready() then
    vim.notify("Anvil is not connected. Open a project with anvil.toml.", vim.log.levels.WARN)
    return
  end

  mcp.get_prompt("anvil:diagnose", {}, function(err, result)
    if err then
      vim.notify("Anvil: " .. err, vim.log.levels.ERROR)
      return
    end

    local text = "No diagnostic data."
    if result and result.messages and result.messages[1] and result.messages[1].content then
      text = result.messages[1].content.text or text
    end

    -- Parse findings into quickfix entries
    local qf_list = {}
    -- Match: `- **CODE** [SEVERITY] message`
    for code, sev, msg in text:gmatch("%-%s*%*%*(.-)%*%*%s*%[(.-)%]%s*(.+)") do
      local lnum = 0
      table.insert(qf_list, {
        lnum = lnum,
        text = ("[%s] [%s] %s"):format(code:match("^%s*(.-)%s*$") or code, sev, msg),
        type = (sev:upper() == "ERROR" or sev:upper() == "CRITICAL") and "E" or "W",
      })
    end

    -- If no structured findings, add the full text as a single entry
    if #qf_list == 0 then
      table.insert(qf_list, {
        lnum = 0,
        text = text:sub(1, 500),
        type = "I",
      })
    end

    vim.fn.setqflist(qf_list, "r")
    vim.api.nvim_command("copen")
    vim.notify("Anvil diagnose complete — " .. #qf_list .. " finding(s)", vim.log.levels.INFO)
  end)
end

--- Show runtime explanation in a floating window.
---
--- Calls `tools/call` with `anvil_explain` and renders the JSON result.
---@param mcp table The MCP client module
---@param runtime string Runtime name to explain
function M.show_explain(mcp, runtime)
  if not mcp or not mcp.is_ready() then
    vim.notify("Anvil is not connected. Open a project with anvil.toml.", vim.log.levels.WARN)
    return
  end

  mcp.call_tool("anvil_explain", { runtime = runtime }, function(err, result)
    if err then
      vim.notify("Anvil: " .. err, vim.log.levels.ERROR)
      return
    end

    local text = "No explanation available."
    if result and result.content and result.content[1] then
      text = result.content[1].text or text
    end

    -- Try to pretty-print JSON
    local ok, parsed = pcall(vim.fn.json_decode, text)
    if ok then
      text = vim.fn.json_encode(parsed)
    end

    _open_float("Anvil: Explain " .. runtime, text)
  end)
end

--- Run a command in the anvil environment and show output in a terminal buffer.
---
--- Calls `tools/call` with `anvil_run`.
---@param mcp table The MCP client module
---@param input string Full command string (e.g., "node --version")
function M.show_run(mcp, input)
  if not mcp or not mcp.is_ready() then
    vim.notify("Anvil is not connected. Open a project with anvil.toml.", vim.log.levels.WARN)
    return
  end

  -- Split command into parts
  local parts = {}
  for part in input:gmatch("%S+") do
    table.insert(parts, part)
  end
  local cmd = parts[1]
  local args = {}
  for i = 2, #parts do
    table.insert(args, parts[i])
  end

  mcp.call_tool("anvil_run", { cmd = cmd, args = args }, function(err, result)
    if err then
      vim.notify("Anvil: " .. err, vim.log.levels.ERROR)
      return
    end

    local text = "(no output)"
    if result and result.content and result.content[1] then
      text = result.content[1].text or text
    end

    -- Open in a new scratch buffer
    local buf = vim.api.nvim_create_buf(false, true)
    vim.api.nvim_buf_set_option(buf, "bufhidden", "wipe")
    vim.api.nvim_buf_set_name(buf, "anvil://run-output")

    -- Pretty-print JSON
    local ok, parsed = pcall(vim.fn.json_decode, text)
    if ok then
      text = vim.fn.json_encode(parsed)
    end

    local lines = vim.split(text, "\n")
    vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)

    if result.is_error then
      table.insert(lines, "")
      table.insert(lines, "[ERROR] Command failed — see above for details.")
      vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
    end

    -- Open in a split window
    vim.api.nvim_set_current_buf(buf)
  end)
end

-- ── Telescope Picker ──────────────────────────────────────────────────────

--- Telescope picker for anvil resources.
---
--- Lists available runtimes and config variables from the anvil context.
--- Selecting an item shows details in a floating window.
function M.telescope_picker(mcp)
  return require("telescope.pickers").new({}, {
    prompt_title = "Anvil Resources",
    finder = require("telescope.finders").new_dynamic({
      fn = function()
        -- Fetch anvil context and build picker items
        local items = {}
        mcp.call_tool("anvil_explain", { runtime = "all" }, function(err, result)
          if err then
            vim.notify("Anvil: " .. err, vim.log.levels.ERROR)
            return
          end
          local text = ""
          if result and result.content and result.content[1] then
            text = result.content[1].text or ""
          end
          local ok, parsed = pcall(vim.fn.json_decode, text)
          if ok and type(parsed) == "table" then
            for key, value in pairs(parsed) do
              table.insert(items, {
                value = key,
                ordinal = key,
                display = key,
                description = vim.inspect(value),
              })
            end
          end
        end)
        return items
      end,
    }),
    sorter = require("telescope.sorters").get_generic_fuzzy_sorter(),
    attach_mappings = function(_, map)
      map("i", "<CR>", function(prompt_bufnr)
        local selection = require("telescope.actions.state").get_selected_entry()
        require("telescope.actions").close(prompt_bufnr)
        if selection then
          _open_float("Anvil: " .. selection.value, selection.description)
        end
      end)
      return true
    end,
  })
end

-- ── Internal Helpers ───────────────────────────────────────────────────────

--- Open a floating window with the given title and text content.
---@param title string Window title
---@param text string Text content (basic markdown supported)
function _open_float(title, text)
  local lines = vim.split(text, "\n")
  local width = math.floor(vim.o.columns * FLOAT_OPTS.width)
  local height = math.floor(vim.o.lines * FLOAT_OPTS.height)

  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_option(buf, "modifiable", true)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  vim.api.nvim_buf_set_option(buf, "modifiable", false)
  vim.api.nvim_buf_set_option(buf, "bufhidden", "wipe")

  -- Basic markdown highlighting
  vim.api.nvim_buf_set_option(buf, "syntax", "markdown")

  local win = vim.api.nvim_open_win(buf, true, {
    relative = "editor",
    width = width,
    height = height,
    row = math.floor((vim.o.lines - height) / 2),
    col = math.floor((vim.o.columns - width) / 2),
    style = "minimal",
    border = "rounded",
    title = " " .. title .. " ",
    title_pos = "center",
  })

  -- Keymaps for the float
  vim.api.nvim_buf_set_keymap(buf, "n", "q", ":q<CR>", { noremap = true, silent = true, desc = "Close float" })
  vim.api.nvim_buf_set_keymap(buf, "n", "<Esc>", ":q<CR>", { noremap = true, silent = true, desc = "Close float" })
end

return M
