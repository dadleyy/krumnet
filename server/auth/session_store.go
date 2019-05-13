package auth

// SessionStore is an interface for the thing that holds the current user in memory.
type SessionStore interface {
	Create(info UserInfo) (SessionHandle, error)
}
