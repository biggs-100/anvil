# Proposal: Anvil Packages

## Intent

Eliminate manual `pip install` after `anvil up`. Users declare `[packages]` in anvil.toml and anvil auto-installs dependencies using the forge-managed python.

## Scope

### In Scope
- `[packages]` section parsing in anvil.toml (pip + requirements.txt)
- Post-sync pip install using forge-managed python binary
- Show install output to user
- Error if requirements.txt doesn't exist
- Error if pip is set but no python runtime configured

### Out of Scope
- Package managers beyond pip (npm, cargo, etc.)
- Dependency resolution or lockfile management
- Version pinning inside requirements.txt

## Capabilities

### New Capabilities
- `package-installer`: Post-sync package dependency installation using forge-managed runtimes

### Modified Capabilities
- `config-engine`: New `[packages]` section in anvil.toml, deserialized into typed config

## Approach

1. Add `PackagesConfig` struct to anvil config model with `pip: Option<String>`
2. Parse `[packages]` during anvil.toml loading
3. After runtime sync completes, if python is installed and `[packages.pip]` is set:
   - Resolve forge-managed python binary path
   - Validate requirements.txt exists
   - Spawn `pip install -r <requirements.txt>` with output streaming
   - On failure, report error without rolling back runtime installation

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/anvil-core/src/config.rs` | Modified | New `[packages]` deserialization |
| `crates/anvil-core/src/runtime/manager.rs` | Modified | Post-sync hook for package install |
| `crates/anvil-core/src/packages/` | New | Package installer module |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| pip not available in extracted python | Low | Clear error guiding user to check python dist |
| Long install blocks anvil up UX | Med | Stream output; timeout considered for later |
| Conflicting deps in requirements.txt | Low | pip handles resolution; anvil relays exit code |

## Rollback Plan

Remove `[packages]` from anvil.toml. Package install is additive — runtime state is never rolled back on pip failure.

## Dependencies

- forge-managed python runtime (already exists)

## Success Criteria

- [ ] `anvil up` with `[packages.pip] = "requirements.txt"` installs listed deps
- [ ] Install output shown in terminal
- [ ] Missing requirements.txt produces a clear error
- [ ] No `[packages]` section = no behavior change
