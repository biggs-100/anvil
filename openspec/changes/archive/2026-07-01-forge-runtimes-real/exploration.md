## Exploration: forge-runtimes-real

### Current State
The `crates/anvil-core/src/lib.rs` file contains a mock runtime resolution framework (`resolve_runtime_lock`). While it constructs placeholder templates for Node.js, Bun, Go, Python, and Rust, it uses hardcoded mock sizes and a dummy SHA-256 hash (`e3b0c442...` which represents an empty SHA-256 hash). The URLs mapped are partially incorrect or incomplete for certain platform/arch combinations (especially Windows Arm64 and Python standalone releases).

### Affected Areas
- `crates/anvil-core/src/lib.rs` — Needs modifications to `resolve_runtime_lock` to dynamically construct URLs, fetch/verify metadata, and handle actual runtime archives.
- `crates/anvil-core/Cargo.toml` — Might need new dependencies like `xz2` or `lzma-rs` to decompress `.tar.xz` files if we use `.tar.xz` archives (particularly for Rust).

### Approaches
1. **Dynamic Remote Querying (API-First)** — Query official remote JSON/TOML metadata endpoints (e.g., `nodejs.org/dist/index.json`, `go.dev/dl/?mode=json`, `channel-rust-stable.toml`) to resolve versions and fetch sizes/SHA-256 hashes dynamically.
   - Pros: Always up-to-date; no CLI updates needed for new runtime versions; accurate hashes directly from vendors.
   - Cons: High network latency; breaks entirely offline; potential rate limits (e.g., GitHub API for Bun/Python).
   - Effort: Medium

2. **Cached Local Database (Offline-First / Embedded)** — Store a static mapping of stable runtime versions, size, and SHA-256 checksums inside the CLI binary or in a bundled metadata file.
   - Pros: Maximum security; zero network latency for resolution; works offline; completely deterministic.
   - Cons: Requires a CLI update to support newly released runtime versions; maintenance overhead to keep versions updated.
   - Effort: Low

3. **Hybrid Registry (Local Registry with Remote Fallback)** — Attempt resolution via a local embedded registry of common stable versions first. If a version is missing or loose, fetch from remote APIs and cache the resolved metadata under `.anvil/metadata_cache.toml`.
   - Pros: Combines offline performance for standard versions with the flexibility to resolve new versions dynamically.
   - Cons: Increased implementation complexity; handles multiple failure modes (network offline, parse errors).
   - Effort: High

### Recommendation
We recommend the **Hybrid Registry (Option 3)**. It provides a robust, developer-friendly experience by ensuring that common stable toolchain versions (e.g., LTS versions of Node, Go, Bun) resolve instantly and work offline if already cached, while still allowing the installation of newer or arbitrary versions on demand. To mitigate the missing `.tar.xz` support in `anvil-core` for Rust standalone toolchains, we recommend adding the `xz2` crate to decompress `.tar.xz` archives, as this is the standard package format for Rust.

### Risks
- **Network Dependency and Rate-Limiting**: Remote APIs (like GitHub's Release API) are subject to rate limiting and temporary downtime. Caching resolved coordinates locally is critical to mitigate this.
- **Decompression Capabilities**: Rust toolchains are primarily packaged as `.tar.xz` files. The current extractor in `lib.rs` only supports `.tar.gz` and `.zip`. Implementing `.tar.xz` extraction adds a dependency (`xz2` or `lzma-rs` which brings liblzma or a pure Rust decoder).
- **Windows ARM64 Support**: Some runtimes (like Go and Bun) have only recently added native Windows ARM64 archives, or do not offer them in portable `.zip` formats for older versions. We must design a graceful fallback to `x86_64` emulation for ARM64 Windows environments when native builds are unavailable.

### Ready for Proposal
Yes — We have investigated the exact URL structures, remote APIs, and decompression needs. We are ready to proceed to the Proposal and Specification phase.
