# anvil-bundle Specification

## Purpose

Anvil has no portable distribution format. This spec defines `anvil bundle` and `anvil restore` — commands to produce and consume a deterministic, self-verifying `.anvil` archive of Anvil descriptors (manifests, lockfiles, metadata). Runtime binaries are never included.

## Requirements

### Requirement: Bundle Format

The `.anvil` archive MUST be tar+gzip. Internal entries in deterministic order:

```
anvil.toml     # original manifest
anvil.lock     # pinned runtimes
metadata.json  # non-sensitive context summary
bundle.sha256  # SHA-256 checksums
```

Entry order MUST be sorted by filename. Each entry MUST use the stored filename as its archive path (no directory prefix). The bundle MUST NOT contain any directory entries or entry padding that varies between runs — same inputs MUST produce identical bytes.

#### Scenario: Deterministic archive from same workspace

- GIVEN a workspace with known `anvil.toml` and `anvil.lock`
- WHEN `anvil bundle` is run twice
- THEN both outputs MUST be byte-identical

#### Scenario: Archive structure on disk

- GIVEN a valid workspace
- WHEN `anvil bundle` completes
- THEN `project.anvil` MUST contain entries in order: anvil.toml, anvil.lock, metadata.json, bundle.sha256

### Requirement: anvil bundle Command

`anvil bundle` MUST produce a `.anvil` archive written to `project.anvil` (or `--output` path). It MUST error if `anvil.toml` is absent. It MUST exclude `.anvil/`, `anvil.secrets`, and `anvil.env`. The `metadata.json` SHOULD include runtime versions, platform info, workspace name — never secrets.

#### Scenario: Bundle a valid project

- GIVEN a workspace with `anvil.toml`, `anvil.lock`, and pinned runtimes
- WHEN the user runs `anvil bundle`
- THEN `project.anvil` is created containing all four internal entries with valid checksums

#### Scenario: Bundle without anvil.toml

- GIVEN a directory without `anvil.toml`
- WHEN the user runs `anvil bundle`
- THEN the command MUST exit with a clear error: "anvil.toml not found"

#### Scenario: Bundle with explicit output path

- GIVEN a valid workspace
- WHEN the user runs `anvil bundle --output /tmp/env.anvil`
- THEN the archive is written to `/tmp/env.anvil`

### Requirement: anvil restore Command

`anvil restore` MUST read a `.anvil` archive, verify every file against `bundle.sha256` before writing anything to disk, then extract `anvil.toml` and `anvil.lock` to the current directory. After extraction, it MUST run `anvil up` to download and sync the pinned runtimes. If any checksum mismatch is detected, the command MUST reject the archive, write NOTHING, and report the expected vs. actual hash. The `--force` flag SHOULD allow overwriting existing files.

#### Scenario: Restore to empty directory

- GIVEN an empty directory and a valid `project.anvil` archive
- WHEN the user runs `anvil restore project.anvil`
- THEN `anvil.toml` and `anvil.lock` are written, and `anvil up` is invoked

#### Scenario: Restore with checksum mismatch

- GIVEN a tampered `project.anvil` where `anvil.toml` content does not match its checksum in `bundle.sha256`
- WHEN the user runs `anvil restore project.anvil`
- THEN the command MUST reject the archive, write no files, and display the expected and actual SHA-256 hashes

#### Scenario: Restore with --force overwrite

- GIVEN a directory with existing `anvil.toml` and a valid `project.anvil`
- WHEN the user runs `anvil restore --force project.anvil`
- THEN existing files are overwritten, and `anvil up` runs

### Requirement: Security

The bundle MUST exclude `.anvil/`, `anvil.secrets`, and any `anvil.env` file. The restore command MUST verify every file's SHA-256 checksum against `bundle.sha256` before extracting. If `anvil.env` exists in the workspace, the bundle SHOULD warn that environment variables are masked (not included) in the archive.

#### Scenario: Secrets excluded from bundle

- GIVEN a workspace containing `anvil.secrets` and `anvil.env`
- WHEN `anvil bundle` runs
- THEN neither file appears in the archive, and if `anvil.env` exists, a warning is printed

### Requirement: Verification

After a successful restore, `anvil status` MUST report `Ready` state — all runtimes pinned and present.

#### Scenario: Post-restore verification

- GIVEN a restored environment
- WHEN the user runs `anvil status`
- THEN the output shows `Ready`
