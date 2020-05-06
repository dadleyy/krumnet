use http::response::{Builder, Response};
use std::io::{Error, ErrorKind};

pub mod auth;

pub fn not_found() -> Result<Response<Option<u8>>, Error> {
  Builder::new()
    .status(404)
    .body(None)
    .map_err(|e| Error::new(ErrorKind::Other, e))
}

pub fn redirect<S>(location: S) -> Result<Response<Option<u8>>, Error>
where
  S: std::fmt::Display,
{
  Builder::new()
    .status(302)
    .header(http::header::LOCATION, format!("{}", location))
    .body(None)
    .map_err(|e| Error::new(ErrorKind::Other, e))
}
