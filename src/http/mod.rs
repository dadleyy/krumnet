extern crate http;
extern crate serde;
extern crate url;

pub use http::header;
pub use http::header::HeaderName;
pub use http::response::Builder;
pub use http::status::StatusCode;
pub use http::uri;
pub use http::uri::Uri;
pub use http::version::Version;
pub use http::{HeaderMap, HeaderValue, Method, Request};

pub use url::form_urlencoded as query;
pub use url::Url;

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
  Empty(StatusCode, Option<HeaderMap>),
  Redirect(String),
  Json(HttpResponse<D>),
}

impl<D> Response<D>
where
  D: Serialize,
{
  pub fn server_error() -> Self {
    Response::Empty(StatusCode::INTERNAL_SERVER_ERROR, None)
  }
  pub fn json(d: HttpResponse<D>) -> Self {
    Response::Json(d)
  }
  pub fn not_found(headers: Option<HeaderMap>) -> Self {
    Response::Empty(StatusCode::NOT_FOUND, headers)
  }
  pub fn redirect(destination: &String) -> Self {
    Response::Redirect(destination.clone())
  }
}

impl<D> std::fmt::Display for Response<D>
where
  D: Serialize,
{
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      Response::Empty(code, headers) => {
        let head = headers
          .as_ref()
          .map(|list| {
            list
              .iter()
              .map(format_header)
              .collect::<Vec<String>>()
              .iter()
              .map(|s| format!("{}\r\n", s))
              .collect::<String>()
          })
          .unwrap_or_default();

        return write!(formatter, "{:?} {}\r\n{}\r\n", Version::HTTP_11, code, head);
      }
      Response::Redirect(destination) => {
        let code = http::status::StatusCode::TEMPORARY_REDIRECT;
        return write!(
          formatter,
          "{:?} {}\r\nLocation: {}\r\n\r\n",
          Version::HTTP_11,
          code,
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
          "{:?} {}\r\n{}\r\n{}",
          version,
          status,
          head,
          payload.unwrap_or_default()
        )
      }
    }
  }
}
