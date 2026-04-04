package reverse

import (
	"encoding/base64"
	"strings"
	"testing"
)

type testConfigProvider struct {
	strings map[string]string
	bools   map[string]bool
	ints    map[string]int
	floats  map[string]float64
	slices  map[string][]int
}

func (p testConfigProvider) String(key string) (string, bool) {
	v, ok := p.strings[key]
	return v, ok
}

func (p testConfigProvider) Bool(key string) (bool, bool) {
	v, ok := p.bools[key]
	return v, ok
}

func (p testConfigProvider) Int(key string) (int, bool) {
	v, ok := p.ints[key]
	return v, ok
}

func (p testConfigProvider) Float64(key string) (float64, bool) {
	v, ok := p.floats[key]
	return v, ok
}

func (p testConfigProvider) IntSlice(key string) ([]int, bool) {
	v, ok := p.slices[key]
	return v, ok
}

func TestStatsigGeneratorStatic(t *testing.T) {
	SetConfigProvider(testConfigProvider{bools: map[string]bool{"app.dynamic_statsig": false}})
	t.Cleanup(func() { SetConfigProvider(nil) })

	if got := StatsigGenerator.GenID(); got != staticStatsigID {
		t.Fatalf("expected static statsig id %q, got %q", staticStatsigID, got)
	}
}

func TestStatsigGeneratorDynamic(t *testing.T) {
	SetConfigProvider(testConfigProvider{bools: map[string]bool{"app.dynamic_statsig": true}})
	t.Cleanup(func() { SetConfigProvider(nil) })

	encoded := StatsigGenerator.GenID()
	decoded, err := base64.StdEncoding.DecodeString(encoded)
	if err != nil {
		t.Fatalf("expected valid base64, got error: %v", err)
	}
	message := string(decoded)
	if !strings.HasPrefix(message, "e:TypeError: Cannot read properties of ") {
		t.Fatalf("unexpected dynamic statsig payload: %q", message)
	}
	if !strings.Contains(message, "reading '") {
		t.Fatalf("expected property read marker in %q", message)
	}
}
