extern crate async_std;
extern crate elaine;
extern crate http;
extern crate log;
extern crate serde;

use async_std::io::{Read as AsyncRead, Write as AsyncWrite};
use async_std::net::TcpListener;
use async_std::prelude::*;
use async_std::task;
use elaine::{recognize, RequestMethod};
use http::response::Response;
use http::uri::Uri;
use log::info;
use serde::Serialize;
use std::io::{Error, ErrorKind};
use std::marker::Unpin;
use std::sync::Arc;

pub mod constants;

pub mod configuration;
use configuration::Configuration;

mod persistence;
use persistence::RecordStore;

mod authorization;
use authorization::{Authorization, AuthorizationUrls};

mod session;
use session::SessionStore;

mod routes;
use routes::{auth, not_found, redirect};

const USER_FOR_SESSION: &'static str = include_str!("data-store/user-for-session.sql");

async fn write<C, D>(mut writer: C, data: Result<Response<Option<D>>, Error>) -> Result<(), Error>
where
  C: AsyncWrite + Unpin,
  D: Serialize,
{
  if let Err(e) = &data {
    info!("[warning] attempted to write a failed handler: {:?}", e);
  }

  let (top, _) = data
    .unwrap_or(
      Response::builder()
        .status(500)
        .body(None)
        .map_err(|e| Error::new(ErrorKind::Other, e))?,
    )
    .into_parts();

  let reason = top.status.canonical_reason().unwrap_or_default();
  let headers = top
    .headers
    .iter()
    .map(|(key, value)| {
      format!(
        "{}: {}\r\n",
        key.as_str(),
        value.to_str().unwrap_or_default()
      )
    })
    .collect::<String>();
  let code = top.status.as_str();
  let serialized = format!("HTTP/1.1 {} {}\r\n{}\r\n", code, reason, headers);

  writer.write(serialized.as_bytes()).await.map(|_| ())
}

pub trait SessionInterface: std::ops::Deref<Target = SessionStore> {}
impl<T> SessionInterface for T where T: std::ops::Deref<Target = SessionStore> {}

pub trait RecordInterface: std::ops::Deref<Target = RecordStore> {}
impl<T> RecordInterface for T where T: std::ops::Deref<Target = RecordStore> {}

pub async fn load_authorization<S: SessionInterface, R: RecordInterface>(
  token: String,
  session: S,
  records: R,
) -> Result<Option<Authorization>, Error> {
  let uid = session.get(token).await?;
  let mut conn = records.get()?;
  let tenant = conn
    .query(USER_FOR_SESSION, &[&uid])
    .map_err(|e| Error::new(ErrorKind::Other, e))?
    .iter()
    .nth(0)
    .and_then(|row| {
      let id = row.try_get::<_, String>(0).ok()?;
      let name = row.try_get::<_, String>(1).ok()?;
      let email = row.try_get::<_, String>(2).ok()?;
      info!("found user '{:?}' {:?} {:?}", id, name, email);
      Some(Authorization(id, name, email))
    });

  info!("loaded tenant from auth header: {:?}", tenant);
  Ok(tenant)
}

async fn handle<T, S, R, A>(
  mut connection: T,
  session: S,
  records: R,
  authorization: A,
) -> Result<(), Error>
where
  T: AsyncRead + AsyncWrite + Unpin,
  S: SessionInterface,
  R: RecordInterface,
  A: std::ops::Deref<Target = AuthorizationUrls>,
{
  let head = recognize(&mut connection).await?;
  match head.path() {
    Some(path) => {
      let uri = path
        .parse::<Uri>()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

      let auth = match head.find_header("Authorization") {
        Some(key) => load_authorization(key, session.deref(), records.deref()).await,
        None => Ok(None),
      };

      info!("request - {} {:?}", uri.path(), auth);

      match (head.method(), uri.path()) {
        (Some(RequestMethod::GET), "/auth/callback") => {
          let res = auth::callback(uri, &session, &records, &authorization).await;
          write(&mut connection, res).await
        }
        (Some(RequestMethod::GET), "/auth/identify") => {
          let res = auth::identify().await;
          write(&mut connection, res).await
        }
        (Some(RequestMethod::GET), "/auth/redirect") => {
          write(&mut connection, redirect(format!("{}", authorization.init))).await
        }
        _ => write(&mut connection, not_found()).await,
      }
    }
    None => Ok(()),
  }
}

pub async fn run(configuration: Configuration) -> Result<(), Error> {
  let listener = TcpListener::bind(&configuration.addr).await?;
  let mut incoming = listener.incoming();

  info!("connecting to record store");
  let records = Arc::new(RecordStore::open(&configuration)?);

  info!("connecting to session store");
  let session = Arc::new(SessionStore::open(&configuration).await?);

  info!("creating authorizaton urls");
  let authorization_urls = Arc::new(AuthorizationUrls::open(&configuration).await?);

  info!("accepting incoming tcp streams");
  while let Some(stream) = incoming.next().await {
    match stream {
      Ok(connection) => {
        let records = records.clone();
        let session = session.clone();
        let auth = authorization_urls.clone();
        task::spawn(async {
          if let Err(e) = handle(connection, session, records, auth).await {
            info!("[warning] unable to handle connection: {:?}", e);
          }
        });
      }
      Err(e) => {
        info!("[warning] invalid connection: {:?}", e);
        continue;
      }
    }
  }

  Ok(())
}
