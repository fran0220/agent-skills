package token

import "testing"

func TestConsumeEntersCoolingAndReturnsRemainingQuota(t *testing.T) {
	token := &TokenInfo{
		Token:     "abc",
		Status:    StatusActive,
		Quota:     3,
		CreatedAt: 1,
	}

	remaining := token.Consume(EffortHigh)

	if remaining != 0 {
		t.Fatalf("expected remaining quota 0, got %d", remaining)
	}
	if token.Status != StatusCooling {
		t.Fatalf("expected token to enter cooling, got %q", token.Status)
	}
	if token.Consumed != 0 {
		t.Fatalf("expected consumed to reset on cooling, got %d", token.Consumed)
	}
	if token.UseCount != 3 {
		t.Fatalf("expected use_count to track actual consumed quota, got %d", token.UseCount)
	}
}

func TestUpdateQuotaWithConsumedRecoversAndResetsConsumed(t *testing.T) {
	token := &TokenInfo{
		Token:      "abc",
		Status:     StatusCooling,
		Quota:      0,
		Consumed:   9,
		FailCount:  2,
		CreatedAt:  1,
		LastSyncAt: 1,
	}

	token.UpdateQuotaWithConsumed(12)

	if token.Status != StatusActive {
		t.Fatalf("expected token to recover to active, got %q", token.Status)
	}
	if token.Quota != 12 {
		t.Fatalf("expected quota 12, got %d", token.Quota)
	}
	if token.Consumed != 0 {
		t.Fatalf("expected consumed reset to 0, got %d", token.Consumed)
	}
	if token.FailCount != 0 {
		t.Fatalf("expected fail count reset to 0, got %d", token.FailCount)
	}
}
