package config

import (
	"os"
	"path/filepath"
	"testing"
)

func TestLoadCoercesStringOverrides(t *testing.T) {
	t.Setenv("GROK2API_PROXY_REFRESH_INTERVAL", "")
	t.Setenv("GROK2API_IMAGINE_FAST_N", "")
	t.Setenv("GROK2API_RETRY_RETRY_BACKOFF_FACTOR", "")

	root := t.TempDir()
	dataDir := filepath.Join(root, "data")
	if err := os.MkdirAll(dataDir, 0o755); err != nil {
		t.Fatalf("mkdir data dir: %v", err)
	}

	defaults := []byte(`[proxy]
refresh_interval = 60

[imagine_fast]
n = 2

[retry]
retry_backoff_factor = 2.0
`)
	override := []byte(`[proxy]
refresh_interval = "3600"

[imagine_fast]
n = "1"

[retry]
retry_backoff_factor = "3.5"
`)

	if err := os.WriteFile(filepath.Join(root, "config.defaults.toml"), defaults, 0o644); err != nil {
		t.Fatalf("write defaults: %v", err)
	}
	if err := os.WriteFile(filepath.Join(dataDir, "config.toml"), override, 0o644); err != nil {
		t.Fatalf("write override: %v", err)
	}

	cfg := New(root)
	if err := cfg.Load(); err != nil {
		t.Fatalf("load config: %v", err)
	}

	if got := cfg.GetInt("proxy.refresh_interval"); got != 3600 {
		t.Fatalf("proxy.refresh_interval = %d, want 3600", got)
	}
	if got := cfg.GetInt("imagine_fast.n"); got != 1 {
		t.Fatalf("imagine_fast.n = %d, want 1", got)
	}
	if got := cfg.GetFloat("retry.retry_backoff_factor"); got != 3.5 {
		t.Fatalf("retry.retry_backoff_factor = %v, want 3.5", got)
	}
}
