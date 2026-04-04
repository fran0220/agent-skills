package token

import "testing"

func TestSelectPrefersHighestQuotaInDefaultMode(t *testing.T) {
	t.Setenv("GROK2API_TOKEN_CONSUMED_MODE_ENABLED", "false")

	pool := NewTokenPool(BasicPoolName)
	pool.Add(&TokenInfo{Token: "a", Status: StatusActive, Quota: 10})
	pool.Add(&TokenInfo{Token: "b", Status: StatusActive, Quota: 25})
	pool.Add(&TokenInfo{Token: "c", Status: StatusCooling, Quota: 99})

	selected := pool.Select(nil, nil)
	if selected == nil {
		t.Fatal("expected a token to be selected")
	}
	if selected.Token != "b" {
		t.Fatalf("expected token b, got %q", selected.Token)
	}
}

func TestSelectPrefersLowestConsumedInConsumedMode(t *testing.T) {
	t.Setenv("GROK2API_TOKEN_CONSUMED_MODE_ENABLED", "true")

	pool := NewTokenPool(BasicPoolName)
	pool.Add(&TokenInfo{Token: "a", Status: StatusActive, Quota: 0, Consumed: 7, Tags: []string{"nsfw"}})
	pool.Add(&TokenInfo{Token: "b", Status: StatusActive, Quota: 0, Consumed: 2, Tags: []string{"nsfw", "video"}})
	pool.Add(&TokenInfo{Token: "c", Status: StatusActive, Quota: 0, Consumed: 5})

	selected := pool.Select(nil, map[string]bool{"nsfw": true})
	if selected == nil {
		t.Fatal("expected a token to be selected")
	}
	if selected.Token != "b" {
		t.Fatalf("expected token b, got %q", selected.Token)
	}
}
