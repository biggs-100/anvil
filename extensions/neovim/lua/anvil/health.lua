--- Health check for the Anvil Neovim plugin.
---
--- Run with `:checkhealth anvil` to verify:
---   - anvil binary is on PATH and accessible
---   - Neovim version meets minimum requirement (0.9+)
---   - MCP client state (if currently running)
---
---@module anvil.health
local M = {}

--- Minimum supported Neovim version.
local MIN_NVIM_VERSION = { 0, 9, 0 }

---@type table|nil
local mcp = nil

--- Run the health check.
function M.check()
  local ok, health = pcall(require, "health")
  if not ok then
    -- Neovim 0.10+ uses vim.health instead
    health = vim.health or vim
  end

  health.start("Anvil Plugin Health Check")

  -- 1. Check anvil binary on PATH
  _check_anvil_binary(health)

  -- 2. Check Neovim version
  _check_nvim_version(health)

  -- 3. Check MCP client state
  _check_mcp_state(health)
end

--- Verify the anvil binary is on PATH.
---@param health table
function _check_anvil_binary(health)
  local anvil_path = vim.fn.executable("anvil")

  if anvil_path and anvil_path > 0 then
    local path = vim.fn.exepath("anvil")
    health.ok("anvil binary found: " .. path)

    -- Check version
    local version_output = vim.fn.system("anvil --version 2>&1"):gsub("%s+$", "")
    if vim.v.shell_error == 0 then
      health.ok("anvil version: " .. version_output)
    else
      health.warn("Could not determine anvil version", version_output)
    end
  else
    health.error(
      "anvil binary not found on PATH",
      "Install anvil and ensure it is available in your PATH. See https://anvil.run/docs/install"
    )
  end
end

--- Verify that Neovim meets the minimum version requirement.
---@param health table
function _check_nvim_version(health)
  local nvim_version = vim.version()
  local min_ver = MIN_NVIM_VERSION

  if nvim_version then
    if nvim_version.major > min_ver[1]
        or (nvim_version.major == min_ver[1] and nvim_version.minor >= min_ver[2]) then
      health.ok(("Neovim %d.%d.%d meets minimum requirement"):format(
        nvim_version.major, nvim_version.minor, nvim_version.patch or 0
      ))
    else
      health.error(
        ("Neovim %d.%d.%d is too old"):format(
          nvim_version.major, nvim_version.minor, nvim_version.patch or 0
        ),
        ("Anvil requires Neovim %d.%d+. Please upgrade."):format(min_ver[1], min_ver[2])
      )
    end
  else
    health.warn("Could not determine Neovim version")
  end
end

--- Check the MCP client connection state.
---@param health table
function _check_mcp_state(health)
  local ok, mcp_mod = pcall(require, "anvil.mcp")
  if not ok then
    health.warn("anvil.mcp module not loaded (plugin may not be started yet)")
    return
  end

  local state = mcp_mod.state()

  if state == "ready" then
    health.ok("MCP client is connected and ready")
  elseif state == "connecting" then
    health.info("MCP client is connecting to anvil mcp...")
  elseif state == "disconnected" then
    health.warn("MCP client is disconnected. Open a project with anvil.toml to start.")
  elseif state == "error" then
    health.error(
      "MCP client is in error state",
      "Check that anvil binary is on PATH and try reloading the plugin."
    )
  end
end

-- Auto-run on :checkhealth
return M
