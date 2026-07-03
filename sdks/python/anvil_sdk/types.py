"""Type definitions for anvil-sdk responses."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass
class StatusInfo:
    state: str


@dataclass
class SyncReport:
    status: str = ""


@dataclass
class RepairReport:
    status: str = ""


@dataclass
class CleanReport:
    status: str = ""


@dataclass
class RuntimeExplanation:
    runtime: str = ""
    state: str = ""
    diagnostics: list[str] = field(default_factory=list)


@dataclass
class OperationSummary:
    id: str = ""
    runtime: str = ""
    duration_ms: int = 0
    status: str = ""


@dataclass
class RunOutput:
    status: str = ""
    duration_ms: int = 0
    warnings: list[str] = field(default_factory=list)
    changes: list[dict[str, str]] = field(default_factory=list)
    diagnostics: list[str] = field(default_factory=list)


@dataclass
class HistoryEntry:
    id: str = ""
    runtime: str = ""
    duration_ms: int = 0
    status: str = ""


@dataclass
class ContextData:
    data: str = ""


@dataclass
class ResolvedEnvironment:
    vars: dict[str, str] = field(default_factory=dict)


@dataclass
class EnvVar:
    key: str = ""
    value: str = ""
