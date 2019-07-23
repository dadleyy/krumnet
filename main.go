package main

import "os"
import "fmt"
import "log"
import "flag"
import "github.com/joho/godotenv"
import "github.com/krumpled/krumnet/server"
import "github.com/krumpled/krumnet/server/env"

func main() {
	if e := godotenv.Load(".env"); e != nil {
		fmt.Printf("unable to load environment: %s", e)
		os.Exit(1)
	}

	opts := env.ServerConfig{}
	opts.Google.ClientID = os.Getenv("GOOGLE_CLIENT_ID")
	opts.Google.ClientSecret = os.Getenv("GOOGLE_CLIENT_SECRET")
	opts.Google.RedirectURI = os.Getenv("GOOGLE_CLIENT_REDIRECT_URI")
	opts.Krumpled.ClientAddr = os.Getenv("KRUMPLED_CLIENT_REDIRECT_URI")
	opts.Krumpled.SessionSecret = os.Getenv("KRUMPLED_SESSION_SECRET")
	opts.Redis.Addr = os.Getenv("REDIS_URI")
	opts.Redis.Password = os.Getenv("REDIS_PASSWORD")

	flag.StringVar(&opts.Krumpled.ServerAddr, "address", "0.0.0.0:8102", "http address")
	flag.StringVar(&opts.Krumpled.ClientAddr, "krumpled-redirect", opts.Krumpled.ClientAddr, "client app url")
	flag.StringVar(&opts.Krumpled.SessionSecret, "krumpled-secret", opts.Krumpled.SessionSecret, "secret for session")
	flag.StringVar(&opts.Google.ClientID, "google-id", opts.Google.ClientID, "google client credentials")
	flag.StringVar(&opts.Google.ClientSecret, "google-secret", opts.Google.ClientSecret, "google client credentials")
	flag.StringVar(&opts.Google.RedirectURI, "google-redirect", opts.Google.RedirectURI, "google client credentials")
	flag.StringVar(&opts.Redis.Addr, "redis-addr", opts.Redis.Addr, "redis uri")
	flag.StringVar(&opts.Redis.Password, "redis-password", opts.Redis.Password, "redis password")
	flag.BoolVar(&opts.Startup.ClearAuthStore, "clear-auth", false, "clear auth store")

	log.SetFlags(log.Ldate | log.LstdFlags | log.Lshortfile)
	flag.Parse()

	log.Printf("initializing server, warming connections")
	handler, e := server.New(opts)

	if e != nil {
		fmt.Printf("unable to start http server: %s", e)
		os.Exit(1)
	}

	log.Printf("start server, binding to %s\n", opts.Krumpled.ServerAddr)
	if e := handler.ListenAndServe(); e != nil {
		fmt.Printf("unable to start http server: %s", e)
		os.Exit(1)
	}

	handler.Close()
}
