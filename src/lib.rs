extern crate async_std;
extern crate elaine;
extern crate log;
extern crate serde;

use async_std::io::{Read as AsyncRead, Write as AsyncWrite};
use async_std::net::TcpListener;
use async_std::prelude::*;
use async_std::task;
use elaine::{recognize, RequestMethod};
use log::info;
use serde::Serialize;
use std::io::{Error, ErrorKind};
use std::marker::Unpin;
use std::sync::Arc;

pub mod constants;

pub mod http;
use crate::http::{Response as Res, StatusCode, Uri};

pub mod configuration;
use configuration::Configuration;

mod persistence;
use persistence::RecordStore;

mod authorization;
use authorization::{cors as cors_headers, Authorization, AuthorizationUrls};

mod interchange;

mod session;
use session::SessionStore;

mod routes;
use routes::{auth, not_found, redirect, server_error};

const USER_FOR_SESSION: &'static str = include_str!("data-store/user-for-session.sql");

pub trait SessionInterface: std::ops::Deref<Target = SessionStore> {}
impl<T> SessionInterface for T where T: std::ops::Deref<Target = SessionStore> {}

pub trait RecordInterface: std::ops::Deref<Target = RecordStore> {}
impl<T> RecordInterface for T where T: std::ops::Deref<Target = RecordStore> {}

// Given a response, writes it to our connection.
async fn write<C, D>(mut writer: C, data: Result<Res<D>, Error>) -> Result<(), Error>
where
  C: AsyncWrite + Unpin,
  D: Serialize,
{
  if let Err(e) = &data {
    info!("[warning] attempted to write a failed handler: {:?}", e);
  }

  info!("writing response");

  let res = data.unwrap_or_else(server_error);

  writer
    .write(format!("{}", res).as_bytes())
    .await
    .map(|_| ())
}

// Attempts to exchange an authorization token for a user id from the session store, subsequently
// loading the actual user information from the record store.
pub async fn load_authorization<S: SessionInterface, R: RecordInterface>(
  token: String,
  session: S,
  records: R,
) -> Result<Option<Authorization>, Error> {
  let uid = session.get(&token).await?;
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
      Some(Authorization(id, name, email, token))
    });

  info!("loaded tenant from auth header: {:?}", tenant);
  Ok(tenant)
}

// Called for each new connection to the server, this is where requests are routed.
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
      }
      .unwrap_or_else(|e| {
        info!("unable to load authorization - {}", e);
        None
      });

      info!("request - {} {:?}", uri.path(), auth);

      match (head.method(), uri.path()) {
        (Some(RequestMethod::OPTIONS), _) => {
          info!("received preflight CORS request, sending headers");
          let response = cors_headers(&authorization)
            .map(|headers| Res::Empty::<()>(StatusCode::OK, Some(headers)));
          write(&mut connection, response).await
        }
        (Some(RequestMethod::GET), "/auth/callback") => {
          let res = auth::callback(uri, &session, &records, &authorization).await;
          write(&mut connection, res).await
        }
        (Some(RequestMethod::GET), "/auth/destroy") => {
          let res = auth::destroy(&auth, &uri, &session, &authorization).await;
          write(&mut connection, res).await
        }
        (Some(RequestMethod::GET), "/auth/identify") => {
          let res = auth::identify(&auth, &records, &authorization).await;
          write(&mut connection, res).await
        }
        (Some(RequestMethod::GET), "/auth/redirect") => {
          let res = Ok(redirect::<()>(format!("{}", authorization.init)));
          write(&mut connection, res).await
        }
        _ => write(&mut connection, Ok(not_found::<()>())).await,
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

#[cfg(test)]
mod test {
  use async_std::io::Write;
  use async_std::task::{block_on, Context, Poll};
  use std::io::{Error, ErrorKind};
  use std::pin::Pin;

  use crate::http::Response;
  use crate::write as write_response;

  struct AsyncStringBuffer {
    contents: String,
  }

  impl AsyncStringBuffer {
    pub fn new() -> Self {
      AsyncStringBuffer {
        contents: String::new(),
      }
    }
  }

  impl Write for AsyncStringBuffer {
    fn poll_write(
      mut self: Pin<&mut Self>,
      _context: &mut Context,
      data: &[u8],
    ) -> Poll<Result<usize, Error>> {
      match std::str::from_utf8(data) {
        Ok(parsed) => {
          self.contents.push_str(parsed);
          Poll::Ready(Ok(data.len()))
        }
        Err(e) => Poll::Ready(Err(Error::new(ErrorKind::Other, e))),
      }
    }

    fn poll_flush(self: Pin<&mut Self>, _context: &mut Context) -> Poll<Result<(), Error>> {
      Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _context: &mut Context) -> Poll<Result<(), Error>> {
      Poll::Ready(Ok(()))
    }
  }

  #[test]
  fn write_redirect() {
    let mut buffer = AsyncStringBuffer::new();
    let result = block_on(async {
      let dest = String::from("http://github.com/krumpled/krumnet");
      let out = Ok(Response::redirect(&dest));
      write_response::<_, ()>(&mut buffer, out).await
    });
    assert!(result.is_ok());
    assert_eq!(
      buffer.contents,
      "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://github.com/krumpled/krumnet\r\n\r\n",
    );
  }

  #[test]
  fn write_not_found() {
    let mut buffer = AsyncStringBuffer::new();
    let result = block_on(async {
      let out = Ok(Response::not_found(None));
      write_response::<_, ()>(&mut buffer, out).await
    });
    assert!(result.is_ok());
    assert_eq!(buffer.contents, "HTTP/1.1 404 Not Found\r\n\r\n");
  }

  #[test]
  fn write_server_error() {
    let mut buffer = AsyncStringBuffer::new();
    let result = block_on(async {
      let err = Err(Error::new(ErrorKind::Other, ""));
      write_response::<_, ()>(&mut buffer, err).await
    });
    assert!(result.is_ok());
    assert_eq!(
      buffer.contents,
      "HTTP/1.1 500 Internal Server Error\r\n\r\n",
    );
  }
}
