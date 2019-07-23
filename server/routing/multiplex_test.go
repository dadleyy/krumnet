package routing

import "bytes"
import "testing"
import "net/http"
import "net/http/httptest"
import "github.com/franela/goblin"

func TestMatch(t *testing.T) {
	g := goblin.Goblin(t)

	var plex Multiplex

	g.Describe("Match", func() {
		g.BeforeEach(func() {
			plex = make(Multiplex, 0)
		})

		g.It("should return nil when nothing matches", func() {
			r := httptest.NewRequest("GET", "/hello", bytes.NewBufferString(""))
			out := plex.Match(r)
			g.Assert(out == nil).Eql(true)
		})

		g.It("should return a function when it does", func() {
			handler := func(response http.ResponseWriter, request *http.Request) {}
			plex = append(plex, RouteConfig{Pattern: "/hello", Method: "GET", Handler: handler})
			r := httptest.NewRequest("GET", "/hello", bytes.NewBufferString(""))
			t.Logf("match %v", plex)
			out := plex.Match(r)
			g.Assert(out == nil).Eql(false)
		})
	})
}
