package auth

import "io"
import "fmt"
import "log"
import "time"
import "strings"
import "bytes"
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

func (r *redisStore) Find(token string) (UserInfo, error) {
	if len(token) == 0 {
		return UserInfo{}, fmt.Errorf("invalid token")
	}

	decrypted, e := r.decrypt(token)

	if e != nil {
		log.Printf("unable to decode token '%s': %s", token, e)
		return UserInfo{}, e
	}

	log.Printf("successfully decrypted token '%s'", decrypted)

	lookupResult := r.client.Get(fmt.Sprintf("%s:session:%s", sessionPrefix, decrypted))

	data, e := lookupResult.Result()

	if e != nil {
		log.Printf("unable to find session by '%s'", decrypted)
		return UserInfo{}, e
	}

	log.Printf("loaded session '%s'", data)
	userString, e := r.decrypt(data)

	if e != nil {
		log.Printf("unable to decrypt session by '%s': %s", decrypted, e)
		return UserInfo{}, e
	}

	result := UserInfo{}
	decoder := json.NewDecoder(strings.NewReader(userString))

	if e := decoder.Decode(&result); e != nil {
		log.Printf("unable to decrypt session by '%s': %s", decrypted, e)
		return UserInfo{}, e
	}

	log.Printf("decrypted session '%s'", userString)

	return result, nil
}

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

	expires := time.Until(time.Now().Add(time.Minute))
	result := r.client.Set(fmt.Sprintf("%s:session:%s", sessionPrefix, id), serialized, expires)

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
