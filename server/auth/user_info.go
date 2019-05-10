package auth

// UserInfo is a reference to a user stored.
type UserInfo struct {
	ID    string `json:"id"`
	Email string `json:"email"`
	Name  string `json:"name"`
}
