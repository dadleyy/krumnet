extern crate serde;

use log::info;
use serde::Deserialize;
use std::env::var_os;
use std::fs::read;
use std::io::{Error, ErrorKind};
use std::str::FromStr;

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
}

impl Configuration {
  pub fn load(source: &str) -> Result<Self, Error> {
    Configuration::from_str(source)
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
      info!("[warning] unable to parse '{}': {:?}", source, e);
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
  pub secret: String,
  pub session_prefix: String,
}
