# Observability Journal Specification

## Purpose
Define the asynchronous journal writing behavior, parsing events from the EventBus and appending serialized JSON Lines to `.forge/journal.jsonl`.

## Requirements

### Requirement: Asynchronous Journal Writer
The system MUST spawn a background task during EventBus initialization that asynchronously writes events to `.forge/journal.jsonl` in JSON Lines (NDJSON) format.

### Requirement: Directory and File Setup
The system MUST automatically create the `.forge` directory and the `journal.jsonl` file if they do not exist before attempting to write.

### Requirement: Thread-Safe Serialization
The background writer MUST write events sequentially to prevent interleaved or corrupt JSON lines.

#### Scenario: Appending Event to Journal
- GIVEN a running EventBus
- WHEN a new event is broadcast
- THEN the system MUST serialize the event to JSON and append it to `.forge/journal.jsonl` with a trailing newline.

#### Scenario: Auto-creation of Journal Directory
- GIVEN a missing `.forge` directory
- WHEN the first event is broadcast
- THEN the system MUST create the `.forge` directory before writing.
