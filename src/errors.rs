use std::io::{Error, ErrorKind};

pub fn humanize_error<E: std::error::Error>(e: E) -> Error {
  Error::new(ErrorKind::Other, format!("{}", e))
}
