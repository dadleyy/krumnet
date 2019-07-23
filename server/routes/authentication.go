package routes

import "log"
import "fmt"
import "net/url"
import "net/http"
import "encoding/json"
import "github.com/krumpled/krumnet/server/env"
import "github.com/krumpled/krumnet/server/auth"
import "github.com/krumpled/krumnet/server/routing"

const (
	login       = "/auth/login"
	logout      = "/auth/logout"
	identify    = "/auth/identify"
	callback    = "/auth/google/callback"
	discoverURL = "https://openidconnect.googleapis.com/v1/userinfo"
	authURL     = "https://accounts.google.com/o/oauth2/v2/auth"
	tokenURL    = "https://www.googleapis.com/oauth2/v4/token"
)

type authenticationRouter struct {
	mux         *http.ServeMux
	credentials env.ServerConfig
	store       auth.SessionStore
}

func (a *authenticationRouter) login(response http.ResponseWriter, request *http.Request) {
	header := response.Header()
	destination, e := url.Parse(authURL)

	if e != nil {
		log.Printf("unable to parse auth url: %s", e)
		response.WriteHeader(500)
		return
	}

	query := make(url.Values)
	query.Set("response_type", "code")
	query.Set("client_id", a.credentials.Google.ClientID)
	query.Set("scope", "email profile")
	query.Set("redirect_uri", a.credentials.Google.RedirectURI)
	destination.RawQuery = query.Encode()

	header.Add("Location", destination.String())
	header.Add("Server", "krumpled-api")
	response.WriteHeader(302)
}

func (a *authenticationRouter) fetchUserInfo(token string) (auth.UserInfo, error) {
	discover, e := http.NewRequest("GET", discoverURL, nil)

	if e != nil {
		return auth.UserInfo{}, e
	}

	discover.Header.Add("Authorization", fmt.Sprintf("Bearer %s", token))

	client := http.Client{}
	discovery, e := client.Do(discover)

	if e != nil {
		return auth.UserInfo{}, e
	}

	defer discovery.Body.Close()

	info := struct {
		ID      string `json:"sub"`
		Name    string `json:"name"`
		Picture string `json:"picture"`
		Email   string `json:"email"`
		Locale  string `json:"locale"`
	}{}

	decoder := json.NewDecoder(discovery.Body)

	if e := decoder.Decode(&info); e != nil {
		return auth.UserInfo{}, e
	}

	log.Printf("successfully pulled '%s' info the system", info.Name)

	return auth.UserInfo{Email: info.Email, ID: info.ID, Name: info.Name}, nil
}

func (a *authenticationRouter) logout(response http.ResponseWriter, request *http.Request) {
	token := request.URL.Query().Get("token")

	if len(token) == 0 {
		response.WriteHeader(404)
		return
	}

	if e := a.store.Destroy(token); e != nil {
		log.Printf("unable to destroy session via '%s'", token)
		response.Header().Add("Location", a.credentials.Krumpled.ClientAddr)
		response.WriteHeader(302)
		return
	}

	log.Printf("logout: '%s'", token)
	response.Header().Add("Location", a.credentials.Krumpled.ClientAddr)
	response.WriteHeader(302)
}

func (a *authenticationRouter) callback(response http.ResponseWriter, request *http.Request) {
	code := request.URL.Query().Get("code")

	if len(code) == 0 {
		response.WriteHeader(404)
		return
	}

	log.Printf("received google auth code, exchanging for token")

	client := http.Client{}
	values := make(url.Values)
	values.Set("code", code)
	values.Set("client_id", a.credentials.Google.ClientID)
	values.Set("client_secret", a.credentials.Google.ClientSecret)
	values.Set("redirect_uri", a.credentials.Google.RedirectURI)
	values.Set("grant_type", "authorization_code")

	result, e := client.PostForm(tokenURL, values)

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

	info, e := a.fetchUserInfo(details.AccessToken)

	if e != nil {
		log.Printf("failed token -> info exchange: %s", e)
		response.WriteHeader(422)
		return
	}

	session, e := a.store.Create(info)

	if e != nil {
		log.Printf("failed session creation: %s", e)
		response.WriteHeader(422)
		return
	}

	log.Printf("created session '%s'", session.ID)

	out, e := url.Parse(a.credentials.Krumpled.ClientAddr)

	if e != nil {
		log.Printf("failed krumpled url generation during auth: %s", e)
		response.WriteHeader(422)
		return
	}

	query := make(url.Values)
	query.Set("token", session.Token)
	out.RawQuery = query.Encode()

	response.Header().Add("Location", out.String())
	log.Printf("completed authentication")
	response.WriteHeader(302)
}

func (a *authenticationRouter) identify(response http.ResponseWriter, request *http.Request) {
	defer request.Body.Close()

	decoder := json.NewDecoder(request.Body)
	payload := struct {
		Token string `json:"token"`
	}{}

	if e := decoder.Decode(&payload); e != nil {
		response.WriteHeader(422)
		return
	}

	log.Printf("identifying user based on token %s", payload.Token)

	user, e := a.store.Find(payload.Token)

	if e != nil {
		log.Printf("unable to find user: %s", e)
		response.WriteHeader(422)
		return
	}

	log.Printf("found user '%s'", user.ID)

	encoder := json.NewEncoder(response)
	response.Header().Add("Content-Type", "application/json")

	if e := encoder.Encode(struct {
		User auth.UserInfo `json:"user"`
	}{user}); e != nil {
		response.WriteHeader(500)
		return
	}
}

// NewAuthenticationRouter returns the http handler that deals with login/logout routes.
func NewAuthenticationRouter(store auth.SessionStore, creds env.ServerConfig) routing.Multiplex {
	router := &authenticationRouter{credentials: creds, store: store}

	return routing.Multiplex{
		{Method: routing.Get, Pattern: login, Handler: router.login},
		{Method: routing.Get, Pattern: logout, Handler: router.logout},
		{Method: routing.Get, Pattern: callback, Handler: router.callback},
		{Method: routing.Post, Pattern: identify, Handler: router.identify},
	}
}
