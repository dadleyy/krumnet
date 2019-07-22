package auth

import "io"
import "fmt"
import "log"
import "time"
import "bytes"
import "strings"
import "crypto/aes"
import "crypto/rand"
import "crypto/md5"
import "encoding/hex"
import "encoding/json"
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

// encrypt uses the secret registered to encrypt plaintext.
func (r *redisStore) encrypt(data []byte) (string, error) {
	block, e := aes.NewCipher([]byte(r.secret))

	if e != nil {
		return "", e
	}

	gcm, e := cipher.NewGCM(block)

	if e != nil {
		return "", e
	}

	nonce := make([]byte, gcm.NonceSize())

	if _, e = io.ReadFull(rand.Reader, nonce); e != nil {
		return "", e
	}

	ciphertext := gcm.Seal(nonce, nonce, data, nil)
	return hex.EncodeToString(ciphertext), nil
}

// decrypt uses the secret registered to return a plaintext string representation of an encrypted string.
func (r *redisStore) decrypt(input string) (string, error) {
	decoded, e := hex.DecodeString(input)

	if e != nil {
		return "", e
	}

	block, e := aes.NewCipher([]byte(r.secret))

	if e != nil {
		return "", e
	}

	gcm, e := cipher.NewGCM(block)

	if e != nil {
		return "", e
	}

	nonceSize := gcm.NonceSize()
	nonce, ciphertext := decoded[:nonceSize], decoded[nonceSize:]
	plaintext, e := gcm.Open(nil, nonce, ciphertext, nil)

	if e != nil {
		return "", e
	}

	return fmt.Sprintf("%s", plaintext), nil
}

func (r *redisStore) keyforID(id string) string {
	return fmt.Sprintf("%s:session:%s", sessionPrefix, id)
}

// keyForToken is responsible for ensuring the validty and decryption of a token info a key.
func (r *redisStore) keyForToken(token string) (string, error) {
	if len(token) == 0 {
		return "", fmt.Errorf("invalid token")
	}

	decrypted, e := r.decrypt(token)

	if e != nil {
		return "", e
	}

	return r.keyforId(decrypted), nil
}

// Find returns the user info that has been encrypted into the key associated with the provided token.
func (r *redisStore) Find(token string) (UserInfo, error) {
	key, e := r.keyForToken(token)

	if e != nil {
		log.Printf("unable to decode token '%s': %s", token, e)
		return UserInfo{}, e
	}

	log.Printf("successfully decrypted token '%s'", key)

	lookupResult := r.client.Get(key)

	data, e := lookupResult.Result()

	if e != nil {
		log.Printf("unable to find session by '%s'", key)
		return UserInfo{}, e
	}

	log.Printf("loaded session '%s'", data)
	userString, e := r.decrypt(data)

	if e != nil {
		log.Printf("unable to decrypt session by '%s': %s", key, e)
		return UserInfo{}, e
	}

	result := UserInfo{}
	decoder := json.NewDecoder(strings.NewReader(userString))

	if e := decoder.Decode(&result); e != nil {
		log.Printf("unable to decrypt session by '%s': %s", key, e)
		return UserInfo{}, e
	}

	log.Printf("decrypted session '%s'", userString)

	return result, nil
}

// Destroy exectures a DEL command for the key associated with the provided token.
func (r *redisStore) Destroy(token string) error {
	key, e := r.keyForToken(token)

	if e != nil {
		return e
	}

	result := r.client.Del(key)

	return result.Err()
}

// Create will insert the provided user info into a new uniquely identifiable key in redis.
func (r *redisStore) Create(info UserInfo) (SessionHandle, error) {
	id := fmt.Sprintf("%s", uuid.New())

	buffer := bytes.NewBufferString("")
	encoder := json.NewEncoder(buffer)

	if e := encoder.Encode(info); e != nil {
		return SessionHandle{}, e
	}

	serialized, e := r.encrypt(buffer.Bytes())

	if e != nil {
		return SessionHandle{}, e
	}

	log.Printf("created encrypted session:\n%s\n", serialized)

	key := r.keyforId(id)
	expires := time.Until(time.Now().AddDate(0, 0, 30))
	result := r.client.Set(key, serialized, expires)

	if e := result.Err(); e != nil {
		log.Printf("failed storing session: %s", e)
		return SessionHandle{}, e
	}

	token, e := r.encrypt([]byte(id))

	if e != nil {
		return SessionHandle{}, e
	}

	log.Printf("creating session '%s' for '%s'", id, info.ID)
	return SessionHandle{ID: id, Token: token}, nil
}

// Purge destroys all active sessions.
func (r *redisStore) Purge() error {
	log.Printf("puring all sessions")
	keyCommand := redis.NewStringSliceCmd("KEYS", fmt.Sprintf("%s*", sessionPrefix))

	if e := r.client.Process(keyCommand); e != nil {
		log.Printf("failed preparing %s: %s", keyCommand, e)
		return e
	}

	keys, e := keyCommand.Result()

	if e != nil {
		log.Printf("failed execution %s: %s", keyCommand, e)
		return e
	}

	if len(keys) == 0 {
		log.Printf("no keys to delete")
		return nil
	}

	joined := strings.Join(keys, " ")
	delCommand := redis.NewStringCmd("DEL", joined)

	if e := r.client.Process(delCommand); e != nil {
		log.Printf("failed processing %s: %s", delCommand, e)
		return e
	}

	log.Printf("purged all keys: %v", keys)
	return nil
}

// NewRedisStore returns an implementation of the auth.Store container that connects w/ redis.
func NewRedisStore(options env.ServerConfig) (SessionStore, error) {
	client := redis.NewClient(&options.Redis)

	result := client.Ping()

	if e := result.Err(); e != nil {
		return nil, e
	}

	hasher := md5.New()

	if _, e := io.Copy(hasher, strings.NewReader(options.Krumpled.SessionSecret)); e != nil {
		return nil, e
	}

	secret := hex.EncodeToString(hasher.Sum(nil))

	log.Printf("successfully pinged redis server '%s'", result.String())

	return &redisStore{client: client, secret: secret}, nil
}
