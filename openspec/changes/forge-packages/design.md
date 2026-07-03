# Design: Forge Packages

## Technical Approach

Add a `[packages]` section to `forge.toml` that declares pip requirements. After `Engine::sync()` completes (in the `Sync` and `Up` command handlers in forge-cli), check if python runtime is present and `packages.pip` is set — if so, run `pip install -r <file>` using the forge-managed python binary via `run_command_in_env`. The logic lives in a new `packages.rs` module in forge-core, invoked from main.rs.

## Architecture Decisions

| Option | Tradeoffs | Decision |
|--------|-----------|----------|
| Hook in Engine::sync() vs caller dispatch | Engine::sync() would couple package install to every sync call (including repair/clean). Caller dispatch keeps it scoped to user-initiated commands. | Dispatch from forge-cli; Engine stays focused on runtime sync. |
| New module vs inline in main.rs | inline keeps surface small (single use) but is untestable. A module in forge-core lets us unit test resolution + validation. | New `forge_core::packages` module with a single public fn. |
| run_command_in_env vs raw Command | `run_command_in_env` already sets up PATH and env vars — exactly what we need to find the forge-managed python `pip` module. | Reuse `run_command_in_env`. |

## Data Flow

```
forge.toml
  │
  ├── config.runtimes["python"] ──→ lockfile ──→ extracted binary path
  └── config.packages.pip ────────→ requirements.txt path
                                        │
Engine::sync() completes                │
  │                                     │
  ▼                                     ▼
forge-cli dispatch (Sync/Up) ──→ packages::install_pip_deps()
                                     │
                                     ▼
                             run_command_in_env(
                               python3,
                               ["-m", "pip", "install", "-r", "requirements.txt"],
                               env: {},
                               bin_dirs: [cache/python/{ver}/extracted/bin],
                             )
                                     │
                                     ▼
                             stdout/stderr ──→ user terminal (passthrough)
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/forge-core/src/manifest.rs` | Modify | Add `PackagesConfig` struct, add `packages` field to `ForgeConfig` |
| `crates/forge-core/src/packages.rs` | Create | New module: `install_pip_deps()` — resolve python binary, validate file, invoke pip |
| `crates/forge-core/src/lib.rs` | Modify | Add `pub mod packages;` and re-export |
| `crates/forge-cli/src/main.rs` | Modify | After `engine.sync().await?` in `Sync` and `Up` handlers, call `packages::install_pip_deps()` |

## Interfaces / Contracts

```rust
// manifest.rs
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackagesConfig {
    #[serde(default)]
    pub pip: Option<String>,  // path to requirements.txt
}

// ForgeConfig gets a new field:
pub packages: Option<PackagesConfig>,

// packages.rs — single entry point
pub fn install_pip_deps(
    workspace_root: &Path,
    cache_dir: &Path,
) -> Result<(), String>
```

`install_pip_deps` returns `Ok(())` if no `[packages]` section exists or install succeeds. Returns `Err(String)` if pip is configured but python runtime is missing, requirements file is missing, or pip exits non-zero.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | `PackagesConfig` TOML deserialization | Parse `[packages]\npip = "reqs.txt"` and assert field is `Some("reqs.txt")` |
| Unit | `install_pip_deps` — missing python | Mock lockfile without python; assert error message |
| Unit | `install_pip_deps` — missing requirements | Mock lockfile with python; assert file-not-found error |
| Unit | `install_pip_deps` — no packages config | Call without packages; assert Ok(()) |
| Integration | Pip install with real forge setup | E2E: `forge up` + `[packages.pip]` → verify pip ran |

## Migration / Rollout

No migration required. Package install is additive — runtime state is never rolled back on pip failure. Users add `[packages]` to forge.toml when ready.

## Open Questions

- [ ] Windows: python binary is `python.exe`, not `python3` — need platform-aware binary name resolution
- [ ] Should pip install errors be fatal (fail `forge up`) or warnings? Spec says non-zero exit code, but UX question remains
