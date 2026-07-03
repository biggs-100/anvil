"""Integration tests for anvil-sdk Python client.

Requires anvil to be built and on $PATH.
Run with: python -m pytest tests/
"""

import json
import os
import subprocess
import sys
import time

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from anvil_sdk import Anvil, AnvilError


def test_anvil_connect():
    """Verify the anvil subprocess can be spawned."""
    client = Anvil()
    try:
        status = client.status()
        assert isinstance(status, dict)
        assert "state" in status
    finally:
        client.close()


def test_env_roundtrip():
    """Verify env_set, env_get, env_unset."""
    client = Anvil()
    try:
        key = "ANVIL_SDK_PY_TEST"
        value = "test_value_py"

        client.env_set(key, value)
        got = client.env_get(key)
        assert got == value, f"Expected {value}, got {got}"

        client.env_unset(key)
        got = client.env_get(key)
        assert got is None, f"Expected None after unset, got {got}"
    finally:
        client.close()


def test_secret_roundtrip():
    """Verify secret_set, secret_get, secret_list, secret_remove."""
    client = Anvil()
    try:
        key = "SDK_PY_TEST_KEY"
        value = "sdk_py_val"

        client.secret_set(key, value)
        got = client.secret_get(key)
        assert got == value, f"Expected {value}, got {got}"

        keys = client.secret_list()
        assert key in keys, f"Expected {key} in secret list"

        client.secret_remove(key)
        got = client.secret_get(key)
        assert got is None, f"Expected None after remove, got {got}"
    finally:
        client.close()


def test_context_manager():
    """Verify context manager lifecycle."""
    with Anvil() as client:
        status = client.status()
        assert isinstance(status, dict)
        assert "state" in status

    # After exit, the process should be dead
    assert client._process.poll() is not None


def test_parse_error():
    """Verify the client handles JSON-RPC error responses."""
    client = Anvil()
    try:
        try:
            # Call an unknown method
            client._call("nonexistent.method")
        except AnvilError as e:
            assert e.code == -32601 or e.code is not None
            assert "Method not found" in str(e)
            return
        assert False, "Expected AnvilError"
    finally:
        client.close()


if __name__ == "__main__":
    test_anvil_connect()
    test_env_roundtrip()
    test_secret_roundtrip()
    test_context_manager()
    test_parse_error()
    print("All tests passed!")
