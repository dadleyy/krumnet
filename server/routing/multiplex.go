package routing

import "net/http"

// Multiplex is used to find a matching handler func based on a http.Request
type Multiplex []RouteConfig

// Match returns the handler func that matches the request, or nil
func (list *Multiplex) Match(request *http.Request) http.HandlerFunc {
	path := request.URL.Path

	for _, config := range *list {
		matches := config.Pattern == path && request.Method == config.Method

		if matches {
			return config.Handler
		}
	}

	return nil
}
