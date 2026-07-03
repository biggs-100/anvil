# Configuration Validation Specification

## Purpose

Define validation rules for environment and configuration variables, ensuring that materialized configurations match declarative manifest schemas.

## Requirements

### Requirement: Declarative Schema Validation

The validation engine MUST validate materialized variables against the `[config.definitions]` schema. It MUST support type checks (`string`, `integer`, `boolean`), required fields, and regex patterns.

#### Scenario: Missing Required Variable
- GIVEN a schema definition for `DATABASE_URL` marked as `required = true`
- WHEN the configuration is materialized without `DATABASE_URL` defined
- THEN the system MUST return a validation error indicating the key is missing

#### Scenario: Pattern Regex Constraint Failure
- GIVEN a schema definition for `PORT` with `pattern = "^[0-9]+$"`
- WHEN the materialized value is `abc`
- THEN the system MUST reject the value and raise a pattern validation error

---

### Requirement: Doctor Integration

The validation engine MUST integrate with the diagnostics suite (`anvil doctor`). It MUST produce a collection of `Finding` structs rather than legacy `DoctorIssue` reports for all validation errors including type mismatches, missing values, and pattern violations.

#### Scenario: Running Doctor with Invalid Configuration
- GIVEN a materialized config with a string instead of an integer for `MAX_CONNECTIONS`
- WHEN `anvil doctor` is run
- THEN the system MUST return a `Finding` indicating a type mismatch error (FG009) for `MAX_CONNECTIONS`
