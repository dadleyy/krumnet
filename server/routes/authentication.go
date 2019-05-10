package routes

import "fmt"
import "net/http"

const (
	login  = "/auth/login"
	logout = "/auth/logout"
)

type authenticationRouter struct {
	mux *http.ServeMux
}

func (auth *authenticationRouter) login(response http.ResponseWriter, request *http.Request) {
	response.WriteHeader(200)
	fmt.Fprintf(response, "logged in\n")
}

func (auth *authenticationRouter) logout(response http.ResponseWriter, request *http.Request) {
	response.WriteHeader(200)
	fmt.Fprintf(response, "logged out\n")
}

// NewAuthenticationRouter returns the http handler that deals with login/logout routes.
func NewAuthenticationRouter() (http.Handler, []string) {
	router := &authenticationRouter{}
	mux := http.NewServeMux()
	mux.HandleFunc(login, router.login)
	mux.HandleFunc(logout, router.logout)
	return mux, []string{login, logout}
}
