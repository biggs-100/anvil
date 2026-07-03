# Anvil Neovim Plugin

Thin MCP client that connects Neovim to the [Anvil](https://anvil.run) engine via `anvil mcp`. Displays environment status, diagnostics, runtime explanations, and command output in native Neovim UI surfaces.

## Requirements

- **Neovim** 0.9+
- **anvil** binary on PATH ([install guide](https://anvil.run/docs/install))

## Installation

### [lazy.nvim](https://github.com/folke/lazy.nvim)

```lua
{
  "anvil/anvil",
  ft = { "toml" },
  config = function()
    require("anvil").setup({
      auto_start = true,
    })
  end,
}
```

### Manual (packer, paq, etc.)

```lua
use {
  "anvil/anvil",
  config = function()
    require("anvil").setup({})
  end,
}
```

## Commands

| Command | Description | MCP Method |
|---------|-------------|------------|
| `:AnvilStatus` | Show environment state in floating window | `prompts/get anvil:status` |
| `:AnvilDoctor` | Run diagnostics and populate quickfix list | `prompts/get anvil:diagnose` |
| `:AnvilExplain {runtime}` | Show runtime configuration in floating window | `tools/call anvil_explain` |
| `:AnvilRun {cmd}` | Execute a command in anvil environment | `tools/call anvil_run` |

For `:AnvilExplain` and `:AnvilRun`, you can pass the argument directly:

```vim
:AnvilExplain node
:AnvilRun node --version
```

Or run without arguments to be prompted:

```vim
:AnvilExplain
Runtime name (e.g., node, python): node
```

## Features

### Floating Windows

`AnvilStatus` and `AnvilExplain` display MCP responses in centered floating windows with rounded borders and markdown syntax highlighting. Press `q` or `Esc` to close.

### Quickfix Diagnostics

`AnvilDoctor` parses the diagnostic report and populates the quickfix list. Open with `:copen` after running.

### Terminal Output

`AnvilRun` displays command output in a scratch buffer. JSON results are pretty-printed automatically.

### Telescope Integration

If [telescope.nvim](https://github.com/nvim-telescope/telescope.nvim) is installed, Anvil registers an `anvil` picker:

```
:Telescope anvil
```

Lists available runtimes and configuration variables. Select an item to view details in a floating window.

### Diagnostics Notifications

The plugin subscribes to `anvil/warning` and `anvil/error` notifications and displays them via `vim.diagnostic` and `vim.notify`.

### Health Check

Run `:checkhealth anvil` to verify:

- anvil binary is on PATH
- Neovim version meets minimum requirements (0.9+)
- MCP client connection state

## Configuration

### `setup(opts)`

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `auto_start` | `boolean` | `true` | Auto-start anvil MCP on BufRead of anvil.toml |
| `anvil_cmd` | `string` | `"anvil"` | Command to use for the anvil binary |

## Plugin Structure

```
extensions/neovim/
└── lua/anvil/
    ├── init.lua      # Module entry, user commands, autocmds
    ├── mcp.lua       # MCP client via vim.fn.jobstart
    ├── ui.lua        # Floating windows, quickfix, terminal, Telescope
    ├── health.lua    # :checkhealth provider
    └── telescope.lua # Telescope extension
```
