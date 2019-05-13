package auth

import "fmt"
import "log"
import "github.com/google/uuid"
import "github.com/go-redis/redis"

type redisStore struct {
	client *redis.Client
}

func (r *redisStore) Create(info UserInfo) (SessionHandle, error) {
	id := fmt.Sprintf("%s", uuid.New())

	log.Printf("creating session for '%s'", info.ID)
	return SessionHandle{ID: id}, fmt.Errorf("poop")
}

// NewRedisStore returns an implementation of the auth.Store container that connects w/ redis.
func NewRedisStore(options redis.Options) (SessionStore, error) {
	client := redis.NewClient(&options)

	result := client.Ping()

	if e := result.Err(); e != nil {
		return nil, e
	}

	log.Printf("successfully pinged redis server '%s'", result.String())

	return &redisStore{client: client}, nil
}
