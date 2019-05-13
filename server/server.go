package server

import "log"
import "fmt"
import "strings"
import "net/http"
import "github.com/krumpled/api/server/auth"
import "github.com/krumpled/api/server/routes"
import "github.com/krumpled/api/server/env"

type server struct {
	verbs map[string]*http.ServeMux
}

func (s *server) init(options env.ServerConfig) error {
	query := http.NewServeMux()

	// Initialize services
	authStore, e := auth.NewRedisStore(options)

	if e != nil {
		return e
	}

	s.verbs = map[string]*http.ServeMux{"get": query}

	for method, handler := range routes.NewAuthenticationRouter(authStore, options) {
		methodMultiplexer, ok := s.verbs[strings.ToLower(method)]

		if !ok {
			return fmt.Errorf("unable to find method handler for '%s'", strings.ToLower(method))
		}

		for key, handle := range handler {
			log.Printf("handling '%s %s'", method, key)
			methodMultiplexer.Handle(key, handle)
		}
	}

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
func New(opts env.ServerConfig) (*http.Server, error) {
	handler := &server{}

	if e := handler.init(opts); e != nil {
		return nil, e
	}

	return &http.Server{Handler: handler, Addr: opts.Krumpled.ServerAddr}, nil
}
