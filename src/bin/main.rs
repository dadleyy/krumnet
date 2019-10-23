extern crate async_std;

use async_std::task;
use krumnet::configuration::{Configuration, GoogleCredentials};
use krumnet::run;
use std::env::{args, var_os};

fn main() {
  let client_id = var_os("GOOGLE_CLIENT_ID")
    .unwrap_or_default()
    .into_string()
    .unwrap_or_default();
  let client_secret = var_os("GOOGLE_CLIENT_SECRET")
    .unwrap_or_default()
    .into_string()
    .unwrap_or_default();
  let redirect_uri = var_os("GOOGLE_CLIENT_REDIRECT_URI")
    .unwrap_or_default()
    .into_string()
    .unwrap_or_default();

  let config = Configuration {
    google: GoogleCredentials::new(client_id, client_secret, redirect_uri),
  };

  let addr = args()
    .skip(1)
    .nth(0)
    .unwrap_or(String::from("0.0.0.0:8080"));

  println!("[debug] starting server '{}'", addr);

  if let Err(e) = task::block_on(run(addr, config)) {
    println!("[error] exiting with error: {:?}", e);
  }
}
