package server

import "log"
import "fmt"
import "net/http"
import "github.com/krumpled/krumnet/server/auth"
import "github.com/krumpled/krumnet/server/routes"
import "github.com/krumpled/krumnet/server/routing"
import "github.com/krumpled/krumnet/server/env"

type server struct {
	routing routing.Multiplex
	auth    auth.SessionStore
}

func (s *server) init(options env.ServerConfig) error {
	var e error
	s.auth, e = auth.NewRedisStore(options)

	if options.Startup.ClearAuthStore {
		log.Printf("clearing auth store, all sessions will be invalidated")
		s.auth.Purge()
	}

	s.routing = append(s.routing, routes.NewAuthenticationRouter(s.auth, options)...)

	if e != nil {
		return e
	}

	return nil
}

func (s *server) notFound(response http.ResponseWriter, request *http.Request) {
	response.WriteHeader(http.StatusNotFound)
	log.Printf("request not found")
	fmt.Fprintf(response, "not found")
}

func (s *server) ServeHTTP(response http.ResponseWriter, request *http.Request) {
	log.Printf("request %s %v", request.Method, request.URL)
	handler := s.routing.Match(request)

	if handler == nil {
		handler = s.notFound
	}

	handler(response, request)
}

// New constructs the krumpled http.Server
func New(opts env.ServerConfig) (*http.Server, error) {
	handler := &server{}

	if e := handler.init(opts); e != nil {
		return nil, e
	}

	return &http.Server{Handler: handler, Addr: opts.Krumpled.ServerAddr}, nil
}
