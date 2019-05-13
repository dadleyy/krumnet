package auth

import "io"
import "crypto/aes"
import "crypto/rand"
import "encoding/hex"
import "crypto/cipher"

// SessionHandle represents a handle to an active session persisted in storage
type SessionHandle struct {
	ID     string
	secret string
}

// Token returns a secure token that is sent to clients
func (h *SessionHandle) Token() string {
	block, e := aes.NewCipher([]byte(h.secret))

	if e != nil {
		return ""
	}

	nonce := make([]byte, 12)
	if _, e := io.ReadFull(rand.Reader, nonce); e != nil {
		return ""
	}

	aesgcm, e := cipher.NewGCM(block)

	if e != nil {
		return ""
	}

	ciphertext := aesgcm.Seal(nil, nonce, []byte(h.ID), nil)
	return hex.EncodeToString(ciphertext)
}
