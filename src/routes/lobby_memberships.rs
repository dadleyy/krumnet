use async_std::io::Read as AsyncRead;
use log::{debug, info, warn};
use serde::Deserialize;
use serde_json::from_slice as deserialize;
use std::io::Result;
use std::marker::Unpin;

use crate::{interchange, read_size_async, Authority, Context, Response};

const LEAVE_LOBBY: &'static str = include_str!("./data-store/leave-lobby-for-user.sql");

#[derive(Deserialize, Debug)]
pub struct DestroyMembershipPayload {
  lobby_id: String,
}

pub async fn destroy_membership<R>(context: &Context, reader: &mut R) -> Result<Response>
where
  R: AsyncRead + Unpin,
{
  let uid = match context.authority() {
    Authority::None => return Ok(Response::not_found().cors(context.cors())),
    Authority::User { id, .. } => id,
  };
  let contents = read_size_async(reader, context.pending()).await?;
  let payload = deserialize::<DestroyMembershipPayload>(&contents)?;
  debug!(
    "attempting to delete membership for user '{}', lobby '{}'",
    uid, payload.lobby_id
  );

  let rows = context
    .records()
    .query(LEAVE_LOBBY, &[&payload.lobby_id, &uid])?;

  let member_id = rows
    .iter()
    .nth(0)
    .and_then(|row| row.try_get::<_, String>(0).ok())
    .unwrap_or_default();

  if member_id.len() == 0 {
    warn!(
      "unable to find row to delete user[{}] lobby[{}]",
      uid, payload.lobby_id
    );
    return Ok(Response::not_found().cors(context.cors()));
  }

  info!("mebership '{}' is left", member_id);
  context
    .jobs()
    .queue(&interchange::jobs::Job::CleanupLobbyMembership {
      member_id,
      result: None,
    })
    .await?;

  Ok(Response::default().cors(context.cors()))
}
