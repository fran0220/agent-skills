package reverse

import (
	"fmt"
	"log/slog"
	"net/url"
	"regexp"
	"strings"
	"unicode"

	"github.com/google/uuid"
)

const baggageHeader = "sentry-environment=production,sentry-release=d6add6fb0460641fd482d767a335ef72b9b6abb8,sentry-public_key=b311e0f2690c81f25e2c4cf6d4f7ce1c"

var headerCharReplacements = map[rune]string{
	'\u2010': "-",
	'\u2011': "-",
	'\u2012': "-",
	'\u2013': "-",
	'\u2014': "-",
	'\u2212': "-",
	'\u2018': "'",
	'\u2019': "'",
	'\u201c': `"`,
	'\u201d': `"`,
	'\u00a0': " ",
	'\u2007': " ",
	'\u202f': " ",
	'\u200b': "",
	'\u200c': "",
	'\u200d': "",
	'\ufeff': "",
}

var (
	spaceRe       = regexp.MustCompile(`\s+`)
	clearanceRe   = regexp.MustCompile(`(?:^|;\s*)cf_clearance=`)
	replaceClearR = regexp.MustCompile(`(^|;\s*)cf_clearance=[^;]*`)
)

func sanitizeHeaderValue(value, fieldName string, removeAllSpaces bool) string {
	raw := value
	var builder strings.Builder
	for _, r := range raw {
		if replacement, ok := headerCharReplacements[r]; ok {
			builder.WriteString(replacement)
			continue
		}
		if r <= unicode.MaxLatin1 {
			builder.WriteRune(r)
		}
	}

	normalized := builder.String()
	if removeAllSpaces {
		normalized = spaceRe.ReplaceAllString(normalized, "")
	} else {
		normalized = strings.TrimSpace(normalized)
	}

	if normalized != raw {
		slog.Warn("sanitized header value", "field", fieldName, "from_len", len(raw), "to_len", len(normalized))
	}
	return normalized
}

// BuildSSOCookie returns the Grok SSO cookie plus optional CF cookies.
func BuildSSOCookie(ssoToken string) string {
	token := strings.TrimPrefix(ssoToken, "sso=")
	token = sanitizeHeaderValue(token, "sso_token", true)
	cookie := fmt.Sprintf("sso=%s; sso-rw=%s", token, token)

	cfCookies := sanitizeHeaderValue(getConfigString("proxy.cf_cookies", ""), "proxy.cf_cookies", false)
	cfClearance := sanitizeHeaderValue(getConfigString("proxy.cf_clearance", ""), "proxy.cf_clearance", true)
	cfRefreshEnabled := getConfigBool("proxy.enabled", false)

	if cfRefreshEnabled {
		if cfCookies == "" && cfClearance != "" {
			cfCookies = "cf_clearance=" + cfClearance
		}
	} else if cfClearance != "" {
		if cfCookies != "" {
			if clearanceRe.MatchString(cfCookies) {
				cfCookies = replaceClearR.ReplaceAllString(cfCookies, "${1}cf_clearance="+cfClearance)
			} else {
				cfCookies = strings.TrimRight(cfCookies, "; ")
				cfCookies += "; cf_clearance=" + cfClearance
			}
		} else {
			cfCookies = "cf_clearance=" + cfClearance
		}
	}

	if cfCookies != "" {
		if cookie != "" && !strings.HasSuffix(cookie, ";") {
			cookie += "; "
		}
		cookie += cfCookies
	}

	return cookie
}

func extractMajorVersion(browser, userAgent string) string {
	for _, source := range []string{browser, userAgent} {
		patterns := []string{`(\d{2,3})`, `Edg/(\d+)`, `Chrome/(\d+)`, `Chromium/(\d+)`}
		for idx, pattern := range patterns {
			if source == "" {
				continue
			}
			if idx == 0 && source == userAgent {
				continue
			}
			if match := regexp.MustCompile(pattern).FindStringSubmatch(source); len(match) > 1 {
				return match[1]
			}
		}
	}
	return ""
}

func detectPlatform(userAgent string) string {
	ua := strings.ToLower(userAgent)
	switch {
	case strings.Contains(ua, "windows"):
		return "Windows"
	case strings.Contains(ua, "mac os x"), strings.Contains(ua, "macintosh"):
		return "macOS"
	case strings.Contains(ua, "android"):
		return "Android"
	case strings.Contains(ua, "iphone"), strings.Contains(ua, "ipad"):
		return "iOS"
	case strings.Contains(ua, "linux"):
		return "Linux"
	default:
		return ""
	}
}

func detectArch(userAgent string) string {
	ua := strings.ToLower(userAgent)
	switch {
	case strings.Contains(ua, "aarch64"), strings.Contains(ua, "arm"):
		return "arm"
	case strings.Contains(ua, "x86_64"), strings.Contains(ua, "x64"), strings.Contains(ua, "win64"), strings.Contains(ua, "intel"):
		return "x86"
	default:
		return ""
	}
}

func buildClientHints(browser, userAgent string) map[string]string {
	browser = strings.ToLower(strings.TrimSpace(browser))
	ua := strings.ToLower(userAgent)

	isEdge := strings.Contains(browser, "edge") || strings.Contains(ua, "edg")
	isBrave := strings.Contains(browser, "brave")
	isChromium := strings.Contains(browser, "chrome") || strings.Contains(browser, "chromium") || strings.Contains(browser, "edge") || strings.Contains(browser, "brave") || strings.Contains(ua, "chrome") || strings.Contains(ua, "chromium") || strings.Contains(ua, "edg")
	isFirefox := strings.Contains(browser, "firefox") || strings.Contains(ua, "firefox")
	isSafari := (strings.Contains(ua, "safari") && !strings.Contains(ua, "chrome") && !strings.Contains(ua, "chromium") && !strings.Contains(ua, "edg")) || strings.Contains(browser, "safari")

	if !isChromium || isFirefox || isSafari {
		return map[string]string{}
	}

	version := extractMajorVersion(browser, userAgent)
	if version == "" {
		return map[string]string{}
	}

	brand := "Google Chrome"
	switch {
	case isEdge:
		brand = "Microsoft Edge"
	case strings.Contains(browser, "chromium"):
		brand = "Chromium"
	case isBrave:
		brand = "Brave"
	}

	platform := detectPlatform(userAgent)
	arch := detectArch(userAgent)
	mobile := "?0"
	if strings.Contains(ua, "mobile") || platform == "Android" || platform == "iOS" {
		mobile = "?1"
	}

	hints := map[string]string{
		"Sec-Ch-Ua":        fmt.Sprintf(`"%s";v="%s", "Chromium";v="%s", "Not(A:Brand";v="24"`, brand, version, version),
		"Sec-Ch-Ua-Mobile": mobile,
		"Sec-Ch-Ua-Model":  "",
	}
	if platform != "" {
		hints["Sec-Ch-Ua-Platform"] = fmt.Sprintf(`"%s"`, platform)
	}
	if arch != "" {
		hints["Sec-Ch-Ua-Arch"] = arch
		hints["Sec-Ch-Ua-Bitness"] = "64"
	}
	return hints
}

func defaultUserAgent() string {
	return getConfigString("proxy.user_agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36")
}

func defaultBrowser() string {
	return getConfigString("proxy.browser", "chrome136")
}

// BuildWSHeaders returns websocket request headers.
func BuildWSHeaders(token, origin string) map[string]string {
	return BuildWSHeadersWithExtra(token, origin, nil)
}

// BuildWSHeadersWithExtra returns websocket request headers with extra overrides.
func BuildWSHeadersWithExtra(token, origin string, extra map[string]string) map[string]string {
	userAgent := sanitizeHeaderValue(defaultUserAgent(), "proxy.user_agent", false)
	safeOrigin := sanitizeHeaderValue(origin, "origin", false)
	if safeOrigin == "" {
		safeOrigin = "https://grok.com"
	}

	headers := map[string]string{
		"Origin":          safeOrigin,
		"User-Agent":      userAgent,
		"Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
		"Cache-Control":   "no-cache",
		"Pragma":          "no-cache",
	}
	for key, value := range buildClientHints(defaultBrowser(), userAgent) {
		headers[key] = value
	}
	if token != "" {
		headers["Cookie"] = BuildSSOCookie(token)
	}
	for key, value := range extra {
		headers[key] = value
	}
	return headers
}

// BuildHeaders returns the standard reverse request headers.
func BuildHeaders(cookieToken, contentType, origin, referer string) map[string]string {
	userAgent := sanitizeHeaderValue(defaultUserAgent(), "proxy.user_agent", false)
	safeOrigin := sanitizeHeaderValue(origin, "origin", false)
	if safeOrigin == "" {
		safeOrigin = "https://grok.com"
	}
	safeReferer := sanitizeHeaderValue(referer, "referer", false)
	if safeReferer == "" {
		safeReferer = "https://grok.com/"
	}

	headers := map[string]string{
		// Do NOT set Accept-Encoding manually: Go's http2 transport does not
		// auto-decompress, so the stream processor would see compressed bytes.
		// Omitting the header lets the standard transport negotiate gzip and
		// transparently decompress for us.
		"Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
		"Baggage":          baggageHeader,
		"Origin":           safeOrigin,
		"Priority":         "u=1, i",
		"Referer":          safeReferer,
		"Sec-Fetch-Mode":   "cors",
		"User-Agent":       userAgent,
		"Cookie":           BuildSSOCookie(cookieToken),
		"x-statsig-id":     StatsigGenerator.GenID(),
		"x-xai-request-id": uuid.NewString(),
	}

	for key, value := range buildClientHints(defaultBrowser(), userAgent) {
		headers[key] = value
	}

	switch contentType {
	case "application/json", "":
		headers["Content-Type"] = "application/json"
		headers["Accept"] = "*/*"
		headers["Sec-Fetch-Dest"] = "empty"
	case "image/jpeg", "image/png", "video/mp4", "video/webm":
		headers["Content-Type"] = contentType
		headers["Accept"] = "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"
		headers["Sec-Fetch-Dest"] = "document"
	default:
		headers["Content-Type"] = "application/json"
		headers["Accept"] = "*/*"
		headers["Sec-Fetch-Dest"] = "empty"
	}

	originURL, _ := url.Parse(headers["Origin"])
	refererURL, _ := url.Parse(headers["Referer"])
	if originURL.Hostname() != "" && originURL.Hostname() == refererURL.Hostname() {
		headers["Sec-Fetch-Site"] = "same-origin"
	} else {
		headers["Sec-Fetch-Site"] = "same-site"
	}

	safeHeaders := make(map[string]string, len(headers))
	for key, value := range headers {
		safeHeaders[key] = value
	}
	safeHeaders["Cookie"] = "<redacted>"
	slog.Debug("built headers", "headers", safeHeaders)

	return headers
}
