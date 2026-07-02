# Forge Guide

Practical examples for using Forge in real projects.

---

## Quick Start

```bash
# Install
curl -fsSL https://forge.sh/install | sh

# Initialize a project
cd my-project
forge init

# Configure runtimes
cat >> forge.toml <<EOF
[runtimes]
node = "20.11.0"
python = "3.12.0"
llvm = "18.1.0"
jdk = "21.0.2"
EOF

# Resolve, lock, and download everything
forge up

# Use it
forge run node --version
forge run python --version
forge run clang --version
forge run java --version
```

---

## Everyday Workflows

### Developing with Forge

```bash
# Start a subshell with all runtimes configured
forge shell

# Inside the subshell:
node app.js
python manage.py runserver
clang -o program main.c
javac Main.java && java Main
```

### Running a single command

```bash
# No subshell needed
forge run npx create-react-app my-app
forge run pip install -r requirements.txt
```

### Checking environment health

```bash
# Quick check
forge status

# Full diagnostics
forge doctor

# Deep diagnostics
forge doctor --deep

# Machine-readable diagnostics
forge doctor --json
```

---

## Configuration & Secrets

### Environment variables

```toml
# forge.toml
[config.definitions.DATABASE_URL]
type = "string"
required = true

[config.definitions.MAX_CONNECTIONS]
type = "integer"
default = 10
```

```bash
# Set environment variables
forge env set DATABASE_URL postgres://localhost/mydb
forge env set MAX_CONNECTIONS 20

# List all variables with their sources
forge explain config
```

### Secrets (encrypted)

```bash
# Store a secret (uses OS keyring, falls back to encrypted file)
forge secret set API_KEY sk-abc123

# Retrieve
forge secret get API_KEY

# Check keyring health
forge secret doctor
```

### Profiles

```toml
# forge.toml
[profile.production.env]
DATABASE_URL = "postgres://prod-db:5432/mydb"
NODE_ENV = "production"

[profile.development.env]
DATABASE_URL = "postgres://localhost:5432/mydb"
NODE_ENV = "development"
```

```bash
# Use a specific profile
FORGE_PROFILE=production forge run node app.js

# See which profile is active
forge explain profile
```

---

## Project Distribution

### Bundle — share a project without the repo

```bash
# Create a portable .forge archive
cd my-project
forge bundle

# Send project.forge to a teammate
# They can restore it anywhere:
forge restore project.forge
forge up
```

### Snapshot — save and restore state

```bash
# Before a risky upgrade
forge snapshot --name before-node-upgrade

# Try the upgrade
# If something breaks:
forge snapshot restore before-node-upgrade

# List all snapshots
forge snapshot list
```

---

## Policies

```toml
# forge.toml
[policy]
allow_network = false      # Block network during builds
require_hashes = true       # Reject unverified downloads
minimum_health = 80         # Require 80% health score
forbid_runtimes = ["node"]  # Block node in this project
```

```bash
# If a policy blocks an operation, you'll see:
# POLICY VIOLATION: allow_network — Network access is disabled
# (current: false, expected: true)
```

---

## SDK Usage

### Rust

```rust
use forge_sdk::Forge;

let forge = Forge::new()?;
let status = forge.status().await?;
println!("{}", status);
```

### Go

```go
import "github.com/user/forge/sdk-go"

client := sdkgo.New()
defer client.Close()
status, _ := client.Status()
fmt.Println(status)
```

### Python

```python
from forge_sdk import Forge

client = Forge()
print(client.status())
client.close()
```

### TypeScript

```typescript
import { Forge } from "@forge/sdk";

const client = new Forge();
const status = await client.status();
console.log(status);
await client.disconnect();
```

---

## AI Integration

### MCP Server (for AI agents)

```bash
# Start MCP server
forge mcp
```

Any MCP-compatible client (Claude Code, Continue.dev, etc.) connects to Forge via:
- **Resources**: `forge://context/active` — full project context
- **Tools**: `forge_run`, `forge_shell`, `forge_sync`, `forge_plan`, `forge_explain`, `forge_doctor`
- **Prompts**: `forge:status`, `forge:diagnose`, `forge:explain`

### AI Agent Context

```bash
# Get structured context for AI agents
forge ai context

# AI-specific diagnostics
forge ai doctor
```

---

## TUI Dashboard

```bash
forge tui

# Navigation:
#   1 — Dashboard (health, runtimes, status)
#   2 — Runtimes (installed runtimes with state)
#   3 — Diagnostics (findings, health score)
#   4 — History (operation timeline)
#   j/k — scroll
#   r   — refresh
#   q   — quit
```

---

## Benchmark

```bash
# Measure performance
forge benchmark

# Machine-readable output
forge benchmark --json
```

---

## Explain Everything

```bash
# Runtime details
forge explain node

# Operation history
forge explain operation op-1234567890

# Context extraction details
forge explain context

# Resolved configuration with sources
forge explain config

# Active profile
forge explain profile
```

---

## IDE Integration

### VS Code

1. Install the extension from VSIX or marketplace
2. Open a project with `forge.toml`
3. Click the Forge icon in the sidebar
4. Use commands: `Forge: Status`, `Forge: Diagnose`, `Forge: Explain Runtime`, `Forge: Run`

### Neovim

```lua
-- lazy.nvim
{
  dir = "/path/to/extensions/neovim",
  opts = {}
}
```

`:ForgeStatus` — floating window  
`:ForgeDoctor` — quickfix list  
`:ForgeExplain node` — runtime details  
`:ForgeRun node app.js` — terminal output  

---

## Toolchains Reference

| Runtime   | forge.toml                     | Providers           |
|-----------|--------------------------------|---------------------|
| Node.js   | `node = "20.11.0"`             | NodeProvider        |
| Python    | `python = "3.12.0"`            | PythonProvider      |
| Bun       | `bun = "1.0.0"`                | BunProvider         |
| Go        | `go = "1.22.0"`                | GoProvider          |
| Rust      | `rust = "1.77.0"`              | RustProvider        |
| LLVM      | `llvm = "18.1.0"`              | LlvmProvider        |
| JDK       | `jdk = "21.0.2"`               | JdkProvider         |

---

## Troubleshooting

```bash
# Forge not found
forge doctor

# Runtimes not syncing
forge repair

# Check what forge would do without executing
forge plan

# View operation history
forge history

# Trace a specific operation
forge trace op-1234567890

# Stream live events
forge events --live
```
