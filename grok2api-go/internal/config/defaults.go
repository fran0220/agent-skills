package config

import (
	"os"
	"path/filepath"
)

const (
	defaultConfigFile = "config.defaults.toml"
	userConfigFile    = "config.toml"
	dataDirName       = "data"
)

type Paths struct {
	Defaults string
	Override string
}

type AppConfig struct {
	AppURL            string   `toml:"app_url"`
	AppKey            string   `toml:"app_key"`
	APIKey            string   `toml:"api_key"`
	FunctionEnabled   bool     `toml:"function_enabled"`
	FunctionKey       string   `toml:"function_key"`
	ImageFormat       string   `toml:"image_format"`
	VideoFormat       string   `toml:"video_format"`
	Temporary         bool     `toml:"temporary"`
	DisableMemory     bool     `toml:"disable_memory"`
	Stream            bool     `toml:"stream"`
	Thinking          bool     `toml:"thinking"`
	DynamicStatsig    bool     `toml:"dynamic_statsig"`
	CustomInstruction string   `toml:"custom_instruction"`
	FilterTags        []string `toml:"filter_tags"`
}

type ProxyConfig struct {
	BaseProxyURL       string `toml:"base_proxy_url"`
	AssetProxyURL      string `toml:"asset_proxy_url"`
	CFCookies          string `toml:"cf_cookies"`
	SkipProxySSLVerify bool   `toml:"skip_proxy_ssl_verify"`
	Enabled            bool   `toml:"enabled"`
	FlaresolverrURL    string `toml:"flaresolverr_url"`
	RefreshInterval    int    `toml:"refresh_interval"`
	Timeout            int    `toml:"timeout"`
	CFClearance        string `toml:"cf_clearance"`
	Browser            string `toml:"browser"`
	UserAgent          string `toml:"user_agent"`
}

type RetryConfig struct {
	MaxRetry                int     `toml:"max_retry"`
	RetryStatusCodes        []int   `toml:"retry_status_codes"`
	ResetSessionStatusCodes []int   `toml:"reset_session_status_codes"`
	RetryBackoffBase        float64 `toml:"retry_backoff_base"`
	RetryBackoffFactor      float64 `toml:"retry_backoff_factor"`
	RetryBackoffMax         float64 `toml:"retry_backoff_max"`
	RetryBudget             float64 `toml:"retry_budget"`
}

type TokenConfig struct {
	AutoRefresh                   bool `toml:"auto_refresh"`
	RefreshIntervalHours          int  `toml:"refresh_interval_hours"`
	SuperRefreshIntervalHours     int  `toml:"super_refresh_interval_hours"`
	FailThreshold                 int  `toml:"fail_threshold"`
	SaveDelayMS                   int  `toml:"save_delay_ms"`
	UsageFlushIntervalSec         int  `toml:"usage_flush_interval_sec"`
	ReloadIntervalSec             int  `toml:"reload_interval_sec"`
	OnDemandRefreshEnabled        bool `toml:"on_demand_refresh_enabled"`
	OnDemandRefreshMinIntervalSec int  `toml:"on_demand_refresh_min_interval_sec"`
	OnDemandRefreshMaxTokens      int  `toml:"on_demand_refresh_max_tokens"`
	ConsumedModeEnabled           bool `toml:"consumed_mode_enabled"`
}

type LogConfig struct {
	MaxFileSizeMB     int  `toml:"max_file_size_mb"`
	MaxFiles          int  `toml:"max_files"`
	LogHealthRequests bool `toml:"log_health_requests"`
	LogAllRequests    bool `toml:"log_all_requests"`
	RequestSlowMS     int  `toml:"request_slow_ms"`
}

type CacheConfig struct {
	EnableAutoClean bool `toml:"enable_auto_clean"`
	LimitMB         int  `toml:"limit_mb"`
}

type ChatConfig struct {
	Concurrent    int `toml:"concurrent"`
	Timeout       int `toml:"timeout"`
	StreamTimeout int `toml:"stream_timeout"`
}

type ImageConfig struct {
	Timeout                 int  `toml:"timeout"`
	StreamTimeout           int  `toml:"stream_timeout"`
	FinalTimeout            int  `toml:"final_timeout"`
	BlockedGraceSeconds     int  `toml:"blocked_grace_seconds"`
	NSFW                    bool `toml:"nsfw"`
	MediumMinBytes          int  `toml:"medium_min_bytes"`
	FinalMinBytes           int  `toml:"final_min_bytes"`
	BlockedParallelAttempts int  `toml:"blocked_parallel_attempts"`
	BlockedParallelEnabled  bool `toml:"blocked_parallel_enabled"`
}

type ImagineFastConfig struct {
	N              int    `toml:"n"`
	Size           string `toml:"size"`
	ResponseFormat string `toml:"response_format"`
}

type VideoConfig struct {
	EnablePublicAsset bool   `toml:"enable_public_asset"`
	Concurrent        int    `toml:"concurrent"`
	Timeout           int    `toml:"timeout"`
	StreamTimeout     int    `toml:"stream_timeout"`
	UpscaleTiming     string `toml:"upscale_timing"`
}

type VoiceConfig struct {
	Timeout int `toml:"timeout"`
}

type AssetConfig struct {
	UploadConcurrent   int `toml:"upload_concurrent"`
	UploadTimeout      int `toml:"upload_timeout"`
	DownloadConcurrent int `toml:"download_concurrent"`
	DownloadTimeout    int `toml:"download_timeout"`
	ListConcurrent     int `toml:"list_concurrent"`
	ListTimeout        int `toml:"list_timeout"`
	ListBatchSize      int `toml:"list_batch_size"`
	DeleteConcurrent   int `toml:"delete_concurrent"`
	DeleteTimeout      int `toml:"delete_timeout"`
	DeleteBatchSize    int `toml:"delete_batch_size"`
}

type NSFWConfig struct {
	Concurrent int `toml:"concurrent"`
	BatchSize  int `toml:"batch_size"`
	Timeout    int `toml:"timeout"`
}

type UsageConfig struct {
	Concurrent int `toml:"concurrent"`
	BatchSize  int `toml:"batch_size"`
	Timeout    int `toml:"timeout"`
}

func New(rootDir string) *Config {
	cfg := &Config{paths: resolvePaths(rootDir)}
	SetCurrent(cfg)
	return cfg
}

func resolvePaths(rootDir string) Paths {
	root := normalizeRoot(rootDir)
	candidates := uniqueStrings(root, currentWorkingDir(), executableDir(), "/")

	defaults := firstExistingFile(candidates, defaultConfigFile)
	if defaults == "" {
		defaults = filepath.Join(root, defaultConfigFile)
	}

	override := firstExistingDataFile(candidates)
	if override == "" {
		override = filepath.Join(root, dataDirName, userConfigFile)
	}

	return Paths{Defaults: defaults, Override: override}
}

func normalizeRoot(rootDir string) string {
	if rootDir != "" {
		return rootDir
	}
	if cwd := currentWorkingDir(); cwd != "" {
		return cwd
	}
	return "."
}

func currentWorkingDir() string {
	cwd, err := os.Getwd()
	if err != nil {
		return ""
	}
	return cwd
}

func executableDir() string {
	exe, err := os.Executable()
	if err != nil {
		return ""
	}
	return filepath.Dir(exe)
}

func firstExistingFile(roots []string, name string) string {
	for _, root := range roots {
		candidate := filepath.Join(root, name)
		if fileExists(candidate) {
			return candidate
		}
	}
	return ""
}

func firstExistingDataFile(roots []string) string {
	for _, root := range roots {
		candidate := filepath.Join(root, dataDirName, userConfigFile)
		if fileExists(candidate) {
			return candidate
		}
	}
	return ""
}

func uniqueStrings(values ...string) []string {
	seen := make(map[string]struct{}, len(values))
	result := make([]string, 0, len(values))
	for _, value := range values {
		if value == "" {
			continue
		}
		if _, ok := seen[value]; ok {
			continue
		}
		seen[value] = struct{}{}
		result = append(result, value)
	}
	return result
}

func fileExists(path string) bool {
	info, err := os.Stat(path)
	return err == nil && !info.IsDir()
}
