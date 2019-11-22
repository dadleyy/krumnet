extern crate async_std;
extern crate elaine;
extern crate http;
extern crate serde;

use async_std::io::{Read as AsyncRead, Write as AsyncWrite};
use async_std::net::TcpListener;
use async_std::prelude::*;
use async_std::task;
use elaine::{recognize, RequestMethod};
use http::response::{Builder, Response};
use http::uri::Uri;
use serde::Serialize;
use std::io::{Error, ErrorKind};
use std::marker::Unpin;
use std::sync::Arc;

pub mod constants;

pub mod configuration;
use configuration::Configuration;

mod persistence;
use persistence::RecordStore;

mod session;
use session::SessionStore;

mod routes;
use routes::auth;

fn not_found() -> Result<Response<Option<u8>>, Error> {
  Builder::new()
    .status(404)
    .body(None)
    .map_err(|e| Error::new(ErrorKind::Other, e))
}

fn redirect<S>(location: S) -> Result<Response<Option<u8>>, Error>
where
  S: std::fmt::Display,
{
  Builder::new()
    .status(302)
    .header(http::header::LOCATION, format!("{}", location))
    .body(None)
    .map_err(|e| Error::new(ErrorKind::Other, e))
}

async fn write<C, D>(mut writer: C, data: Result<Response<Option<D>>, Error>) -> Result<(), Error>
where
  C: AsyncWrite + Unpin,
  D: Serialize,
{
  if let Err(e) = &data {
    println!("[warning] attempted to write a failed handler: {:?}", e);
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

async fn handle<T, S, R>(mut connection: T, session: S, records: R) -> Result<(), Error>
where
  T: AsyncRead + AsyncWrite + Unpin,
  S: std::ops::Deref<Target = SessionStore>,
  R: std::ops::Deref<Target = RecordStore>,
{
  let head = recognize(&mut connection).await?;
  match head.path() {
    Some(path) => {
      let uri = path
        .parse::<Uri>()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

      let auth = head.find_header("Authorization");

      println!("[debug] request - {} {:?}", uri.path(), auth);

      match (head.method(), uri.path()) {
        (Some(RequestMethod::GET), "/auth/callback") => {
          let res = auth::callback(uri, &session, &records).await;
          write(&mut connection, res).await
        }
        (Some(RequestMethod::GET), "/auth/identify") => {
          let res = auth::identify().await;
          write(&mut connection, res).await
        }
        (Some(RequestMethod::GET), "/auth/redirect") => {
          write(&mut connection, redirect(session.login_url())).await
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

  println!("[debug] connecting to record store");
  let records = Arc::new(RecordStore::open(&configuration)?);

  println!("[debug] connecting to session store");
  let session = Arc::new(SessionStore::open(&configuration).await?);

  println!("[debug] accepting incoming tcp streams");
  while let Some(stream) = incoming.next().await {
    match stream {
      Ok(connection) => {
        let records = records.clone();
        let session = session.clone();
        task::spawn(async {
          if let Err(e) = handle(connection, session, records).await {
            println!("[warning] unable to handle connection: {:?}", e);
          }
        });
      }
      Err(e) => {
        println!("[warning] invalid connection: {:?}", e);
        continue;
      }
    }
  }

  Ok(())
}
