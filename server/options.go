package server

import "github.com/go-redis/redis"

// Options contains the configuration necessary for running the krumpled http.Server
type Options struct {
	Addr   string
	Google struct {
		ClientID     string
		ClientSecret string
		RedirectURI  string
	}
	Redis    redis.Options
	Krumpled struct {
		RedirectURI string
	}
}
