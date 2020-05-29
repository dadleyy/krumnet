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
use log::{debug, info, warn};
use serde::Serialize;

pub mod authority;
pub mod configuration;
pub mod constants;
pub mod context;
pub mod errors;
pub mod http;
pub mod interchange;
pub mod jobs;
pub mod names;
pub mod oauth;
pub mod records;
pub mod routes;
pub mod session;

pub use crate::authority::Authority;
pub use crate::configuration::{Configuration, GoogleCredentials};
pub use crate::context::{Context, ContextBuilder};
pub use crate::http::{read_size_async, Response, Uri};
pub use crate::jobs::JobStore;
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
  debug!("recognized request - '{:?}'", head.path());
  let ctx = builder.for_request(&head).await?;
  let (method, path) = extract_parts(&head)?;
  let uri = path.parse::<Uri>().map_err(errors::humanize_error)?;

  info!("request {} (context: {:?})", uri, &ctx);

  let response = match (method, uri.path()) {
    (RequestMethod::OPTIONS, _) => {
      debug!("cors preflight request");
      Ok(Response::default().cors(ctx.cors()))
    }
    // Authentication routing
    (RequestMethod::GET, "/auth/redirect") => {
      debug!("initiating oauth flow");
      oauth::redirect(&ctx)
    }
    (RequestMethod::GET, "/auth/identify") => routes::identify(&ctx).await,
    (RequestMethod::GET, "/auth/destroy") => routes::destroy(&ctx, &uri).await,
    (RequestMethod::GET, "/auth/callback") => {
      debug!("oauth callback");
      oauth::callback(&ctx, &uri).await
    }
    // Basic health check for sanity
    (RequestMethod::GET, "/health-check") => {
      info!("health-check - '{}'", path);
      health_check(&ctx).await
    }

    // Jobs
    (RequestMethod::GET, "/jobs") => routes::jobs::find(&ctx, &uri).await,

    // Lobbies
    (RequestMethod::GET, "/lobbies") => routes::lobbies::find(&ctx, &uri).await,
    (RequestMethod::POST, "/lobbies") => routes::lobbies::create(&ctx, &mut connection).await,

    (RequestMethod::POST, "/lobby-memberships") => {
      routes::lobby_memberships::create_membership(&ctx, &mut connection).await
    }
    (RequestMethod::DELETE, "/lobby-memberships") => {
      routes::lobby_memberships::destroy_membership(&ctx, &mut connection).await
    }

    (RequestMethod::POST, "/games") => routes::games::create(&ctx, &mut connection).await,
    (RequestMethod::GET, "/games") => routes::games::find(&ctx, &uri).await,

    (RequestMethod::GET, "/rounds") => routes::rounds::find(&ctx, &uri).await,

    (RequestMethod::POST, "/round-entries") => {
      routes::games::create_entry(&ctx, &mut connection).await
    }

    _ => {
      debug!("not-found - '{}'", path);
      Ok(Response::not_found().cors(ctx.cors()))
    }
  }
  .unwrap_or_else(|e| {
    warn!("request handler failed - {}", e);
    Response::failed().cors(ctx.cors())
  });

  connection
    .write(format!("{}", response).as_bytes())
    .await
    .map(|_| ())
}

pub async fn serve(configuration: Configuration) -> Result<()> {
  let listener = TcpListener::bind(&configuration.addr).await?;
  let mut incoming = listener.incoming();

  info!("opening session store");
  let session = Arc::new(SessionStore::open(&configuration).await?);

  info!("opening job store");
  let jobs = Arc::new(JobStore::open(&configuration).await?);

  info!("opening record store");
  let records = Arc::new(RecordStore::open(&configuration).await?);

  info!("accepting incoming tcp streams");
  while let Some(stream) = incoming.next().await {
    match stream {
      Ok(mut connection) => {
        let builder = Context::builder()
          .configuration(&configuration)
          .jobs(jobs.clone())
          .session(session.clone())
          .records(records.clone());

        task::spawn(async move {
          let result = route(&mut connection, builder).await;

          if let Err(e) = result {
            info!("[warning] unable to handle connection: {:?}", e);
          }

          connection.shutdown(std::net::Shutdown::Both)
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
