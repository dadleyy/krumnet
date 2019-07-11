package auth

// SessionHandle represents a handle to an active session persisted in storage
type SessionHandle struct {
	ID    string
	Token string
}
