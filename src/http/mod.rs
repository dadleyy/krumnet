extern crate http;
extern crate serde;

pub use http::header::HeaderName;
pub use http::response::Builder;
pub use http::status::StatusCode;
pub use http::uri::Uri;
pub use http::version::Version;
pub use http::{HeaderValue, Method, Request};

use http::response::Response as HttpResponse;
use log::info;
use serde::Serialize;

fn format_header(pair: (&HeaderName, &HeaderValue)) -> String {
  let (key, value) = pair;
  format!("{}: {}", key, value.to_str().unwrap_or(""))
}

pub enum Response<D>
where
  D: Serialize,
{
  Empty(StatusCode),
  Redirect(String),
  Json(HttpResponse<D>),
}

impl<D> Response<D>
where
  D: Serialize,
{
  pub fn server_error() -> Self {
    Response::Empty(StatusCode::INTERNAL_SERVER_ERROR)
  }
  pub fn json(d: HttpResponse<D>) -> Self {
    Response::Json(d)
  }
  pub fn not_found() -> Self {
    Response::Empty(StatusCode::NOT_FOUND)
  }
  pub fn redirect(destination: String) -> Self {
    Response::Redirect(destination)
  }
}

impl<D> std::fmt::Display for Response<D>
where
  D: Serialize,
{
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      Response::Empty(code) => {
        let reason = code.canonical_reason().unwrap_or_default();
        return write!(
          formatter,
          "{:?} {} {}\r\n\r\n",
          Version::HTTP_11,
          code,
          reason
        );
      }
      Response::Redirect(destination) => {
        let code = http::status::StatusCode::TEMPORARY_REDIRECT;
        let reason = code.canonical_reason().unwrap_or_default();
        return write!(
          formatter,
          "{:?} {} {}\r\nLocation: {}\r\n\r\n",
          Version::HTTP_11,
          code,
          reason,
          destination,
        );
      }
      Response::Json(response) => {
        let (version, status, headers, body) = (
          response.version(),
          response.status(),
          response.headers(),
          response.body(),
        );

        let mut headers = headers.iter().map(format_header).collect::<Vec<String>>();

        let reason = status.canonical_reason().unwrap_or_default();
        let code = status.as_str();
        let payload = match serde_json::to_string(&body) {
          Ok(data) => {
            info!("serialized payload - {}", data);
            let len = data.len().into();
            let nam = HeaderName::from_static("content-length");
            let typ = HeaderName::from_static("content-type");
            let jso = HeaderValue::from_static("application/json");
            headers.push(format_header((&nam, &len)));
            headers.push(format_header((&typ, &jso)));
            Some(data)
          }
          Err(e) => {
            info!("unable to serialize payload - {}", e);
            None
          }
        };

        let head = headers
          .iter()
          .map(|s| format!("{}\r\n", s))
          .collect::<String>();

        write!(
          formatter,
          "{:?} {} {}\r\n{}\r\n{}",
          version,
          code,
          reason,
          head,
          payload.unwrap_or_default()
        )
      }
    }
  }
}
