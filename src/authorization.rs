use std::io::{Error, ErrorKind};

use crate::configuration::{Configuration, GoogleCredentials};
use crate::http::{header, HeaderMap, HeaderValue, Url};

use crate::constants::{
  google_auth_url, google_info_url, google_token_url, GOOGLE_AUTH_CLIENT_ID_KEY,
  GOOGLE_AUTH_REDIRECT_URI_KEY, GOOGLE_AUTH_RESPONSE_TYPE_KEY, GOOGLE_AUTH_RESPONSE_TYPE_VALUE,
  GOOGLE_AUTH_SCOPE_KEY, GOOGLE_AUTH_SCOPE_VALUE,
};

#[derive(Debug, Clone)]
pub struct Authorization(pub String, pub String, pub String, pub String);

#[derive(Debug, Clone)]
pub struct AuthorizationUrls {
  pub init: String,
  pub exchange: (String, GoogleCredentials),
  pub identify: String,
  pub callback: String,
  pub cors_origin: String,
}

impl AuthorizationUrls {
  pub async fn open(configuration: &Configuration) -> Result<Self, Error> {
    let url = google_auth_url();

    let mut location = url
      .parse::<Url>()
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    location
      .query_pairs_mut()
      .clear()
      .append_pair(
        GOOGLE_AUTH_RESPONSE_TYPE_KEY,
        GOOGLE_AUTH_RESPONSE_TYPE_VALUE,
      )
      .append_pair(GOOGLE_AUTH_CLIENT_ID_KEY, &configuration.google.client_id)
      .append_pair(
        GOOGLE_AUTH_REDIRECT_URI_KEY,
        &configuration.google.redirect_uri,
      )
      .append_pair(GOOGLE_AUTH_SCOPE_KEY, GOOGLE_AUTH_SCOPE_VALUE);

    let authorization_url = format!("{}", location.as_str());

    Ok(AuthorizationUrls {
      init: authorization_url,
      cors_origin: configuration.krumi.cors_origin.clone(),
      identify: google_info_url(),
      exchange: (google_token_url(), configuration.google.clone()),
      callback: configuration.krumi.auth_uri.clone(),
    })
  }
}

pub fn cors(urls: &AuthorizationUrls) -> Result<HeaderMap, Error> {
  let mut headers = HeaderMap::with_capacity(5);
  headers.insert(
    header::ACCESS_CONTROL_ALLOW_ORIGIN,
    HeaderValue::from_str(&urls.cors_origin).map_err(|e| Error::new(ErrorKind::Other, e))?,
  );
  headers.insert(
    header::ACCESS_CONTROL_ALLOW_HEADERS,
    HeaderValue::from_str("Authorization").map_err(|e| Error::new(ErrorKind::Other, e))?,
  );
  return Ok(headers);
}
