# Package Installer Specification

## Purpose

Define the post-sync package installer that automatically installs project dependencies using anvil-managed runtimes after `anvil up` completes runtime synchronization.

## Requirements

### Requirement: Post-Sync Package Installation

The system MUST execute package installation automatically after `anvil up` finishes runtime synchronization, provided the `[packages]` section is present in `anvil.toml`.

#### Scenario: Auto-Install After Sync

- GIVEN `anvil.toml` contains `[packages]` with `pip = "requirements.txt"` and the anvil-managed python runtime is installed
- WHEN `anvil up` finishes syncing runtimes
- THEN the system MUST run `pip install -r requirements.txt` using the anvil-managed python binary

#### Scenario: No Packages Section

- GIVEN `anvil.toml` does NOT contain a `[packages]` section
- WHEN `anvil up` finishes syncing runtimes
- THEN the system MUST NOT attempt package installation

### Requirement: Anvil-Managed Python Binary

The package installer MUST use the anvil-managed python binary that was downloaded during runtime synchronization. It MUST NOT use system python or a user-configured python path.

#### Scenario: Anvil Python Used for Pip

- GIVEN the anvil-managed python runtime is installed at `~/.anvil/runtimes/python/3.11/bin/python3`
- WHEN the installer executes `pip install`
- THEN it MUST invoke `/home/user/.anvil/runtimes/python/3.11/bin/python3 -m pip install -r requirements.txt`

#### Scenario: No Python Runtime Configured

- GIVEN `[packages]` has `pip = "requirements.txt"` but no python runtime is installed
- WHEN `anvil up` tries to run package installation
- THEN the system MUST report a clear error indicating no python runtime is available

### Requirement: Install Output Streaming

The system MUST stream pip install stdout and stderr to the user's terminal in real time during installation.

#### Scenario: Progress Displayed

- GIVEN pip install is running for a requirements.txt with 3 packages
- WHEN installation proceeds
- THEN the user MUST see each package's download and install progress lines in the terminal

### Requirement: Missing Requirements File Error

The system MUST produce a clear, actionable error message if the path specified in `[packages].pip` does not exist, and MUST exit with a non-zero status code.

#### Scenario: Missing Requirements File

- GIVEN `anvil.toml` has `[packages].pip = "requirements.txt"` but the file does not exist
- WHEN `anvil up` initiates package installation
- THEN the system MUST display `"Requirements file not found: requirements.txt"`
- AND the system MUST exit with a non-zero status code

### Requirement: Configuration Format Support

The system SHOULD support a `[packages]` section in `anvil.toml` with a `pip` field specifying a string path to a pip-compatible requirements file.

#### Scenario: Pip Field Parsing

- GIVEN `anvil.toml` contains `[packages]\npip = "deps/requirements.txt"`
- WHEN the configuration is loaded
- THEN the system MUST parse the `pip` value as the string `"deps/requirements.txt"`
