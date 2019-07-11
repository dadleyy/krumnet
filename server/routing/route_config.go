package routing

import "net/http"

// RouteConfig matching information for an http.HandlerFunc
type RouteConfig struct {
	Method  string
	Pattern string
	Handler http.HandlerFunc
}
