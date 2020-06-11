extern crate serde;

use log::{debug, warn};
use serde::Deserialize;
use std::env::var_os;
use std::fs::read;
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::str::FromStr;

const DEFAULT_CONFIG_FILE: &'static str = "krumnet-config.json";
const DEFAULT_POSTGRES_URI: &'static str = "postgresql://postgres@0.0.0.0:5432/krumnet";

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
  pub job_store: JobStoreConfiguration,

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
    let path = Path::new(DEFAULT_CONFIG_FILE);

    if path.exists() {
      debug!(
        "found '{}', attempting to load as default",
        DEFAULT_CONFIG_FILE
      );

      if let Ok(config) = Configuration::from_str(DEFAULT_CONFIG_FILE) {
        return config;
      }
    }

    let google = GoogleCredentials::default();
    let krumi = KrumiConfiguration::default();
    Configuration {
      google,
      krumi,
      addr: String::from("0.0.0.0:8080"),
      session_store: SessionStoreConfiguration::default(),
      record_store: RecordStoreConfiguration::default(),
      job_store: JobStoreConfiguration::default(),
    }
  }
}

impl FromStr for Configuration {
  type Err = Error;

  fn from_str(source: &str) -> Result<Self, Self::Err> {
    let buffer = read(source).and_then(|buffer| {
      String::from_utf8(buffer).map_err(|err| Error::new(ErrorKind::Other, err))
    })?;
    let result = serde_json::from_str::<Configuration>(buffer.as_str());

    result.map_err(|err| {
      warn!("unable to parse '{}': {:?}", source, err);
      Error::new(ErrorKind::Other, err)
    })
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
pub struct JobStoreConfiguration {
  #[serde(default)]
  pub queue_key: String,
  #[serde(default)]
  pub dequeue_key: String,
  #[serde(default)]
  pub map_key: String,
  #[serde(default)]
  pub redis_uri: String,
  #[serde(default)]
  pub queue_delay: u64,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RecordStoreConfiguration {
  #[serde(default = "RecordStoreConfiguration::default_url_from_env")]
  pub postgres_uri: String,
  pub redis_uri: String,
}

impl RecordStoreConfiguration {
  pub fn default_url_from_env() -> String {
    let attempt = std::env::var("DATABASE_URL");
    let out = attempt.unwrap_or(String::from(DEFAULT_POSTGRES_URI));
    debug!("attempting to pull record store url from env - {}", out);
    out
  }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SessionStoreConfiguration {
  pub redis_uri: String,
  pub secret: String,
  pub session_prefix: String,
  pub expiration_timeout: Option<u64>,
}

#[cfg(test)]
use std::env;

#[cfg(test)]
const CONFIG_VAR: &'static str = "KRUMNET_TEST_CONFIG_FILE";

#[cfg(test)]
pub fn load_test_config() -> Result<Configuration, Error> {
  let path = env::var(CONFIG_VAR).unwrap_or(String::from("krumnet-config.example.json"));
  Configuration::load(&path)
}

#[cfg(test)]
mod test {
  use crate::configuration::Configuration;

  #[test]
  fn from_file_exists() {
    let result = Configuration::load("ci/github-actions/krumnet-config.json");
    assert_eq!(result.is_ok(), true);
  }

  #[test]
  fn from_file_not_exists() {
    let result = Configuration::load("does-not-exist");
    assert_eq!(result.is_err(), true);
  }
}
