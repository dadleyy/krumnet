use async_std::io::Read as AsyncRead;
use log::{debug, info, warn};
use serde::Deserialize;
use serde_json::from_slice as deserialize;
use sqlx::query_file;
use std::io::Result;
use std::marker::Unpin;

use crate::{constants, errors, interchange, read_size_async, Authority, Context, Response};

const TOO_MANY_MEMBERS: &'static str = "errors.lobbies.too_many_members";

#[derive(Deserialize, Debug)]
pub struct DestroyMembershipPayload {
  lobby_id: String,
}

async fn join_jobby(context: &Context, lobby_id: &String, user_id: &String) -> Result<(String, String, String)> {
  let mut conn = context.records_connection().await?;
  query_file!(
    "src/routes/lobby_memberships/data-store/join-lobby.sql",
    lobby_id,
    user_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(errors::humanize_error)?
  .into_iter()
  .nth(0)
  .map(|row| (row.member_id, row.lobby_id, row.user_id))
  .ok_or_else(|| errors::e("Unable to join lobby"))
}

// Route
// POST /lobby-memberships
pub async fn create_membership<R>(context: &Context, reader: &mut R) -> Result<Response>
where
  R: AsyncRead + Unpin,
{
  let uid = match context.authority() {
    Authority::None => return Ok(Response::unauthorized().cors(context.cors())),
    Authority::User { id, .. } => id,
  };

  let contents = read_size_async(reader, context.pending()).await?;
  let payload = deserialize::<DestroyMembershipPayload>(&contents)?;

  let mut conn = context.records_connection().await?;
  let member_count = query_file!(
    "src/routes/lobby_memberships/data-store/count-members.sql",
    payload.lobby_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(errors::humanize_error)?
  .into_iter()
  .nth(0)
  .and_then(|row| row.member_count);

  match member_count {
    None => {
      warn!("unable to find lobby '{}' to join", payload.lobby_id);
      return Ok(Response::not_found().cors(context.cors()));
    }
    Some(value) if value >= constants::MAX_LOBBY_MEMBERS.into() => {
      warn!("too many members in '{}' to join", payload.lobby_id);
      return Ok(Response::bad_request(TOO_MANY_MEMBERS).cors(context.cors()));
    }
    Some(value) => info!("member count for '{}' satisfactory ({})", payload.lobby_id, value),
  };

  let (member_id, lobby_id, user_id) = join_jobby(context, &payload.lobby_id, &uid).await?;

  info!("user {} is now member {} of lobby {}", user_id, member_id, lobby_id);
  let out = interchange::http::NewLobbyMembership {
    member_id,
    user_id,
    lobby_id,
  };
  Response::ok_json(&out).map(|r| r.cors(context.cors()))
}

async fn leave_lobby(context: &Context, lobby_id: &String, user_id: &String) -> Result<(String, String)> {
  let mut conn = context.records_connection().await?;
  query_file!(
    "src/routes/lobby_memberships/data-store/leave-lobby-for-user.sql",
    lobby_id,
    user_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(errors::humanize_error)?
  .into_iter()
  .nth(0)
  .map(|row| (row.member_id, row.lobby_id))
  .ok_or_else(|| errors::e("Unable to leave lobby"))
}

// Route
// DELETE /lobby-memberships
pub async fn destroy_membership<R>(context: &Context, reader: &mut R) -> Result<Response>
where
  R: AsyncRead + Unpin,
{
  let uid = match context.authority() {
    Authority::None => return Ok(Response::unauthorized().cors(context.cors())),
    Authority::User { id, .. } => id,
  };

  let contents = read_size_async(reader, context.pending()).await?;
  let payload = deserialize::<DestroyMembershipPayload>(&contents)?;

  debug!(
    "attempting to delete membership for user '{}', lobby '{}'",
    uid, payload.lobby_id
  );

  let (member_id, lobby_id) = leave_lobby(context, &payload.lobby_id, &uid).await?;

  if member_id.len() == 0 {
    warn!("unable to find row to delete user[{}] lobby[{}]", uid, payload.lobby_id);
    return Ok(Response::not_found().cors(context.cors()));
  }

  info!("marking membership '{}' as left", member_id);
  let details = interchange::jobs::CleanupLobbyMembership {
    member_id,
    lobby_id,
    result: None,
  };

  context
    .jobs()
    .queue(&interchange::jobs::Job::CleanupLobbyMembership(details))
    .await?;

  Ok(Response::default().cors(context.cors()))
}
