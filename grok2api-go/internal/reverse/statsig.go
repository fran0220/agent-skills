package reverse

import (
	"encoding/base64"
	"fmt"
	"log/slog"
	"math/rand/v2"
	"strings"
)

const staticStatsigID = "ZTpUeXBlRXJyb3I6IENhbm5vdCByZWFkIHByb3BlcnRpZXMgb2YgdW5kZWZpbmVkIChyZWFkaW5nICdjaGlsZE5vZGVzJyk="

type statsigGenerator struct{}

// StatsigGenerator mirrors the original Python call pattern: StatsigGenerator.GenID().
var StatsigGenerator statsigGenerator

func (statsigGenerator) rand(length int, alphanumeric bool) string {
	chars := "abcdefghijklmnopqrstuvwxyz"
	if alphanumeric {
		chars += "0123456789"
	}
	var builder strings.Builder
	builder.Grow(length)
	for range length {
		builder.WriteByte(chars[rand.IntN(len(chars))])
	}
	return builder.String()
}

// GenID generates either a dynamic or static Statsig identifier.
func (g statsigGenerator) GenID() string {
	if getConfigBool("app.dynamic_statsig", true) {
		slog.Debug("generating dynamic Statsig ID")

		var message string
		if rand.IntN(2) == 0 {
			randPart := g.rand(5, true)
			message = fmt.Sprintf("e:TypeError: Cannot read properties of null (reading 'children['%s']')", randPart)
		} else {
			randPart := g.rand(10, false)
			message = fmt.Sprintf("e:TypeError: Cannot read properties of undefined (reading '%s')", randPart)
		}

		return base64.StdEncoding.EncodeToString([]byte(message))
	}

	slog.Debug("generating static Statsig ID")
	return staticStatsigID
}
