use std::io::{Error, ErrorKind};
use std::time::SystemTime;

use async_std::net::TcpStream;
use async_std::sync::RwLock;

use jsonwebtoken::{encode, EncodingKey, Header};
use kramer::{execute, Arity, Command, Insertion, StringCommand};
use log::{info, trace, warn};
use serde::{Deserialize, Serialize};

use crate::configuration::Configuration;

#[derive(Debug, Serialize, Deserialize)]
struct SessionClaims {
  uid: String,
  created: SystemTime,
}

fn lookup_command<S: std::fmt::Display>(prefix: S, key: &String) -> StringCommand<String, String> {
  StringCommand::Get::<_, String>(Arity::One(format!("{}:{}", prefix, key)))
}

fn destroy_command<S: std::fmt::Display>(prefix: S, key: &String) -> Command<String, String> {
  Command::Del::<_, String>(Arity::One(format!("{}:{}", prefix, key)))
}

pub struct Session {
  _stream: RwLock<TcpStream>,
  _secret: String,
  _encoding_key: EncodingKey,
  _session_prefix: String,
}

impl Session {
  pub async fn open<C>(configuration: C) -> Result<Self, Error>
  where
    C: std::ops::Deref<Target = Configuration>,
  {
    let stream = TcpStream::connect(configuration.session_store.redis_uri.as_str()).await?;

    info!(
      "session store ready with secret: {}",
      configuration.session_store.secret
    );
    let key = EncodingKey::from_secret(configuration.session_store.secret.as_bytes());

    Ok(Session {
      _stream: RwLock::new(stream),
      _session_prefix: configuration.session_store.session_prefix.clone(),
      _secret: configuration.session_store.secret.clone(),
      _encoding_key: key,
    })
  }

  pub async fn destroy(&self, key: &String) -> Result<(), Error> {
    info!("removing key {}", key);
    let des = destroy_command(&self._session_prefix, key);
    let mut stream = self._stream.write().await;
    match execute(&mut (*stream), des).await? {
      kramer::Response::Item(kramer::ResponseValue::Integer(1)) => Ok(()),
      kramer::Response::Item(kramer::ResponseValue::Integer(0)) => {
        info!("unable to find session");
        return Ok(());
      }
      _ => Err(Error::new(
        ErrorKind::Other,
        format!("Unable to find key '{}' for deletion", key),
      )),
    }
  }

  pub async fn get(&self, key: &String) -> Result<String, Error> {
    let lookup = lookup_command(&self._session_prefix, key);
    trace!("writing command {} to redis connection", lookup);
    let mut stream = self._stream.write().await;

    match execute(&mut (*stream), lookup).await? {
      kramer::Response::Item(kramer::ResponseValue::String(id)) => Ok(id),
      r => {
        warn!("strange response from session lookup - {:?}", r);
        Err(Error::new(
          ErrorKind::Other,
          format!("Unable to find user for token '{}'", key),
        ))
      }
    }
  }

  pub async fn create<S>(&self, id: S) -> Result<String, Error>
  where
    S: std::fmt::Display,
  {
    let claims = SessionClaims {
      uid: format!("{}", id),
      created: SystemTime::now(),
    };

    let token = encode(&Header::default(), &claims, &self._encoding_key)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    let key = format!("{}:{}", self._session_prefix, token);
    let insert = StringCommand::Set(Arity::One((&key, &id)), None, Insertion::Always);
    let mut stream = self._stream.write().await;
    execute(&mut (*stream), insert).await?;
    info!("creating session for user id: {}", id);
    Ok(token)
  }
}
