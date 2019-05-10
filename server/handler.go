package server

import "log"
import "fmt"
import "strings"
import "net/http"
import "github.com/krumpled/api/server/routes"

type server struct {
	verbs map[string]*http.ServeMux
}

func (s *server) init(options Options) error {
	query := http.NewServeMux()

	auth, patterns := routes.NewAuthenticationRouter(struct {
		Google struct {
			ClientId     string
			ClientSecret string
			RedirectUri  string
		}
		Krumpled struct {
			RedirectUri string
		}
	}{options.Google, options.Krumpled})

	for _, p := range patterns {
		query.Handle(p, auth)
	}

	s.verbs = map[string]*http.ServeMux{"get": query}

	return nil
}

func (s *server) ServeHTTP(response http.ResponseWriter, request *http.Request) {
	log.Printf("request %s %v", request.Method, request.URL)
	multiplexer, ok := s.verbs[strings.ToLower(request.Method)]

	if !ok || multiplexer == nil {
		response.WriteHeader(422)
		fmt.Fprintf(response, "bad-verb\n")
		return
	}

	handler, pattern := multiplexer.Handler(request)

	if handler == nil {
		response.WriteHeader(404)
		log.Printf("404: %s", pattern)
		return
	}

	handler.ServeHTTP(response, request)
}

// New constructs the krumpled http.Server
func New(opts Options) (*http.Server, error) {
	handler := &server{}

	if e := handler.init(opts); e != nil {
		return nil, e
	}

	return &http.Server{Handler: handler, Addr: opts.Addr}, nil
}
