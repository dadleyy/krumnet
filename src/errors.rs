use std::io::{Error, ErrorKind};

pub fn humanize_error<E: std::error::Error>(e: E) -> Error {
  Error::new(ErrorKind::Other, format!("{}", e))
}

pub fn e<S: std::fmt::Display>(s: S) -> Error {
  Error::new(ErrorKind::Other, format!("{}", s))
}
