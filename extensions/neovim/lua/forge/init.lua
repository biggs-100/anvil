--- Forge Neovim plugin entry point.
---
--- Registers four user commands (ForgeStatus, ForgeDoctor, ForgeExplain, ForgeRun),
--- sets up BufRead autocmd for forge.toml, and cleans up on VimLeave.
---
--- Also provides a Telescope extension (`:Telescope forge`) when telescope.nvim
--- is available.
---
--- Usage:
---   -- lazy.nvim
---   {
---     "forge/forge",
---     ft = { "toml" },
---     config = function()
---       require("forge").setup({})
---     end,
---   }
---
---@module forge
local M = {}

---@type table
local defaults = {
  --- When true, auto-start forge MCP on BufRead of forge.toml
  auto_start = true,
  --- Command to use for the forge binary
  forge_cmd = "forge",
}

local mcp = nil

--- Setup the Forge Neovim plugin.
---
--- Call this from your plugin configuration (e.g., lazy.nvim `config`).
---@param opts? table|nil Configuration options (see defaults above)
function M.setup(opts)
  local config = vim.tbl_deep_extend("force", defaults, opts or {})
  mcp = require("forge.mcp")

  -- Create user commands
  vim.api.nvim_create_user_command("ForgeStatus", function()
    require("forge.ui").show_status(mcp)
  end, { desc = "Show Forge environment status" })

  vim.api.nvim_create_user_command("ForgeDoctor", function()
    require("forge.ui").show_diagnose(mcp)
  end, { desc = "Run Forge diagnostics and populate quickfix list" })

  vim.api.nvim_create_user_command("ForgeExplain", function(cmd_opts)
    local runtime = cmd_opts.args
    if runtime == "" then
      runtime = vim.fn.input("Runtime name (e.g., node, python): ")
    end
    if runtime and runtime ~= "" then
      require("forge.ui").show_explain(mcp, runtime)
    end
  end, {
    desc = "Explain a specific runtime configuration",
    nargs = "?",
    complete = function()
      return { "node", "python", "ruby", "go", "rust" }
    end,
  })

  vim.api.nvim_create_user_command("ForgeRun", function(cmd_opts)
    local input = cmd_opts.args
    if input == "" then
      input = vim.fn.input("Command to run in forge environment: ")
    end
    if input and input ~= "" then
      require("forge.ui").show_run(mcp, input)
    end
  end, {
    desc = "Run a command in the forge environment",
    nargs = "?",
  })

  -- Autocmd: auto-start forge MCP when opening forge.toml
  if config.auto_start then
    vim.api.nvim_create_autocmd("BufRead", {
      pattern = "forge.toml",
      group = vim.api.nvim_create_augroup("forge_start", { clear = true }),
      callback = function()
        if not mcp.is_ready() then
          mcp.start(function(err)
            if err then
              vim.notify("Forge: " .. err, vim.log.levels.ERROR)
            else
              vim.notify("Forge MCP connected", vim.log.levels.INFO)
            end
          end)
        end
      end,
    })
  end

  -- Cleanup on VimLeave
  vim.api.nvim_create_autocmd("VimLeavePre", {
    group = vim.api.nvim_create_augroup("forge_cleanup", { clear = true }),
    callback = function()
      mcp.stop()
    end,
  })

  -- Setup Telescope extension if telescope is available
  local ok, _ = pcall(require, "telescope")
  if ok then
    pcall(function()
      require("telescope").load_extension("forge")
    end)
  end
end

--- Return the mcp module for direct access (e.g., for debugging or custom usage).
---@return table
function M.get_mcp()
  return mcp
end

return M
