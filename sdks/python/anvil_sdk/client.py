"""Anvil Python SDK — subprocess-based JSON-RPC client."""

from __future__ import annotations

import json
import subprocess
import sys
from typing import Any, Optional


class AnvilError(Exception):
    """Error raised by anvil-sdk operations.

    Attributes:
        code: Optional JSON-RPC error code.
        message: Human-readable error description.
    """

    def __init__(self, message: str, code: int | None = None):
        super().__init__(message)
        self.code = code
        self.message = message

    def __str__(self) -> str:
        if self.code is not None:
            return f"[{self.code}] {self.message}"
        return self.message


class Anvil:
    """Client that controls a anvil jsonrpc subprocess.

    All methods communicate via JSON-RPC 2.0 over stdin/stdout.

    Can be used as a context manager::

        with Anvil() as client:
            status = client.status()
    """

    def __init__(self, anvil_path: str = "anvil"):
        """Spawn the anvil jsonrpc subprocess.

        Args:
            anvil_path: Path to the anvil binary (default: "anvil").
        """
        self._process = subprocess.Popen(
            [anvil_path, "jsonrpc"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=None,
            text=True,
            bufsize=1,  # line-buffered
        )
        self._next_id = 0

    def close(self) -> None:
        """Terminate the anvil subprocess."""
        if self._process and self._process.poll() is None:
            self._process.terminate()
            try:
                self._process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self._process.kill()
                self._process.wait()

    def __enter__(self) -> "Anvil":
        return self

    def __exit__(self, *args: Any) -> None:
        self.close()

    # ── Core RPC ──────────────────────────────────────────────────────

    def _call(self, method: str, params: Optional[dict[str, Any]] = None) -> Any:
        """Send a JSON-RPC call and return the result."""
        if self._process.poll() is not None:
            raise AnvilError("anvil subprocess is not running")

        self._next_id += 1
        request = {
            "jsonrpc": "2.0",
            "id": self._next_id,
            "method": method,
            "params": params or {},
        }

        line = json.dumps(request, ensure_ascii=False)
        assert self._process.stdin is not None
        self._process.stdin.write(line + "\n")
        self._process.stdin.flush()

        assert self._process.stdout is not None
        response_line = self._process.stdout.readline()
        if not response_line:
            raise AnvilError("anvil subprocess closed connection unexpectedly")

        response = json.loads(response_line)
        if "error" in response and response["error"] is not None:
            err = response["error"]
            raise AnvilError(err.get("message", "Unknown error"), code=err.get("code"))

        return response.get("result")

    # ── Async variants ────────────────────────────────────────────────

    async def async_status(self) -> dict[str, Any]:
        return self._call("engine.status")

    async def async_sync(self) -> dict[str, Any]:
        return self._call("engine.sync")

    async def async_repair(self) -> dict[str, Any]:
        return self._call("engine.repair")

    async def async_clean(self) -> dict[str, Any]:
        return self._call("engine.clean")

    async def async_explain(self, runtime: str) -> Any:
        return self._call("engine.explain", {"runtime": runtime})

    async def async_history(self, limit: int = 10) -> Any:
        return self._call("engine.history", {"limit": limit})

    async def async_context(self, fmt: str = "json") -> Any:
        return self._call("context.get", {"format": fmt})

    # ── Engine methods ─────────────────────────────────────────────────

    def status(self) -> dict[str, Any]:
        """Get the current lifecycle state."""
        return self._call("engine.status")

    def sync(self) -> dict[str, Any]:
        """Sync runtimes from lockfile."""
        return self._call("engine.sync")

    def repair(self) -> dict[str, Any]:
        """Repair corrupted or missing runtimes."""
        return self._call("engine.repair")

    def clean(self) -> dict[str, Any]:
        """Clean all local cache and state."""
        return self._call("engine.clean")

    def explain(self, runtime: str) -> Any:
        """Explain a runtime's configuration and cache status."""
        return self._call("engine.explain", {"runtime": runtime})

    def history(self, limit: int = 10) -> Any:
        """Show past operations history."""
        return self._call("engine.history", {"limit": limit})

    def run(self, cmd: str, *args: str) -> Any:
        """Execute a command inside the activated environment."""
        return self._call("exec.run", {"cmd": cmd, "args": list(args)})

    def context(self, fmt: str = "json") -> Any:
        """Query contextual environment information."""
        return self._call("context.get", {"format": fmt})

    # ── Environment methods ────────────────────────────────────────────

    def env_list(self) -> dict[str, str]:
        """List all environment variables."""
        return self._call("env.list")

    def env_get(self, key: str) -> Optional[str]:
        """Get a single environment variable by key."""
        return self._call("env.get", {"key": key})

    def env_set(self, key: str, value: str) -> Any:
        """Set an environment variable."""
        return self._call("env.set", {"key": key, "value": value})

    def env_unset(self, key: str) -> Any:
        """Unset/remove an environment variable."""
        return self._call("env.unset", {"key": key})

    def env_resolve(self, key: str) -> Any:
        """Resolve a specific environment variable."""
        return self._call("env.resolve", {"key": key})

    # ── Secrets methods ───────────────────────────────────────────────

    def secret_set(self, key: str, value: str) -> Any:
        """Set a secret."""
        return self._call("secret.set", {"key": key, "value": value})

    def secret_get(self, key: str) -> Optional[str]:
        """Get a secret by key."""
        return self._call("secret.get", {"key": key})

    def secret_list(self) -> list[str]:
        """List all secret keys."""
        return self._call("secret.list")

    def secret_remove(self, key: str) -> Any:
        """Remove a secret."""
        return self._call("secret.remove", {"key": key})
