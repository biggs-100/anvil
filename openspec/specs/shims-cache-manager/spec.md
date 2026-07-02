# Shims Cache Manager Specification

## Purpose

Managing serialization, format, validation, and project integration of the toolchain mapping cache.

## Requirements

### Requirement: Key-Value Cache Layout

The system MUST store mappings in a flat key-value file named `.forge/shims.cache` containing metadata comments and runtime executable paths parsed line-by-line.

#### Scenario: Parse Key-Value Layout
- GIVEN a cache file `.forge/shims.cache` with header `# forge-shims-cache-v1` and mappings like `node = /path/to/node`
- WHEN the manager reads the cache file
- THEN the system MUST successfully deserialize the configuration

### Requirement: Validation Signature

The cache manager MUST validate that the cache file starts with the correct header signature.

#### Scenario: Version Header Invalidation
- GIVEN a cache file with an outdated or missing header version (e.g. not starting with `# forge-shims-cache-v1`)
- WHEN the manager performs validation
- THEN the system MUST reject the cache file as invalid

### Requirement: Gitignore Integration

The project initialization process MUST ensure cache files are excluded from version control.

#### Scenario: Add Cache to Gitignore
- GIVEN a newly initialized project using `forge init`
- WHEN the system creates/updates `.gitignore`
- THEN the system MUST append `.forge/shims.cache` if not already present
