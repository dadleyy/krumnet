package main

import "os"
import "fmt"
import "log"
import "flag"
import "github.com/joho/godotenv"
import "github.com/krumpled/api/server"

func main() {
	if e := godotenv.Load(".env"); e != nil {
		fmt.Printf("unable to load environment: %s", e)
		os.Exit(1)
	}

	opts := server.Options{}
	opts.Google.ClientID = os.Getenv("GOOGLE_CLIENT_ID")
	opts.Google.ClientSecret = os.Getenv("GOOGLE_CLIENT_SECRET")
	opts.Google.RedirectURI = os.Getenv("GOOGLE_CLIENT_REDIRECT_URI")
	opts.Krumpled.RedirectURI = os.Getenv("KRUMPLED_CLIENT_REDIRECT_URI")
	opts.Redis.Addr = os.Getenv("REDIS_URI")
	opts.Redis.Password = os.Getenv("REDIS_PASSWORD")

	flag.StringVar(&opts.Addr, "address", "0.0.0.0:8102", "http address")
	flag.StringVar(&opts.Google.ClientID, "google-id", opts.Google.ClientID, "google client credentials")
	flag.StringVar(&opts.Google.ClientSecret, "google-secret", opts.Google.ClientSecret, "google client credentials")
	flag.StringVar(&opts.Google.RedirectURI, "google-redirect", opts.Google.RedirectURI, "google client credentials")
	flag.StringVar(&opts.Krumpled.RedirectURI, "krumpled-redirect", opts.Krumpled.RedirectURI, "client app url")
	flag.StringVar(&opts.Redis.Addr, "redis-addr", opts.Redis.Addr, "redis uri")
	flag.StringVar(&opts.Redis.Password, "redis-password", opts.Redis.Password, "redis password")

	flag.Parse()

	log.Printf("initializing server, warming connections")
	handler, e := server.New(opts)

	if e != nil {
		fmt.Printf("unable to start http server: %s", e)
		os.Exit(1)
	}

	log.Printf("start server, binding to %s\n", opts.Addr)
	if e := handler.ListenAndServe(); e != nil {
		fmt.Printf("unable to start http server: %s", e)
		os.Exit(1)
	}

	handler.Close()
}
