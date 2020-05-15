use std::io::Result;
use std::marker::Unpin;

use async_std::io::Read;
use log::{debug, info};
use serde::Deserialize;
use serde_json::from_slice as deserialize;

use crate::{interchange, read_size_async, Authority, Context, Response};

#[derive(Deserialize, Debug)]
pub struct Payload {
  kind: String,
}

pub async fn create<R>(context: &Context, reader: &mut R) -> Result<Response>
where
  R: Read + Unpin,
{
  let uid = match context.authority() {
    Authority::User { id: s, token: _ } => s,
    _ => return Ok(Response::not_found().cors(context.cors())),
  };

  debug!(
    "authorized action, attempting to read {} bytes",
    context.pending()
  );

  let contents = read_size_async(reader, context.pending()).await?;

  info!(
    "creating new lobby for user '{}' ({} pending bytes)",
    uid,
    context.pending()
  );

  context
    .jobs()
    .queue(&interchange::jobs::Job::CreateLoby {
      creator: uid.clone(),
      result: None,
    })
    .await?;

  let payload = deserialize::<Payload>(&contents)?;
  debug!("buffer after read {:?}", payload);
  Ok(Response::default().cors(context.cors()))
}
