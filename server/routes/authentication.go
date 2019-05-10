package routes

import "log"
import "fmt"
import "net/url"
import "net/http"
import "encoding/json"

const (
	login    = "/auth/login"
	logout   = "/auth/logout"
	callback = "/auth/google/callback"
	authUrl  = "https://accounts.google.com/o/oauth2/v2/auth"
	tokenUrl = "https://www.googleapis.com/oauth2/v4/token"
)

type credentials struct {
	Google struct {
		ClientId     string
		ClientSecret string
		RedirectUri  string
	}
	Krumpled struct {
		RedirectUri string
	}
}

type authenticationRouter struct {
	mux         *http.ServeMux
	credentials credentials
}

func (auth *authenticationRouter) login(response http.ResponseWriter, request *http.Request) {
	header := response.Header()
	destination, e := url.Parse(authUrl)

	if e != nil {
		log.Printf("unable to parse auth url: %s", e)
		response.WriteHeader(500)
		return
	}

	query := make(url.Values)
	query.Set("response_type", "code")
	query.Set("client_id", auth.credentials.Google.ClientId)
	query.Set("scope", "email")
	query.Set("redirect_uri", auth.credentials.Google.RedirectUri)
	destination.RawQuery = query.Encode()

	header.Add("Location", destination.String())
	header.Add("Server", "krumpled-api")
	response.WriteHeader(302)
}

func (auth *authenticationRouter) logout(response http.ResponseWriter, request *http.Request) {
	response.WriteHeader(200)
	fmt.Fprintf(response, "logged out\n")
}

func (auth *authenticationRouter) callback(response http.ResponseWriter, request *http.Request) {
	code := request.URL.Query().Get("code")

	if len(code) == 0 {
		response.WriteHeader(404)
		return
	}

	log.Printf("received google auth code, exchanging for token")

	client := http.Client{}
	values := make(url.Values)
	values.Set("code", code)
	values.Set("client_id", auth.credentials.Google.ClientId)
	values.Set("client_secret", auth.credentials.Google.ClientSecret)
	values.Set("redirect_uri", auth.credentials.Google.RedirectUri)
	values.Set("grant_type", "authorization_code")

	result, e := client.PostForm(tokenUrl, values)

	if e != nil {
		log.Printf("failed code -> token exchange: %s", e)
		response.WriteHeader(422)
		return
	}
	defer result.Body.Close()

	decoder := json.NewDecoder(result.Body)
	details := struct {
		AccessToken  string `json:"access_token"`
		RefreshToken string `json:"refresh_token"`
		ExpiresIn    int    `json:"expires_in"`
		TokenType    string `json:"token_type"`
	}{}

	if e := decoder.Decode(&details); e != nil {
		log.Printf("failed code -> token exchange: %s", e)
		response.WriteHeader(422)
		return
	}

	response.Header().Add("Location", auth.credentials.Krumpled.RedirectUri)
	log.Printf("successfully got token '%v'", details.AccessToken)
	response.WriteHeader(302)
}

// NewAuthenticationRouter returns the http handler that deals with login/logout routes.
func NewAuthenticationRouter(creds credentials) (http.Handler, []string) {
	router := &authenticationRouter{credentials: creds}
	mux := http.NewServeMux()
	mux.HandleFunc(login, router.login)
	mux.HandleFunc(logout, router.logout)
	mux.HandleFunc(callback, router.callback)
	return mux, []string{login, logout, callback}
}
