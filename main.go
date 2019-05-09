package main

import "os"
import "fmt"
import "flag"
import "net/http"

type options struct {
	port string
}

type handler struct {
}

func (h handler) ServeHTTP(response http.ResponseWriter, request *http.Request) {
	response.WriteHeader(200)
	fmt.Fprintf(response, "hello")
}

func main() {
	opts := options{}
	flag.StringVar(&opts.port, "port", "1991", "http port")
	fmt.Println("hello")

	if e := http.ListenAndServe(fmt.Sprintf("0.0.0.0:%s", opts.port), handler{}); e != nil {
		fmt.Printf("unable to start http server: %s", e)
		os.Exit(1)
	}
}
