package anvilsdk

import (
	"context"
	"testing"
	"time"
)

// TestNewAnvil verifies the anvil subprocess can be spawned.
// Requires anvil to be on $PATH.
func TestNewAnvil(t *testing.T) {
	client, err := NewAnvil()
	if err != nil {
		t.Skipf("anvil not on PATH or not built: %v", err)
	}
	defer client.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	status, err := client.Status(ctx)
	if err != nil {
		t.Fatalf("Status() failed: %v", err)
	}
	if status.State == "" {
		t.Error("Status() returned empty state")
	}
	t.Logf("Status: %+v", status)
}

// TestSync verifies the sync method works.
func TestSync(t *testing.T) {
	client, err := NewAnvil()
	if err != nil {
		t.Skipf("anvil not on PATH: %v", err)
	}
	defer client.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	report, err := client.Sync(ctx)
	if err != nil {
		t.Fatalf("Sync() failed: %v", err)
	}
	t.Logf("Sync report: %+v", report)
}

// TestEnvRoundtrip verifies env_set, env_get, env_unset.
func TestEnvRoundtrip(t *testing.T) {
	client, err := NewAnvil()
	if err != nil {
		t.Skipf("anvil not on PATH: %v", err)
	}
	defer client.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	key := "ANVIL_SDK_GO_TEST"
	value := "test_value_go"

	// Set
	if err := client.EnvSet(ctx, key, value); err != nil {
		t.Fatalf("EnvSet() failed: %v", err)
	}

	// Get
	got, err := client.EnvGet(ctx, key)
	if err != nil {
		t.Fatalf("EnvGet() failed: %v", err)
	}
	if got == nil || *got != value {
		t.Fatalf("EnvGet() = %v, want %v", got, value)
	}

	// Unset
	if err := client.EnvUnset(ctx, key); err != nil {
		t.Fatalf("EnvUnset() failed: %v", err)
	}
}

// TestContextCancellation verifies cancellation works.
func TestContextCancellation(t *testing.T) {
	client, err := NewAnvil()
	if err != nil {
		t.Skipf("anvil not on PATH: %v", err)
	}
	defer client.Close()

	ctx, cancel := context.WithCancel(context.Background())
	cancel() // Cancel immediately

	_, err = client.Status(ctx)
	if err == nil {
		t.Error("Expected error for cancelled context, got nil")
	}
}
