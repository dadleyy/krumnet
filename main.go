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
	opts.Google.ClientId = os.Getenv("GOOGLE_CLIENT_ID")
	opts.Google.ClientSecret = os.Getenv("GOOGLE_CLIENT_SECRET")
	opts.Google.RedirectUri = os.Getenv("GOOGLE_CLIENT_REDIRECT_URI")
	opts.Krumpled.RedirectUri = os.Getenv("KRUMPLED_CLIENT_REDIRECT_URI")

	flag.StringVar(&opts.Addr, "address", "0.0.0.0:8102", "http address")
	flag.StringVar(&opts.Google.ClientId, "google-id", opts.Google.ClientId, "google client credentials")
	flag.StringVar(&opts.Google.ClientSecret, "google-secret", opts.Google.ClientSecret, "google client credentials")
	flag.StringVar(&opts.Google.RedirectUri, "google-redirect", opts.Google.RedirectUri, "google client credentials")
	flag.StringVar(&opts.Krumpled.RedirectUri, "krumpled-redirect", opts.Krumpled.RedirectUri, "client app url")

	flag.Parse()

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
