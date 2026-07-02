# Agent Inspector Specification

## Purpose

Providing context inspection and diagnostic commands tailored for AI agents to assess environment health.

## Requirements

### Requirement: Structured Context and Diagnostics

The system MUST provide `forge ai context` and `forge ai doctor` commands returning JSON outputs that check toolchains, system packages, and secrets without exposing sensitive secret values.

#### Scenario: Context Output Redaction
- GIVEN `forge.env` contains a secret `API_KEY=supersecret123`
- WHEN `forge ai context` is executed
- THEN the output MUST be valid JSON and the `API_KEY` value MUST be represented as a masked string (e.g. `true` or `[REDACTED]`)

#### Scenario: Environment Diagnostics and Remediation
- GIVEN a missing Python toolchain and missing system package `git`
- WHEN `forge ai doctor` is executed
- THEN the output MUST return a list of failing checks, their severity, and structured remediation instructions
