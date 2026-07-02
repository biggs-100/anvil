--- Forge Telescope extension.
---
--- Provides a Telescope picker (`:Telescope forge`) that lists available forge
--- runtimes and config variables. Selecting an item displays its details in a
--- floating window.
---
--- @module forge.telescope
local has_telescope, telescope = pcall(require, "telescope")
if not has_telescope then
  return {}
end

local pickers = require("telescope.pickers")
local finders = require("telescope.finders")
local sorters = require("telescope.sorters")
local actions = require("telescope.actions")
local action_state = require("telescope.actions.state")

local forge_mcp = require("forge.mcp")

--- Fetch forge context data and build picker items.
--- @param cb function(items: table) Callback with the items table
local function fetch_items(cb)
  local items = {}

  if not forge_mcp.is_ready() then
    table.insert(items, {
      value = "__disconnected__",
      ordinal = "",
      display = "[Forge is not connected]",
      description = "Open a project with forge.toml and try again.",
    })
    cb(items)
    return
  end

  forge_mcp.call_tool("forge_explain", { runtime = "all" }, function(err, result)
    if err then
      table.insert(items, {
        value = "__error__",
        ordinal = "",
        display = "[Error: " .. err .. "]",
        description = err,
      })
      cb(items)
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

    cb(items)
  end)
end

--- Open a floating window to display item details.
--- @param title string Window title
--- @param text string Content to display
local function open_float(title, text)
  local lines = vim.split(text, "\n")
  local width = math.floor(vim.o.columns * 0.8)
  local height = math.floor(vim.o.lines * 0.8)

  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_option(buf, "modifiable", true)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  vim.api.nvim_buf_set_option(buf, "modifiable", false)
  vim.api.nvim_buf_set_option(buf, "bufhidden", "wipe")
  vim.api.nvim_buf_set_option(buf, "syntax", "markdown")

  vim.api.nvim_open_win(buf, true, {
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

  vim.api.nvim_buf_set_keymap(buf, "n", "q", ":q<CR>", { noremap = true, silent = true, desc = "Close float" })
  vim.api.nvim_buf_set_keymap(buf, "n", "<Esc>", ":q<CR>", { noremap = true, silent = true, desc = "Close float" })
end

--- Forge resources picker.
---
--- Lists runtimes and config details from the forge environment.
--- Selecting an item shows its full details in a floating window.
--- @param opts? table Telescope picker options
local function forge_picker(opts)
  opts = opts or {}

  pickers.new(opts, {
    prompt_title = "Forge Resources",
    finder = finders.new_dynamic({
      fn = function()
        local results = {}
        fetch_items(function(items)
          results = items
        end)
        vim.wait(5000, function()
          return #results > 0
        end)
        return results
      end,
    }),
    sorter = sorters.get_generic_fuzzy_sorter(),
    attach_mappings = function(_, map)
      map("i", "<CR>", function(prompt_bufnr)
        local selection = action_state.get_selected_entry()
        actions.close(prompt_bufnr)
        if selection and selection.value ~= "__disconnected__" and selection.value ~= "__error__" then
          open_float("Forge: " .. selection.value, selection.description)
        end
      end)
      return true
    end,
  }):find()
end

--- Register the forge telescope extension.
return telescope.register_extension({
  setup = function(ext_config, _config)
    -- No setup needed for now
  end,
  exports = {
    forge = forge_picker,
  },
})
