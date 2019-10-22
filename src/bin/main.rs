extern crate async_std;

use async_std::task;
use krumnet::google::GoogleCredentials;
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

  let google = GoogleCredentials::new(client_id, client_secret);

  let addr = args()
    .skip(1)
    .nth(0)
    .unwrap_or(String::from("0.0.0.0:8080"));

  println!("[debug] starting server '{}': {:?}", addr, google);

  if let Err(e) = task::block_on(run(addr, google)) {
    println!("[error] exiting with error: {:?}", e);
  }
}
