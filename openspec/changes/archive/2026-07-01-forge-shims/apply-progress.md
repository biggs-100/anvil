# Implementation Progress: forge-shims

**Change**: forge-shims  
**Mode**: openspec  
**Workload mode**: `size:exception`

All tasks and remediation fixes have been successfully completed. All unit and integration tests compile and pass perfectly.

## Created/Modified Files

| File | Action | What Was Done |
|------|--------|---------------|
| `crates/forge-shim/Cargo.toml` | Created | Defined a lightweight dependency-free binary crate. |
| `crates/forge-shim/src/main.rs` | Modified | Added validation check for version header signature `# forge-shims-cache-v1` in `read_shims_cache`. Added unit test `test_cache_invalidation_incorrect_header`. |
| `Cargo.toml` | Modified | Registered `crates/forge-shim` in the workspace members. |
| `crates/forge-core/Cargo.toml` | Unchanged | Retained existing dependencies. |
| `crates/forge-core/src/lib.rs` | Modified | Implemented shims cache serialization mapping, regeneration triggers on lock and installation, gitignore incremental update helper, and core unit tests. |
| `crates/forge-cli/Cargo.toml` | Modified | Added the `dirs` crate dependency. |
| `crates/forge-cli/src/main.rs` | Modified | Modified `Commands::Setup` to accept a boolean flag `--uninstall`. Implemented `uninstall_shims` logic and `get_home_dir` support for custom home overrides. Added integration test `test_setup_and_uninstall_shims`. |

## Status

All tasks complete. Verification remediation successful. Ready for verification.
