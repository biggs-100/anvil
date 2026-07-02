# Archive Extractors Specification

## Purpose

Define a unified extraction interface to safely decompress ZIP, TarGz, and TarXz archive formats.

## Requirements

| Requirement ID | Description | Strength |
|---|---|---|
| REQ-EXT-001 | The system MUST support decompression of ZIP, TarGz, and TarXz (using LZMA) archive formats. | MUST |
| REQ-EXT-002 | The extractor MUST prevent path traversal attacks by rejecting files with paths resolving outside the target directory. | MUST |

### Requirement: Format Support

#### Scenario: Extract TarXz Package
- GIVEN a TarXz package containing a Go toolchain
- WHEN the extraction process is executed
- THEN the system MUST decompress the package and place contents in the target cache directory.

### Requirement: Path Traversal Prevention

#### Scenario: Trap Traversal Archive
- GIVEN a malicious ZIP archive containing files with relative paths like "../../etc/passwd"
- WHEN decompression is requested
- THEN the extractor MUST abort execution, delete partial files, and return a path traversal error.
