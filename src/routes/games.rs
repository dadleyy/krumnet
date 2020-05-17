use log::{debug, warn};
use serde::Deserialize;
use serde_json::from_slice as deserialize;
use std::io::Result;

use super::lobbies::LOAD_LOBBY_DETAILS;
use crate::{interchange, read_size_async, Authority, Context, Response};

#[derive(Deserialize)]
pub struct CreatePayload {
  pub lobby_id: String,
}

pub async fn create<R>(context: &Context, reader: &mut R) -> Result<Response>
where
  R: async_std::io::Read + std::marker::Unpin,
{
  let uid = match context.authority() {
    Authority::None => return Ok(Response::not_found().cors(context.cors())),
    Authority::User { id, .. } => id,
  };
  debug!("creating new game for user - {}", uid);
  let contents = read_size_async(reader, context.pending()).await?;
  let payload = deserialize::<CreatePayload>(&contents)?;

  if let None = context
    .records()
    .query(LOAD_LOBBY_DETAILS, &[&payload.lobby_id, &uid])?
    .iter()
    .nth(0)
  {
    warn!(
      "unable to find lobby '{}' for user '{}'",
      payload.lobby_id, uid
    );
    return Ok(Response::not_found().cors(context.cors()));
  }

  debug!(
    "lobby exists and ready for new game, queuing job for lobby '{}'",
    payload.lobby_id
  );

  let job_id = context
    .jobs()
    .queue(&interchange::jobs::Job::CreateGame {
      creator: uid.clone(),
      lobby_id: payload.lobby_id.clone(),
      result: None,
    })
    .await?;

  Response::ok_json(interchange::http::JobHandle {
    id: job_id.clone(),
    result: None,
  })
  .map(|r| r.cors(context.cors()))
}
