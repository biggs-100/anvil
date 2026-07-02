## Exploration: forge-bootstrap

### Current State
Currently, the `forge` project is in a bootstrapping phase with an empty repository (only containing metadata folders). Development environment setups for developers and AI agents are typically manual, platform-dependent, and prone to environmental drift.

To design `forge` effectively, we must evaluate existing tools across three key dimensions: isolation style, configuration/reproducibility, and agent friendliness.

#### Tool Comparison Matrix
| Tool | Scope / Type | Core Pros | Core Cons | Agent Suitability |
| :--- | :--- | :--- | :--- | :--- |
| **Nix / Flakes** | Declarative System Package Manager | Pure hermetic reproducibility; massive package catalog. | Extremely high learning curve; Nix language is complex; poor native Windows support. | Low. Hard for agents to dynamically write Nix expressions or debug build failures. |
| **Devbox** | Nix Wrapper | Human-friendly JSON/TOML on top of Nix. | Inherits Nix dependencies and poor native Windows support (WSL2 required). | Medium. Easier configuration but complex failure states under the WSL/Nix boundary. |
| **Devcontainers**| Container-based Dev Environment | Standardized VM/Container isolation; great IDE support. | Heavyweight; requires Docker/Podman; slow startup; resource intensive. | Low-Medium. Agents struggle to run inside container scopes without complex mount setups. |
| **mise / asdf** | Polyglot Runtime Manager | Fast, directory-local tool mapping (Node, Python, etc.). | Focuses only on runtime versions, not system-level dependencies. | High. Simple configuration, fast execution, easy to query. |
| **Pixi** | Conda-based Package Manager | Fast, cross-platform native package installation. | Tied to Conda repository ecosystems; smaller general package footprint than Nix/Apt. | High. Extremely fast, robust dependency resolution. |
| **uv** | Python Package Installer/Resolver | Blazing fast Python environment management. | Python-specific; cannot manage Node, Go, or system-level tools. | High. Simple, deterministic, and fast, but too narrow in scope. |
| **Homebrew** | OS Package Manager (macOS/Linux) | Huge library of user packages; simple commands. | Non-reproducible; mutates state; version pinning is difficult. | Medium. Agents can install tools, but with no version stability. |
| **Winget/Scoop/Choco** | OS Package Managers (Windows) | Standard native Windows installations. | Mutate state; hard to version-pin; divergent command syntaxes. | Medium. Requires shell scripting; divergent CLI conventions. |

---

### Affected Areas
- `openspec/changes/forge-bootstrap/exploration.md` — This exploration file containing the baseline analysis.
- `Cargo.toml` — Will be created to define the Rust workspace, CLI dependencies (e.g., `clap`, `serde`, `tokio`), and targets.
- `src/` — Implementation directory for the CLI interface, version parsing, and environment/agent commands.

---

### Approaches

#### 1. Nix & Devcontainer Wrapper
`forge` acts as an orchestration layer wrapping Nix (for Linux/macOS) and Devcontainers/Docker (for Windows). It translates a project-level `forge.toml` file into Nix expressions or `.devcontainer/devcontainer.json` files, hiding their complexity behind unified commands.
- **Pros:**
  - Guarantees 100% hermetic reproducibility of the dev environment.
  - Reuses mature ecosystems (Nixpkgs and Devcontainers).
- **Cons:**
  - High host-system prerequisite load (Docker, WSL2, Nix).
  - Heavy performance overhead (slow container start times, heavy disk usage).
  - Windows becomes a second-class citizen relying on virtualization, making interactive agent workflows slow and complex.
- **Effort:** Medium

#### 2. Native Rust-Based Polyglot Runtime Manager with Native OS Fallbacks
`forge` is built as a pure, native Rust CLI that manages toolchains directly. It downloads precompiled runtime runtimes (e.g., Node.js, Python, Go, Ruby) and sets up a directory-local PATH (via shims or shell hooks, similar to `mise`). For native system packages, it integrates wrapper drivers for OS package managers (Homebrew on macOS, Winget/Scoop on Windows, Apt/Pacman on Linux) to bootstrap missing dependencies.
- **Pros:**
  - Zero heavyweight dependencies (no Docker/Nix required).
  - Fast execution and sub-millisecond startup times.
  - Native cross-platform execution (true native support on Windows, macOS, Linux).
  - Facilitates deep integration with AI agents through direct environment inspection and local self-healing.
- **Cons:**
  - System fallbacks are not perfectly hermetic (e.g., a package on Homebrew might differ slightly from Winget).
  - Requires maintaining or leveraging precompiled tool asset mappings.
- **Effort:** High

---

### Architecture Considerations for a Rust-Based Tool
Building `forge` in Rust provides several structural advantages:
1. **Single Binary Distribution:** Distributing `forge` as a single statically linked binary ensures that both humans and AI agents can install it instantly without python/node runtime bootstrapping issues.
2. **Deterministic Shell Activation:** To alter the active environment, `forge` can implement:
   - **Shell Hook (`forge hook`)**: Generates shell-specific code (Bash, Zsh, Fish, PowerShell) to dynamically modify the environment on directory change (`cd`).
   - **Execution Wrapping (`forge run -- <cmd>`)**: Spawns a child process with a custom environment map. This is highly reliable for AI agent scripts.
3. **Structured Outputs:** The CLI must support a global `--json` flag to return structured machine-readable outputs for every command, minimizing text-parsing issues for LLM-based agents.
4. **Concurrent Resolution:** Using `tokio` allows asynchronous downloading and extraction of multiple toolchains simultaneously.

---

### Design of AI Agent Integration

`forge` is designed with AI agents as first-class citizens, implemented through two primary commands:

#### 1. `forge ai context`
Provides a concise, structured data envelope describing the active workspace's state. When an agent enters a repository, it runs this command to immediately ingest:
- **Runtimes & Tooling:** Currently active versions (e.g. Node 20.11, Python 3.12).
- **Project Structure:** Major framework signatures (e.g. Next.js, Cargo workspace).
- **Commands & Scripts:** Available developer tasks (defined in `forge.toml` or package managers) and their run commands.
- **Environment Variables:** Necessary variables configured or missing.

*Example Output:*
```json
{
  "project_type": "rust_workspace",
  "active_runtimes": {
    "rust": "1.79.0",
    "node": "20.11.0"
  },
  "tasks": {
    "build": "cargo build --release",
    "test": "cargo test"
  },
  "env_vars": {
    "DATABASE_URL": {
      "status": "unset",
      "required": true
    }
  }
}
```

#### 2. `forge ai doctor`
An automated diagnostic and remediation utility. When an agent encounters a build/test failure or boots up a new project workspace, it calls `forge ai doctor`.
- **Diagnostics:** Checks if the correct toolchain version is installed, active env vars are set, dependencies are installed, and necessary services are running.
- **Remediation Plan:** Instead of reporting text errors that an agent must guess how to fix, it returns a list of issues along with explicit, pre-authorized shell commands to fix them.

*Example Output:*
```json
{
  "status": "unhealthy",
  "issues": [
    {
      "id": "missing_runtime",
      "severity": "critical",
      "tool": "node",
      "message": "Node.js v20.11.0 is required but not installed.",
      "remediation": "forge install node@20.11.0"
    },
    {
      "id": "missing_env_var",
      "severity": "warning",
      "message": "DATABASE_URL is required but unset.",
      "remediation": "echo 'DATABASE_URL=postgres://localhost:5432/db' >> .env"
    }
  ]
}
```
This enables the agent to parse the JSON and autonomously execute the remediation commands.

---

### Recommendation
**Approach 2 (Native Rust Polyglot Manager + OS Fallbacks)** is the recommended path. 
- **Rationale:** AI agents and developers on Windows face major hurdles when forced to run heavy VM/Nix layers. A native, lightweight Rust CLI ensures immediate startup speed and low cognitive friction. 
- **Agent Integration Plan:** First-class agent context (`forge ai context`) and diagnostic repair APIs (`forge ai doctor`) should be built directly into the core CLI domain model, outputting strict JSON schemas.

---

### Risks
- **Windows Runtime Paths & Shims:** Managing Windows execution shims (e.g., `.exe` wrapper generation or path modifications in PowerShell/CMD) is notoriously complex. We must test Powershell path resolution early.
- **System Package Non-Determinism:** OS-level fallbacks (Homebrew, Winget) are mutable and can introduce version drift across environments. `forge` should encourage hermetic precompiled toolchains first, reserving system packages only for library compilation dependencies (e.g. openssl, libxml).
- **Environment Hijacking / Shell Bloat:** Dynamic environment modification can sometimes cause shell slow-downs or conflict with other managers (e.g. nvm, pyenv). The activation hook must be optimized to run in under 5ms.

---

### Ready for Proposal
**Yes**. The orchestrator should proceed to the `sdd-propose` phase to draft the implementation proposal, define the `forge.toml` schema, and detail the CLI interface.
