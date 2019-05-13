package auth

import "io"
import "fmt"
import "log"
import "time"
import "crypto/aes"
import "crypto/rand"
import "encoding/hex"
import "crypto/cipher"
import "github.com/google/uuid"
import "github.com/go-redis/redis"
import "github.com/krumpled/api/server/env"

const (
	sessionPrefix = "krumpled_session"
)

type redisStore struct {
	client *redis.Client
	secret string
}

func (r *redisStore) encrypt(info UserInfo) (string, error) {
	plaintext := fmt.Sprintf("%v:%v:%v", info.Email, info.ID, info.Name)
	block, e := aes.NewCipher([]byte(r.secret))

	if e != nil {
		return "", e
	}

	nonce := make([]byte, 12)
	if _, e := io.ReadFull(rand.Reader, nonce); e != nil {
		return "", e
	}

	aesgcm, e := cipher.NewGCM(block)

	if e != nil {
		return "", e
	}

	ciphertext := aesgcm.Seal(nil, nonce, []byte(plaintext), nil)
	return hex.EncodeToString(ciphertext), nil
}

func (r *redisStore) Create(info UserInfo) (SessionHandle, error) {
	id := fmt.Sprintf("%s", uuid.New())
	serialized, e := r.encrypt(info)

	if e != nil {
		return SessionHandle{}, e
	}

	log.Printf("created encrypted session:\n%s\n", serialized)

	expires := time.Until(time.Now().Add(time.Minute))
	result := r.client.Set(fmt.Sprintf("%s:session:%s", sessionPrefix, id), serialized, expires)

	if e := result.Err(); e != nil {
		log.Printf("failed storing session: %s", e)
		return SessionHandle{}, e
	}

	log.Printf("creating session '%s' for '%s'", id, info.ID)
	return SessionHandle{ID: id, secret: r.secret}, nil
}

// NewRedisStore returns an implementation of the auth.Store container that connects w/ redis.
func NewRedisStore(options env.ServerConfig) (SessionStore, error) {
	client := redis.NewClient(&options.Redis)

	result := client.Ping()

	if e := result.Err(); e != nil {
		return nil, e
	}

	log.Printf("successfully pinged redis server '%s'", result.String())

	return &redisStore{client: client, secret: options.Krumpled.SessionSecret}, nil
}
