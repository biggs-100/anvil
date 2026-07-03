--- Anvil Neovim plugin entry point.
---
--- Registers four user commands (AnvilStatus, AnvilDoctor, AnvilExplain, AnvilRun),
--- sets up BufRead autocmd for anvil.toml, and cleans up on VimLeave.
---
--- Also provides a Telescope extension (`:Telescope anvil`) when telescope.nvim
--- is available.
---
--- Usage:
---   -- lazy.nvim
---   {
---     "anvil/anvil",
---     ft = { "toml" },
---     config = function()
---       require("anvil").setup({})
---     end,
---   }
---
---@module anvil
local M = {}

---@type table
local defaults = {
  --- When true, auto-start anvil MCP on BufRead of anvil.toml
  auto_start = true,
  --- Command to use for the anvil binary
  anvil_cmd = "anvil",
}

local mcp = nil

--- Setup the Anvil Neovim plugin.
---
--- Call this from your plugin configuration (e.g., lazy.nvim `config`).
---@param opts? table|nil Configuration options (see defaults above)
function M.setup(opts)
  local config = vim.tbl_deep_extend("force", defaults, opts or {})
  mcp = require("anvil.mcp")

  -- Create user commands
  vim.api.nvim_create_user_command("AnvilStatus", function()
    require("anvil.ui").show_status(mcp)
  end, { desc = "Show Anvil environment status" })

  vim.api.nvim_create_user_command("AnvilDoctor", function()
    require("anvil.ui").show_diagnose(mcp)
  end, { desc = "Run Anvil diagnostics and populate quickfix list" })

  vim.api.nvim_create_user_command("AnvilExplain", function(cmd_opts)
    local runtime = cmd_opts.args
    if runtime == "" then
      runtime = vim.fn.input("Runtime name (e.g., node, python): ")
    end
    if runtime and runtime ~= "" then
      require("anvil.ui").show_explain(mcp, runtime)
    end
  end, {
    desc = "Explain a specific runtime configuration",
    nargs = "?",
    complete = function()
      return { "node", "python", "ruby", "go", "rust" }
    end,
  })

  vim.api.nvim_create_user_command("AnvilRun", function(cmd_opts)
    local input = cmd_opts.args
    if input == "" then
      input = vim.fn.input("Command to run in anvil environment: ")
    end
    if input and input ~= "" then
      require("anvil.ui").show_run(mcp, input)
    end
  end, {
    desc = "Run a command in the anvil environment",
    nargs = "?",
  })

  -- Autocmd: auto-start anvil MCP when opening anvil.toml
  if config.auto_start then
    vim.api.nvim_create_autocmd("BufRead", {
      pattern = "anvil.toml",
      group = vim.api.nvim_create_augroup("anvil_start", { clear = true }),
      callback = function()
        if not mcp.is_ready() then
          mcp.start(function(err)
            if err then
              vim.notify("Anvil: " .. err, vim.log.levels.ERROR)
            else
              vim.notify("Anvil MCP connected", vim.log.levels.INFO)
            end
          end)
        end
      end,
    })
  end

  -- Cleanup on VimLeave
  vim.api.nvim_create_autocmd("VimLeavePre", {
    group = vim.api.nvim_create_augroup("anvil_cleanup", { clear = true }),
    callback = function()
      mcp.stop()
    end,
  })

  -- Setup Telescope extension if telescope is available
  local ok, _ = pcall(require, "telescope")
  if ok then
    pcall(function()
      require("telescope").load_extension("anvil")
    end)
  end
end

--- Return the mcp module for direct access (e.g., for debugging or custom usage).
---@return table
function M.get_mcp()
  return mcp
end

return M
