# Event Bus Delta Specification

## Purpose
This delta spec modifies the existing Event Bus specification to register hooks/subscribers forwarding memory event broadcasts to the filesystem writer.

## Added Requirements

### Requirement: Event Bus Telemetry Forwarding
The Event Bus MUST support a dedicated subscriber that intercepts all internal progress/state broadcast events and forwards them to the journal queue.

#### Scenario: Forwarding Event Broadcast
- GIVEN an active Event Bus
- WHEN any event is broadcast on the memory channel
- THEN the system MUST intercept it and enqueue it for background journal persistence.
