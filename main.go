package main

import "os"
import "fmt"
import "flag"
import "github.com/joho/godotenv"
import "github.com/krumpled/api/server"

func main() {
	if e := godotenv.Load(".env"); e != nil {
		fmt.Printf("unable to load environment: %s", e)
		os.Exit(1)
	}

	opts := server.Options{}
	flag.StringVar(&opts.Addr, "address", ":1991", "http address")

	handler, e := server.New(opts)

	if e != nil {
		fmt.Printf("unable to start http server: %s", e)
		os.Exit(1)
	}

	fmt.Printf("start server, binding to %s\n", opts.Addr)
	if e := handler.ListenAndServe(); e != nil {
		fmt.Printf("unable to start http server: %s", e)
		os.Exit(1)
	}

	handler.Close()
}
