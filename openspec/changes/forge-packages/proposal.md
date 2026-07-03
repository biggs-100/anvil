# Proposal: Forge Packages

## Intent

Eliminate manual `pip install` after `forge up`. Users declare `[packages]` in forge.toml and forge auto-installs dependencies using the forge-managed python.

## Scope

### In Scope
- `[packages]` section parsing in forge.toml (pip + requirements.txt)
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
- `config-engine`: New `[packages]` section in forge.toml, deserialized into typed config

## Approach

1. Add `PackagesConfig` struct to forge config model with `pip: Option<String>`
2. Parse `[packages]` during forge.toml loading
3. After runtime sync completes, if python is installed and `[packages.pip]` is set:
   - Resolve forge-managed python binary path
   - Validate requirements.txt exists
   - Spawn `pip install -r <requirements.txt>` with output streaming
   - On failure, report error without rolling back runtime installation

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/forge-core/src/config.rs` | Modified | New `[packages]` deserialization |
| `crates/forge-core/src/runtime/manager.rs` | Modified | Post-sync hook for package install |
| `crates/forge-core/src/packages/` | New | Package installer module |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| pip not available in extracted python | Low | Clear error guiding user to check python dist |
| Long install blocks forge up UX | Med | Stream output; timeout considered for later |
| Conflicting deps in requirements.txt | Low | pip handles resolution; forge relays exit code |

## Rollback Plan

Remove `[packages]` from forge.toml. Package install is additive — runtime state is never rolled back on pip failure.

## Dependencies

- forge-managed python runtime (already exists)

## Success Criteria

- [ ] `forge up` with `[packages.pip] = "requirements.txt"` installs listed deps
- [ ] Install output shown in terminal
- [ ] Missing requirements.txt produces a clear error
- [ ] No `[packages]` section = no behavior change
