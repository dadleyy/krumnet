use std::io::{Error, ErrorKind};
use url::Url;

use crate::configuration::{Configuration, GoogleCredentials};

use crate::constants::{
  google_auth_url, google_token_url, GOOGLE_AUTH_CLIENT_ID_KEY, GOOGLE_AUTH_REDIRECT_URI_KEY,
  GOOGLE_AUTH_RESPONSE_TYPE_KEY, GOOGLE_AUTH_RESPONSE_TYPE_VALUE, GOOGLE_AUTH_SCOPE_KEY,
  GOOGLE_AUTH_SCOPE_VALUE,
};

#[derive(Debug, Clone)]
pub struct AuthorizationUrls {
  pub init: String,
  pub exchange: (String, GoogleCredentials),
  pub callback: String,
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
      exchange: (google_token_url(), configuration.google.clone()),
      callback: configuration.krumi.auth_uri.clone(),
    })
  }
}
