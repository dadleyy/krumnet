package auth

import "log"
import "github.com/go-redis/redis"

type redisStore struct {
	client *redis.Client
}

// NewRedisStore returns an implementation of the auth.Store container that connects w/ redis.
func NewRedisStore(options redis.Options) (Store, error) {
	client := redis.NewClient(&options)

	result := client.Ping()

	if e := result.Err(); e != nil {
		return nil, e
	}

	log.Printf("successfully pinged redis server '%s'", result.String())

	return &redisStore{client: client}, nil
}
