use async_std::net::TcpStream;
use jsonwebtoken::{decode, encode, Header};
use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind};
use url::Url;

use crate::configuration::{Configuration, GoogleCredentials};

use crate::constants::{
  google_auth_url, GOOGLE_AUTH_CLIENT_ID_KEY, GOOGLE_AUTH_REDIRECT_URI_KEY,
  GOOGLE_AUTH_RESPONSE_TYPE_KEY, GOOGLE_AUTH_RESPONSE_TYPE_VALUE, GOOGLE_AUTH_SCOPE_KEY,
  GOOGLE_AUTH_SCOPE_VALUE,
};

pub struct SessionStore {
  _stream: TcpStream,
  _login_url: String,
  _google: GoogleCredentials,
  _secret: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SessionClaims {
  uid: String,
}

impl SessionStore {
  pub async fn open<C>(configuration: C) -> Result<Self, Error>
  where
    C: std::ops::Deref<Target = Configuration>,
  {
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

    let login_url = format!("{}", location.as_str());
    let stream = TcpStream::connect(configuration.session_store.redis_uri.as_str()).await?;

    println!(
      "[debug] session store ready with secret: {}",
      configuration.session_store.secret
    );

    Ok(SessionStore {
      _stream: stream,
      _login_url: login_url,
      _secret: configuration.session_store.secret.clone(),
      _google: configuration.google.clone(),
    })
  }

  pub async fn create<S>(&self, id: S) -> Result<String, Error>
  where
    S: std::fmt::Display,
  {
    let claims = SessionClaims {
      uid: format!("{}", id),
    };
    let token = encode(&Header::default(), &claims, self._secret.as_bytes())
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    println!("[debug] creating session for user id: {}", id);
    Ok(token)
  }

  pub fn login_url(&self) -> String {
    self._login_url.clone()
  }

  pub fn google(&self) -> GoogleCredentials {
    self._google.clone()
  }
}
