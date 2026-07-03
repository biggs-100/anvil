# Proposal: Publish ACP and AMS as Open Specifications

## Intent

Forge has shipped all core protocols and formats (frozen in Core 1.0). External tools cannot interoperate without reverse-engineering the wire format. Publish ACP (Forge Context Protocol) and AMS (Forge Manifest Specification) as formal, versioned, open spec documents so other tools can integrate without reimplementing the core.

## Scope

### In Scope
- **ACP spec**: handshake flow, wire format, schema, provider/exporter interface, security rules, capability negotiation
- **AMS spec**: `anvil.toml` schema, `anvil.lock` schema, `anvil.env` format, profile resolution, variable interpolation, precedence rules
- Both published as Markdown in `docs/specs/`

### Out of Scope
- New protocol features, version bumps, or implementation changes
- ARRS specification (already documented separately)
- Publishing process (website, registry) — document only

## Capabilities

> Contract between proposal and specs phases.

### New Capabilities
- `acp-spec`: Anvil Context Protocol — versioned open specification covering handshake, schema, providers, exporters, adapters, and security
- `ams-spec`: Anvil Manifest Specification — versioned open specification covering anvil.toml, anvil.lock, anvil.env, resolution stack, and interpolation

### Modified Capabilities
- None (pure documentation — existing implementation specs in `openspec/specs/` are unchanged)

## Approach

Write two specification documents in `docs/specs/`:
1. `docs/specs/fcp-v1.md` — extracted from `crates/anvil-core/src/context/` and existing openspec/ context specs. Covers protocol version 1.0.0.
2. `docs/specs/fms-v1.md` — extracted from `manifest.rs`, `types.rs`, `environment.rs`, `resolver.rs`. Covers the full manifest format, lockfile schema, env file parsing, 8-level resolution stack, and `${...}` interpolation.

Documents codify existing frozen behavior — no new design decisions.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `docs/specs/fcp-v1.md` | New | ACP specification document |
| `docs/specs/fms-v1.md` | New | AMS specification document |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Spec drifts from implementation | Low | Core 1.0 frozen; spec extracted from code, not written from scratch |
| Openspec/ specs and published spec diverge | Low | Published spec references same behaviors already spec'd in openspec/ |

## Rollback Plan

Revert is simply `git rm docs/specs/fcp-v1.md docs/specs/fms-v1.md`. No code, protocol, or schema changes to roll back.

## Dependencies

- None (all implementations exist and are frozen)

## Success Criteria

- [ ] `docs/specs/fcp-v1.md` published, reviewed, and accurate against implementation
- [ ] `docs/specs/fms-v1.md` published, reviewed, and accurate against implementation
- [ ] Both documents versioned (v1) with changelog section for future revisions
