package storage

import (
	"os"
	"path/filepath"
	"runtime"
)

func DataDir() string {
	dataDir := os.Getenv("DATA_DIR")
	if dataDir != "" {
		return dataDir
	}
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		return filepath.Clean(filepath.Join(".", "data"))
	}
	return filepath.Clean(filepath.Join(filepath.Dir(file), "..", "..", "data"))
}
