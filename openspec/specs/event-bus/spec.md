# Event Bus Specification

## Purpose

Define the Event Bus progress dispatch system, utilizing a Tokio-based broadcast channel to broadcast environment operational progress and state change events.

## Requirements

### Requirement: Structured Telemetry Broadcast
The system MUST broadcast runtime installation and operational status events through an asynchronous multi-producer multi-consumer (`broadcast`) channel.

Every event MUST adhere to the following schema:
- **timestamp**: RFC 3339 datetime string.
- **operation_id**: UUID string uniquely identifying the execution runner.
- **runtime**: String identifier of the toolchain (e.g., `python`, `node`, `global`).
- **phase**: Step name (e.g., `resolve`, `download`, `extract`, `verify`, `commit`).
- **status**: Progress indicator enum (`started`, `progress(u8)`, `completed`, `failed`).
- **message**: Optional descriptive text.

### Requirement: Thread-Safe Subscriptions
The Event Bus MUST support multiple parallel subscribers subscribing to the stream without blocking active execution operations.

#### Scenario: Progress Event Broadcast during Download
- GIVEN an active toolchain download operation
- WHEN 50% of the payload is downloaded
- THEN the system MUST broadcast an event with `runtime` set to the specific toolchain, `phase` set to `download`, `status` set to `progress(50)`, and include a valid `timestamp`.

#### Scenario: Subscriber Receive Failures Do Not Block Producers
- GIVEN a slow event receiver subscribing to the channel
- WHEN the event producer publishes events at high frequency
- THEN the producer MUST NOT be blocked by the slow receiver, and the receiver SHOULD receive a lag warning or skipped message indicator per channel capacity limits.

### Requirement: Event Bus Telemetry Forwarding
The Event Bus MUST support a dedicated subscriber that intercepts all internal progress/state broadcast events and forwards them to the journal queue.

#### Scenario: Forwarding Event Broadcast
- GIVEN an active Event Bus
- WHEN any event is broadcast on the memory channel
- THEN the system MUST intercept it and enqueue it for background journal persistence.
