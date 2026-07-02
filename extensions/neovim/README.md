# Forge Neovim Plugin

Thin MCP client that connects Neovim to the [Forge](https://forge.run) engine via `forge mcp`. Displays environment status, diagnostics, runtime explanations, and command output in native Neovim UI surfaces.

## Requirements

- **Neovim** 0.9+
- **forge** binary on PATH ([install guide](https://forge.run/docs/install))

## Installation

### [lazy.nvim](https://github.com/folke/lazy.nvim)

```lua
{
  "forge/forge",
  ft = { "toml" },
  config = function()
    require("forge").setup({
      auto_start = true,
    })
  end,
}
```

### Manual (packer, paq, etc.)

```lua
use {
  "forge/forge",
  config = function()
    require("forge").setup({})
  end,
}
```

## Commands

| Command | Description | MCP Method |
|---------|-------------|------------|
| `:ForgeStatus` | Show environment state in floating window | `prompts/get forge:status` |
| `:ForgeDoctor` | Run diagnostics and populate quickfix list | `prompts/get forge:diagnose` |
| `:ForgeExplain {runtime}` | Show runtime configuration in floating window | `tools/call forge_explain` |
| `:ForgeRun {cmd}` | Execute a command in forge environment | `tools/call forge_run` |

For `:ForgeExplain` and `:ForgeRun`, you can pass the argument directly:

```vim
:ForgeExplain node
:ForgeRun node --version
```

Or run without arguments to be prompted:

```vim
:ForgeExplain
Runtime name (e.g., node, python): node
```

## Features

### Floating Windows

`ForgeStatus` and `ForgeExplain` display MCP responses in centered floating windows with rounded borders and markdown syntax highlighting. Press `q` or `Esc` to close.

### Quickfix Diagnostics

`ForgeDoctor` parses the diagnostic report and populates the quickfix list. Open with `:copen` after running.

### Terminal Output

`ForgeRun` displays command output in a scratch buffer. JSON results are pretty-printed automatically.

### Telescope Integration

If [telescope.nvim](https://github.com/nvim-telescope/telescope.nvim) is installed, Forge registers a `forge` picker:

```
:Telescope forge
```

Lists available runtimes and configuration variables. Select an item to view details in a floating window.

### Diagnostics Notifications

The plugin subscribes to `forge/warning` and `forge/error` notifications and displays them via `vim.diagnostic` and `vim.notify`.

### Health Check

Run `:checkhealth forge` to verify:

- forge binary is on PATH
- Neovim version meets minimum requirements (0.9+)
- MCP client connection state

## Configuration

### `setup(opts)`

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `auto_start` | `boolean` | `true` | Auto-start forge MCP on BufRead of forge.toml |
| `forge_cmd` | `string` | `"forge"` | Command to use for the forge binary |

## Plugin Structure

```
extensions/neovim/
└── lua/forge/
    ├── init.lua      # Module entry, user commands, autocmds
    ├── mcp.lua       # MCP client via vim.fn.jobstart
    ├── ui.lua        # Floating windows, quickfix, terminal, Telescope
    └── health.lua    # :checkhealth provider
```
