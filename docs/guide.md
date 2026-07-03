# Anvil Guide

Practical examples for using Anvil in real projects.

---

## Quick Start

```bash
# Install
curl -fsSL https://anvil.dev/install | sh

# Initialize a project
cd my-project
anvil init

# Configure runtimes
cat >> anvil.toml <<EOF
[runtimes]
node = "20.11.0"
python = "3.12.0"
llvm = "18.1.0"
jdk = "21.0.2"
EOF

# Resolve, lock, and download everything
anvil up

# Use it
anvil run node --version
anvil run python --version
anvil run clang --version
anvil run java --version
```

---

## Everyday Workflows

### Developing with Anvil

```bash
# Start a subshell with all runtimes configured
anvil shell

# Inside the subshell:
node app.js
python manage.py runserver
clang -o program main.c
javac Main.java && java Main
```

### Running a single command

```bash
# No subshell needed
anvil run npx create-react-app my-app
anvil run pip install -r requirements.txt
```

### Checking environment health

```bash
# Quick check
anvil status

# Full diagnostics
anvil doctor

# Deep diagnostics
anvil doctor --deep

# Machine-readable diagnostics
anvil doctor --json
```

---

## Configuration & Secrets

### Environment variables

```toml
# anvil.toml
[config.definitions.DATABASE_URL]
type = "string"
required = true

[config.definitions.MAX_CONNECTIONS]
type = "integer"
default = 10
```

```bash
# Set environment variables
anvil env set DATABASE_URL postgres://localhost/mydb
anvil env set MAX_CONNECTIONS 20

# List all variables with their sources
anvil explain config
```

### Secrets (encrypted)

```bash
# Store a secret (uses OS keyring, falls back to encrypted file)
anvil secret set API_KEY sk-abc123

# Retrieve
anvil secret get API_KEY

# Check keyring health
anvil secret doctor
```

### Profiles

```toml
# anvil.toml
[profile.production.env]
DATABASE_URL = "postgres://prod-db:5432/mydb"
NODE_ENV = "production"

[profile.development.env]
DATABASE_URL = "postgres://localhost:5432/mydb"
NODE_ENV = "development"
```

```bash
# Use a specific profile
ANVIL_PROFILE=production anvil run node app.js

# See which profile is active
anvil explain profile
```

---

## Project Distribution

### Bundle — share a project without the repo

```bash
# Create a portable .anvil archive
cd my-project
anvil bundle

# Send project.anvil to a teammate
# They can restore it anywhere:
anvil restore project.anvil
anvil up
```

### Snapshot — save and restore state

```bash
# Before a risky upgrade
anvil snapshot --name before-node-upgrade

# Try the upgrade
# If something breaks:
anvil snapshot restore before-node-upgrade

# List all snapshots
anvil snapshot list
```

---

## Policies

```toml
# anvil.toml
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
use anvil_sdk::Anvil;

let anvil = Anvil::new()?;
let status = anvil.status().await?;
println!("{}", status);
```

### Go

```go
import "github.com/user/anvil/sdk-go"

client := sdkgo.New()
defer client.Close()
status, _ := client.Status()
fmt.Println(status)
```

### Python

```python
from anvil_sdk import Anvil

client = Anvil()
print(client.status())
client.close()
```

### TypeScript

```typescript
import { Anvil } from "@anvil/sdk";

const client = new Anvil();
const status = await client.status();
console.log(status);
await client.disconnect();
```

---

## AI Integration

### MCP Server (for AI agents)

```bash
# Start MCP server
anvil mcp
```

Any MCP-compatible client (Claude Code, Continue.dev, etc.) connects to Anvil via:
- **Resources**: `anvil://context/active` — full project context
- **Tools**: `anvil_run`, `anvil_shell`, `anvil_sync`, `anvil_plan`, `anvil_explain`, `anvil_doctor`
- **Prompts**: `anvil:status`, `anvil:diagnose`, `anvil:explain`

### AI Agent Context

```bash
# Get structured context for AI agents
anvil ai context

# AI-specific diagnostics
anvil ai doctor
```

---

## TUI Dashboard

```bash
anvil tui

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
anvil benchmark

# Machine-readable output
anvil benchmark --json
```

---

## Explain Everything

```bash
# Runtime details
anvil explain node

# Operation history
anvil explain operation op-1234567890

# Context extraction details
anvil explain context

# Resolved configuration with sources
anvil explain config

# Active profile
anvil explain profile
```

---

## IDE Integration

### VS Code

1. Install the extension from VSIX or marketplace
2. Open a project with `anvil.toml`
3. Click the Anvil icon in the sidebar
4. Use commands: `Anvil: Status`, `Anvil: Diagnose`, `Anvil: Explain Runtime`, `Anvil: Run`

### Neovim

```lua
-- lazy.nvim
{
  dir = "/path/to/extensions/neovim",
  opts = {}
}
```

`:AnvilStatus` — floating window  
`:AnvilDoctor` — quickfix list  
`:AnvilExplain node` — runtime details  
`:AnvilRun node app.js` — terminal output  

---

## Toolchains Reference

| Runtime   | anvil.toml                     | Providers           |
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
# Anvil not found
anvil doctor

# Runtimes not syncing
anvil repair

# Check what anvil would do without executing
anvil plan

# View operation history
anvil history

# Trace a specific operation
anvil trace op-1234567890

# Stream live events
anvil events --live
```
