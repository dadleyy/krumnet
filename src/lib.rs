extern crate async_std;
extern crate elaine;
extern crate log;

use std::io::Result;
use std::marker::Unpin;
use std::time::SystemTime;

use async_std::io::{Read as AsyncRead, Write as AsyncWrite};
use async_std::net::TcpListener;
use async_std::prelude::*;
use async_std::sync::Arc;
use async_std::task;
use elaine::{recognize, Head, RequestMethod};
use log::{debug, info};
use serde::Serialize;

pub mod authority;
pub mod configuration;
pub mod constants;
pub mod context;
pub mod errors;
pub mod http;
pub mod oauth;
pub mod records;
pub mod session;

pub use crate::authority::Authority;
pub use crate::configuration::{Configuration, GoogleCredentials};
pub use crate::context::{Context, ContextBuilder};
pub use crate::http::{Response, Uri};
pub use crate::records::RecordStore;
pub use crate::session::Session as SessionStore;

#[derive(Serialize)]
struct HealthCheckData {
  time: SystemTime,
}

impl Default for HealthCheckData {
  fn default() -> Self {
    HealthCheckData {
      time: SystemTime::now(),
    }
  }
}

fn extract_parts(head: &Head) -> Result<(RequestMethod, String)> {
  let method = head.method().ok_or(errors::e("invalid method"))?;
  let path = head.path().ok_or(errors::e("invalid path"))?;
  Ok((method, path))
}

async fn health_check(context: &Context) -> Result<Response> {
  info!("health check against context - '{:?}'", context);
  Ok(Response::ok_json(HealthCheckData::default())?.cors(context.cors()))
}

// Called for each new connection to the server, this is where requests are routed.
async fn route<T>(mut connection: T, builder: ContextBuilder) -> Result<()>
where
  T: AsyncRead + AsyncWrite + Unpin,
{
  let head = recognize(&mut connection).await?;
  let ctx = builder.for_request(&head)?;
  let (method, path) = extract_parts(&head)?;
  let uri = path.parse::<Uri>().map_err(errors::humanize_error)?;

  info!("request {} (context: {:?}", uri, &ctx);

  let response = match (method, uri.path()) {
    (RequestMethod::GET, "/auth/redirect") => {
      debug!("initiating oauth flow");
      oauth::redirect(&ctx)
    }
    (RequestMethod::GET, "/auth/callback") => {
      debug!("oauth callback");
      oauth::callback(&ctx, &uri).await
    }
    (RequestMethod::GET, "/health-check") => {
      info!("health-check - '{}'", path);
      health_check(&ctx).await
    }
    _ => {
      debug!("not-found - '{}'", path);
      Ok(Response::not_found())
    }
  }
  .unwrap_or_else(|e| {
    info!("request handler failed - {}", e);
    Response::default()
  });

  connection
    .write_all(format!("{}", response).as_bytes())
    .await
}

pub async fn serve(configuration: Configuration) -> Result<()> {
  let listener = TcpListener::bind(&configuration.addr).await?;
  let mut incoming = listener.incoming();

  info!("opening session store");
  let session = Arc::new(SessionStore::open(&configuration).await?);

  info!("opening record store");
  let records = Arc::new(RecordStore::open(&configuration).await?);

  info!("accepting incoming tcp streams");
  while let Some(stream) = incoming.next().await {
    match stream {
      Ok(connection) => {
        let builder = Context::builder()
          .configuration(&configuration)
          .session(session.clone())
          .records(records.clone());

        task::spawn(async {
          let result = route(connection, builder).await;

          if let Err(e) = result {
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
mod test_helpers {
  use async_std::io::Write;
  use async_std::task::{block_on, Context, Poll};
  use std::io::{Error, ErrorKind};
  use std::pin::Pin;

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
}
