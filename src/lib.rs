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
use std::io::{Error, Result};
use std::marker::Unpin;
use std::sync::Arc;

pub mod constants;

pub mod http;
use crate::http::{Response, StatusCode, Uri};

pub mod configuration;
pub use configuration::{Configuration, GoogleCredentials};

mod authorization;
use authorization::{Authorization, AuthorizationUrls};

mod context;
use context::{StaticContext, StaticContextBuilder};

mod errors;
mod interchange;

mod session;
use session::SessionStore;

mod records;
pub use records::{Provisioner, RecordStore};

mod routes;
use routes::auth;

// Given a response, writes it to our connection.
async fn write<C, D>(mut writer: C, data: Result<Response<D>>) -> Result<()>
where
  C: AsyncWrite + Unpin,
  D: Serialize,
{
  if let Err(e) = &data {
    info!("[warning] attempted to write a failed handler: {:?}", e);
  }

  let res = data.unwrap_or_else(|e| {
    info!("reponse handler error - {}", e);
    Response::server_error()
  });

  writer
    .write(format!("{}", res).as_bytes())
    .await
    .map(|_| ())
}

// Called for each new connection to the server, this is where requests are routed.
async fn handle<T>(mut connection: T, context: StaticContextBuilder) -> Result<()>
where
  T: AsyncRead + AsyncWrite + Unpin,
{
  let head = recognize(&mut connection).await?;
  match head.path() {
    Some(path) => {
      let uri = path.parse::<Uri>().map_err(errors::humanize_error)?;
      let ctx = context.for_request(&head).await?;

      info!("request - {}", uri.path());

      match (head.method(), uri.path()) {
        (Some(RequestMethod::OPTIONS), _) => {
          info!("received preflight CORS request, sending headers");
          let response = ctx
            .cors()
            .map(|headers| Response::Empty::<()>(StatusCode::OK, Some(headers)));
          write(&mut connection, response).await
        }
        /*
        (Some(RequestMethod::GET), "/provision") => {
          // let response = provisions::find(&ctx).await;
          let response = Response::not_found(None);
          write(&mut connection, response).await
        }
        (Some(RequestMethod::POST), "/provision") => {
          // let response = provisions::create(&ctx, &mut connection).await;
          let response = Response::not_found(None);
          write(&mut connection, response).await
        }
        */
        (Some(RequestMethod::GET), "/auth/callback") => {
          let response = auth::callback(&ctx, &uri).await;
          write(&mut connection, response).await
        }
        (Some(RequestMethod::GET), "/auth/destroy") => {
          let response = auth::destroy(&ctx, &uri).await;
          write(&mut connection, response).await
        }
        (Some(RequestMethod::GET), "/auth/identify") => {
          let response = auth::identify(&ctx).await;
          write(&mut connection, response).await
        }
        (Some(RequestMethod::GET), "/auth/redirect") => {
          let destination = format!("{}", ctx.urls().init);
          let response = Ok(Response::redirect(&destination) as Response<()>);
          write(&mut connection, response).await
        }
        _ => {
          let response = Ok(Response::not_found(None) as Response<()>);
          write(&mut connection, response).await
        }
      }
    }
    None => Ok(()),
  }
}

pub async fn run(configuration: Configuration) -> Result<()> {
  let listener = TcpListener::bind(&configuration.addr).await?;
  let mut incoming = listener.incoming();

  info!("connecting to record store");
  let records = Arc::new(RecordStore::open(&configuration).await?);
  let provisioner = Arc::new(Provisioner::open(&configuration).await?);

  info!("connecting to session store");
  let session = Arc::new(SessionStore::open(&configuration).await?);

  info!("creating authorizaton urls");
  let authorization_urls = Arc::new(AuthorizationUrls::open(&configuration).await?);

  info!("accepting incoming tcp streams");
  while let Some(stream) = incoming.next().await {
    match stream {
      Ok(connection) => {
        let ctx = StaticContextBuilder::new()
          .session(session.clone())
          .records(records.clone())
          .provisioner(provisioner.clone())
          .urls(authorization_urls.clone());

        task::spawn(async {
          if let Err(e) = handle(connection, ctx).await {
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
