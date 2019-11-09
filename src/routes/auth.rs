use http::response::{Builder, Response};
use http::status::StatusCode;
use http::Uri;
use serde::Deserialize;
use std::io::{Error, ErrorKind};
use url::form_urlencoded;

use crate::configuration::Configuration;
use crate::constants;

#[derive(Debug, PartialEq, Deserialize)]
struct TokenExchangePayload {
  access_token: String,
}

async fn exchange_code(code: &str, config: &Configuration) -> Result<TokenExchangePayload, Error> {
  let client = isahc::HttpClient::new().map_err(|e| Error::new(ErrorKind::Other, e))?;

  let encoded: String = form_urlencoded::Serializer::new(String::new())
    .append_pair("code", code)
    .append_pair("client_id", &config.google.client_id)
    .append_pair("client_secret", &config.google.client_secret)
    .append_pair("redirect_uri", &config.google.redirect_uri)
    .append_pair("grant_type", "authorization_code")
    .finish();

  match client.post(constants::google_token_url(), encoded) {
    Ok(mut response) if response.status() == StatusCode::OK => {
      let body = response.body_mut();
      let payload = match serde_json::from_reader(body) {
        Ok(p) => p,
        Err(e) => {
          return Err(Error::new(
            ErrorKind::Other,
            format!("unable to parse response body: {:?}", e),
          ));
        }
      };
      Ok(payload)
    }
    Ok(response) => Err(Error::new(
      ErrorKind::Other,
      format!("bad response from google sso: {:?}", response.status()),
    )),
    Err(e) => Err(Error::new(
      ErrorKind::Other,
      format!("unable to send code to google sso: {:?}", e),
    )),
  }
}

pub async fn callback(uri: Uri, config: &Configuration) -> Result<Response<Option<u8>>, Error> {
  let query = uri.query().unwrap_or_default().as_bytes();
  let code = match form_urlencoded::parse(query).find(|(key, _)| key == "code") {
    Some((_, code)) => code,
    None => {
      return Builder::new()
        .status(404)
        .body(None)
        .map_err(|e| Error::new(ErrorKind::Other, e))
    }
  };

  let payload = match exchange_code(&code, config).await {
    Ok(payload) => payload,
    Err(e) => {
      println!("[warning] unable ot exchange code: {}", e);
      return Builder::new()
        .status(404)
        .body(None)
        .map_err(|e| Error::new(ErrorKind::Other, e));
    }
  };

  println!("[debug] callback for {} {}", code, payload.access_token);

  Builder::new()
    .status(200)
    .body(None)
    .map_err(|e| Error::new(ErrorKind::Other, e))
}
