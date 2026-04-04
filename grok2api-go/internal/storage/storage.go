package storage

type TokenData map[string]any

type Storage interface {
	LoadTokens() (map[string][]TokenData, error)
	SaveTokens(data map[string][]TokenData) error
	LoadConfig() (map[string]interface{}, error)
	SaveConfig(data map[string]interface{}) error
}
