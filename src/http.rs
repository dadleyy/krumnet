extern crate http;

use http::header::{
  HeaderName, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
  ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_REQUEST_HEADERS, AUTHORIZATION, CONTENT_LENGTH,
  CONTENT_TYPE, LOCATION,
};
pub use http::{header, Method, Request, StatusCode, Uri};
pub use url::form_urlencoded as query;
pub use url::Url;

use log::{debug, info};
use std::io::Result;

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
    info!("building json response");
    header_map.push((CONTENT_TYPE, "application/json".to_string()));
    Ok(Response(StatusCode::OK, header_map, Payload::String(vec)))
  }

  pub fn not_found() -> Response {
    Response(StatusCode::NOT_FOUND, HeaderMap::default(), Payload::Empty)
  }

  pub fn redirect<S: std::fmt::Display>(destination: &S) -> Response {
    let mut header_map = HeaderMap::default();
    header_map.push((LOCATION, format!("{}", destination)));
    Response(StatusCode::TEMPORARY_REDIRECT, header_map, Payload::Empty)
  }

  pub fn cors(self, origin: String) -> Self {
    let Response(code, mut header_map, body) = self;

    debug!("adding cors headers");
    header_map.push((ACCESS_CONTROL_ALLOW_ORIGIN, origin));
    header_map.push((ACCESS_CONTROL_ALLOW_HEADERS, AUTHORIZATION.to_string()));
    header_map.push((ACCESS_CONTROL_REQUEST_HEADERS, CONTENT_TYPE.to_string()));
    header_map.push((
      ACCESS_CONTROL_ALLOW_METHODS,
      "post, get, put, delete".to_string(),
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
      .map(|(v, k)| format!("{}: {}\r\n", v, k))
      .collect::<String>();

    write!(formatter, "HTTP/1.0 {}\r\n{}\r\n{}", code, headers, body)
  }
}
