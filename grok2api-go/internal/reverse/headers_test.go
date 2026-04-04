package reverse

import "testing"

func TestBuildSSOCookieMergesClearance(t *testing.T) {
	SetConfigProvider(testConfigProvider{
		strings: map[string]string{
			"proxy.cf_cookies":   "foo=bar; cf_clearance=old; baz=qux",
			"proxy.cf_clearance": "new-token",
		},
		bools: map[string]bool{"proxy.enabled": false},
	})
	t.Cleanup(func() { SetConfigProvider(nil) })

	got := BuildSSOCookie("sso= a b c ")
	want := "sso=abc; sso-rw=abc; foo=bar; cf_clearance=new-token; baz=qux"
	if got != want {
		t.Fatalf("expected cookie %q, got %q", want, got)
	}
}

func TestBuildHeadersSetsOriginSiteAndHints(t *testing.T) {
	SetConfigProvider(testConfigProvider{
		strings: map[string]string{
			"proxy.user_agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36",
			"proxy.browser":    "chrome136",
		},
		bools: map[string]bool{"app.dynamic_statsig": false},
	})
	t.Cleanup(func() { SetConfigProvider(nil) })

	headers := BuildHeaders("token", "application/json", "https://grok.com", "https://grok.com/")
	if headers["Sec-Fetch-Site"] != "same-origin" {
		t.Fatalf("expected same-origin fetch site, got %q", headers["Sec-Fetch-Site"])
	}
	if headers["Cookie"] != "sso=token; sso-rw=token" {
		t.Fatalf("unexpected cookie header: %q", headers["Cookie"])
	}
	if headers["Sec-Ch-Ua"] == "" {
		t.Fatal("expected chromium client hints to be populated")
	}
	if headers["x-statsig-id"] != staticStatsigID {
		t.Fatalf("expected static statsig id, got %q", headers["x-statsig-id"])
	}
}
