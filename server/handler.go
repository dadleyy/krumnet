package server

import "fmt"
import "net/http"

type server struct {
}

func (s *server) init() error {
	return nil
}

func (s *server) ServeHTTP(response http.ResponseWriter, request *http.Request) {
	response.WriteHeader(200)
	fmt.Fprintf(response, "who dat")
}

func New(opts Options) (*http.Server, error) {
	handler := &server{}

	if e := handler.init(); e != nil {
		return nil, e
	}

	return &http.Server{Handler: handler, Addr: opts.Addr}, nil
}
