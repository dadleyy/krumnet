package env

import "github.com/go-redis/redis"

// ServerConfig contains the configuration necessary for running the krumpled http.Server
type ServerConfig struct {
	Google   GoogleConfig
	Redis    redis.Options
	Krumpled KrumpledConfig
	Startup  struct {
		ClearAuthStore bool
	}
}
