extern crate http;

use async_std::io::{timeout, Read};
use async_std::prelude::*;
use http::header::{
  HeaderName, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
  ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_REQUEST_HEADERS, CONTENT_LENGTH, CONTENT_TYPE,
  LOCATION,
};
use log::{debug, info};
use std::io::{Error, ErrorKind, Result};
use std::marker::Unpin;
use std::time::Duration;

use crate::constants::MAX_FILE_SIZE;
pub use http::header::AUTHORIZATION;
pub use http::{header, Method, Request, StatusCode, Uri};
pub use url::form_urlencoded as query;
pub use url::Url;

pub fn query_values<S: std::fmt::Display>(uri: &Uri, key: S) -> Vec<String> {
  let q = uri.query().unwrap_or_default().as_bytes();
  let target = format!("{}", key);

  query::parse(q)
    .filter_map(|(k, v)| {
      if k.to_string() == target {
        Some(String::from(v))
      } else {
        None
      }
    })
    .collect::<Vec<String>>()
}

pub async fn read_size_async<R>(reader: &mut R, size: usize) -> Result<Vec<u8>>
where
  R: Read + Unpin,
{
  if size > MAX_FILE_SIZE {
    let m = format!("requested read too large - {}", size);
    let e = Error::new(ErrorKind::Other, m);
    return Err(e);
  }
  timeout(Duration::from_millis(300), async {
    let mut contents: Vec<u8> = Vec::with_capacity(size);
    info!("inside timeout, reading {} bytes", size);
    reader.take(size as u64).read_to_end(&mut contents).await?;
    Ok(contents)
  })
  .await
}

pub type HeaderMap = Vec<(HeaderName, String)>;

#[derive(Debug)]
pub enum Payload {
  Bytes(Vec<u8>),
  String(String),
  Empty,
}

impl Default for Payload {
  fn default() -> Self {
    Payload::Empty
  }
}

impl std::fmt::Display for Payload {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      Payload::Bytes(b) => {
        let stringified = String::from_utf8(b.to_vec()).unwrap_or_default();
        write!(formatter, "{}", stringified)
      }
      Payload::String(s) => write!(formatter, "{}", s),
      Payload::Empty => write!(formatter, ""),
    }
  }
}

impl Payload {
  pub fn len(&self) -> Option<usize> {
    match self {
      Payload::Bytes(v) => Some(v.len()),
      Payload::String(s) => Some(s.len()),
      Payload::Empty => None,
    }
  }
}

#[derive(Debug, Default)]
pub struct Response(StatusCode, HeaderMap, Payload);

impl Response {
  pub fn ok_json<S: serde::Serialize>(data: S) -> Result<Self> {
    let vec = serde_json::to_string(&data)?;
    let mut header_map = HeaderMap::default();
    header_map.push((CONTENT_TYPE, "application/json; charset=utf-8".to_string()));
    Ok(Response(StatusCode::OK, header_map, Payload::String(vec)))
  }

  pub fn bad_request<S: std::fmt::Display>(reason: S) -> Self {
    let mut header_map = HeaderMap::default();
    header_map.push((CONTENT_TYPE, "text/plain; charset=utf-8".to_string()));
    Response(
      StatusCode::BAD_REQUEST,
      header_map,
      Payload::String(format!("{}", reason)),
    )
  }

  pub fn failed() -> Self {
    Response(
      StatusCode::BAD_REQUEST,
      HeaderMap::default(),
      Payload::Empty,
    )
  }

  pub fn unauthorized() -> Self {
    Response(
      StatusCode::UNAUTHORIZED,
      HeaderMap::default(),
      Payload::Empty,
    )
  }

  pub fn not_found() -> Self {
    Response(StatusCode::NOT_FOUND, HeaderMap::default(), Payload::Empty)
  }

  pub fn redirect<S: std::fmt::Display>(destination: &S) -> Self {
    let mut header_map = HeaderMap::default();
    header_map.push((LOCATION, format!("{}", destination)));
    Response(StatusCode::TEMPORARY_REDIRECT, header_map, Payload::Empty)
  }

  pub fn cors(self, origin: String) -> Self {
    let Response(code, mut header_map, body) = self;

    debug!("adding cors headers");
    header_map.push((ACCESS_CONTROL_ALLOW_ORIGIN, origin));
    header_map.push((
      ACCESS_CONTROL_ALLOW_HEADERS,
      format!("{}, {}", AUTHORIZATION, CONTENT_TYPE),
    ));
    header_map.push((ACCESS_CONTROL_REQUEST_HEADERS, CONTENT_TYPE.to_string()));
    header_map.push((
      ACCESS_CONTROL_ALLOW_METHODS,
      "POST, GET, PUT, DELETE".to_string(),
    ));

    Response(code, header_map, body)
  }
}

impl std::fmt::Display for Response {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    let Response(code, header_map, body) = self;
    let lenh = body.len().map(|b| (CONTENT_LENGTH, format!("{}", b)));

    let headers = header_map
      .iter()
      .chain(lenh.iter())
      .chain(Some((header::CONNECTION, "close".to_string())).iter())
      .map(|(v, k)| format!("{}: {}\r\n", v, k))
      .collect::<String>();

    write!(formatter, "HTTP/1.1 {}\r\n{}\r\n{}", code, headers, body)
  }
}

#[cfg(test)]
mod test {
  use super::Response;

  #[test]
  fn not_found() {
    let res = Response::not_found();
    assert_eq!(
      format!("{}", res),
      "HTTP/1.1 404 Not Found\r\nconnection: close\r\n\r\n"
    );
  }
}
