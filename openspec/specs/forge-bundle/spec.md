# forge-bundle Specification

## Purpose

Forge has no portable distribution format. This spec defines `forge bundle` and `forge restore` — commands to produce and consume a deterministic, self-verifying `.forge` archive of Forge descriptors (manifests, lockfiles, metadata). Runtime binaries are never included.

## Requirements

### Requirement: Bundle Format

The `.forge` archive MUST be tar+gzip. Internal entries in deterministic order:

```
forge.toml     # original manifest
forge.lock     # pinned runtimes
metadata.json  # non-sensitive context summary
bundle.sha256  # SHA-256 checksums
```

Entry order MUST be sorted by filename. Each entry MUST use the stored filename as its archive path (no directory prefix). The bundle MUST NOT contain any directory entries or entry padding that varies between runs — same inputs MUST produce identical bytes.

#### Scenario: Deterministic archive from same workspace

- GIVEN a workspace with known `forge.toml` and `forge.lock`
- WHEN `forge bundle` is run twice
- THEN both outputs MUST be byte-identical

#### Scenario: Archive structure on disk

- GIVEN a valid workspace
- WHEN `forge bundle` completes
- THEN `project.forge` MUST contain entries in order: forge.toml, forge.lock, metadata.json, bundle.sha256

### Requirement: forge bundle Command

`forge bundle` MUST produce a `.forge` archive written to `project.forge` (or `--output` path). It MUST error if `forge.toml` is absent. It MUST exclude `.forge/`, `forge.secrets`, and `forge.env`. The `metadata.json` SHOULD include runtime versions, platform info, workspace name — never secrets.

#### Scenario: Bundle a valid project

- GIVEN a workspace with `forge.toml`, `forge.lock`, and pinned runtimes
- WHEN the user runs `forge bundle`
- THEN `project.forge` is created containing all four internal entries with valid checksums

#### Scenario: Bundle without forge.toml

- GIVEN a directory without `forge.toml`
- WHEN the user runs `forge bundle`
- THEN the command MUST exit with a clear error: "forge.toml not found"

#### Scenario: Bundle with explicit output path

- GIVEN a valid workspace
- WHEN the user runs `forge bundle --output /tmp/env.forge`
- THEN the archive is written to `/tmp/env.forge`

### Requirement: forge restore Command

`forge restore` MUST read a `.forge` archive, verify every file against `bundle.sha256` before writing anything to disk, then extract `forge.toml` and `forge.lock` to the current directory. After extraction, it MUST run `forge up` to download and sync the pinned runtimes. If any checksum mismatch is detected, the command MUST reject the archive, write NOTHING, and report the expected vs. actual hash. The `--force` flag SHOULD allow overwriting existing files.

#### Scenario: Restore to empty directory

- GIVEN an empty directory and a valid `project.forge` archive
- WHEN the user runs `forge restore project.forge`
- THEN `forge.toml` and `forge.lock` are written, and `forge up` is invoked

#### Scenario: Restore with checksum mismatch

- GIVEN a tampered `project.forge` where `forge.toml` content does not match its checksum in `bundle.sha256`
- WHEN the user runs `forge restore project.forge`
- THEN the command MUST reject the archive, write no files, and display the expected and actual SHA-256 hashes

#### Scenario: Restore with --force overwrite

- GIVEN a directory with existing `forge.toml` and a valid `project.forge`
- WHEN the user runs `forge restore --force project.forge`
- THEN existing files are overwritten, and `forge up` runs

### Requirement: Security

The bundle MUST exclude `.forge/`, `forge.secrets`, and any `forge.env` file. The restore command MUST verify every file's SHA-256 checksum against `bundle.sha256` before extracting. If `forge.env` exists in the workspace, the bundle SHOULD warn that environment variables are masked (not included) in the archive.

#### Scenario: Secrets excluded from bundle

- GIVEN a workspace containing `forge.secrets` and `forge.env`
- WHEN `forge bundle` runs
- THEN neither file appears in the archive, and if `forge.env` exists, a warning is printed

### Requirement: Verification

After a successful restore, `forge status` MUST report `Ready` state — all runtimes pinned and present.

#### Scenario: Post-restore verification

- GIVEN a restored environment
- WHEN the user runs `forge status`
- THEN the output shows `Ready`
