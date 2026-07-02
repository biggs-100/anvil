# Configuration Validation Delta Specification

## MODIFIED Requirements

### Requirement: Doctor Integration

The validation engine MUST integrate with the diagnostics suite (`forge doctor`). It MUST produce a collection of `Finding` structs rather than legacy `DoctorIssue` reports for all validation errors including type mismatches, missing values, and pattern violations.

#### Scenario: Running Doctor with Invalid Configuration
- GIVEN a materialized config with a string instead of an integer for `MAX_CONNECTIONS`
- WHEN `forge doctor` is run
- THEN the system MUST return a `Finding` indicating a type mismatch error (FG009) for `MAX_CONNECTIONS`
