extern crate serde;

use serde::Deserialize;
use std::env::var_os;
use std::fs::read;
use std::io::{Error, ErrorKind};
use std::str::FromStr;
use url::Url;

use crate::constants::{
  google_auth_url, GOOGLE_AUTH_CLIENT_ID_KEY, GOOGLE_AUTH_REDIRECT_URI_KEY,
  GOOGLE_AUTH_RESPONSE_TYPE_KEY, GOOGLE_AUTH_RESPONSE_TYPE_VALUE, GOOGLE_AUTH_SCOPE_KEY,
  GOOGLE_AUTH_SCOPE_VALUE,
};

#[derive(Clone, Debug, Deserialize)]
pub struct Configuration {
  #[serde(default)]
  pub google: GoogleCredentials,

  #[serde(default)]
  pub krumi: KrumiConfiguration,

  #[serde(default)]
  pub session_store: SessionStoreConfiguration,

  #[serde(default)]
  pub record_store: RecordStoreConfiguration,

  #[serde(default)]
  pub addr: String,

  #[serde(default)]
  pub session_secret: String,
}

impl Configuration {
  pub fn login_url(&self) -> Result<String, Error> {
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
      .append_pair(GOOGLE_AUTH_CLIENT_ID_KEY, &self.google.client_id)
      .append_pair(GOOGLE_AUTH_REDIRECT_URI_KEY, &self.google.redirect_uri)
      .append_pair(GOOGLE_AUTH_SCOPE_KEY, GOOGLE_AUTH_SCOPE_VALUE);

    Ok(format!("{}", location.as_str()))
  }
}

impl Default for Configuration {
  fn default() -> Self {
    let google = GoogleCredentials::default();
    let krumi = KrumiConfiguration::default();
    Configuration {
      google,
      krumi,
      addr: String::from("0.0.0.0:8080"),
      session_store: SessionStoreConfiguration::default(),
      record_store: RecordStoreConfiguration::default(),
      session_secret: format!("{}", uuid::Uuid::new_v4()),
    }
  }
}

impl FromStr for Configuration {
  type Err = Error;

  fn from_str(source: &str) -> Result<Self, Self::Err> {
    let result = serde_json::from_str::<Configuration>(
      String::from_utf8(read(source)?)
        .or(Err(Error::from(ErrorKind::InvalidData)))?
        .as_str(),
    );

    if let Err(e) = &result {
      println!("[warning] unable to parse '{}': {:?}", source, e);
    }

    result.or(Err(Error::from(ErrorKind::InvalidData)))
  }
}

#[derive(Clone, Debug, Deserialize)]
pub struct GoogleCredentials {
  #[serde(default)]
  pub client_id: String,

  #[serde(default)]
  pub client_secret: String,

  #[serde(default)]
  pub redirect_uri: String,
}

impl Default for GoogleCredentials {
  fn default() -> Self {
    let client_id = var_os("GOOGLE_CLIENT_ID")
      .unwrap_or_default()
      .into_string()
      .unwrap_or_default();
    let client_secret = var_os("GOOGLE_CLIENT_SECRET")
      .unwrap_or_default()
      .into_string()
      .unwrap_or_default();
    let redirect_uri = var_os("GOOGLE_CLIENT_REDIRECT_URI")
      .unwrap_or_default()
      .into_string()
      .unwrap_or_default();

    Self::new(client_id, client_secret, redirect_uri)
  }
}

impl GoogleCredentials {
  pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
    GoogleCredentials {
      client_id,
      client_secret,
      redirect_uri,
    }
  }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct KrumiConfiguration {
  #[serde(default)]
  pub auth_uri: String,

  #[serde(default)]
  pub cors_origin: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RecordStoreConfiguration {
  pub postgres_uri: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SessionStoreConfiguration {
  pub redis_uri: String,
}
