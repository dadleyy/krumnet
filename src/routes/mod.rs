use log::info;
use serde::Serialize;
use std::io::Error;

use crate::http::Response as Res;

pub mod auth;
pub mod lobbies;

pub fn server_error<T: Serialize>(original: Error) -> Res<T> {
  info!("server error - {}", original);
  Res::server_error()
}

pub fn not_found<T: Serialize>() -> Res<T> {
  Res::not_found(None)
}

pub fn redirect<T: Serialize>(location: String) -> Res<T> {
  Res::Redirect(location)
}
