use async_std::io::Read as AsyncRead;
use log::{debug, info, warn};
use serde::Deserialize;
use serde_json::from_slice as deserialize;
use std::io::Result;
use std::marker::Unpin;

use crate::{errors, interchange, read_size_async, Authority, Context, Response};

const LEAVE_LOBBY: &'static str = include_str!("./data-store/leave-lobby-for-user.sql");
const JOIN_LOBBY: &'static str = include_str!("./data-store/join-lobby.sql");

#[derive(Deserialize, Debug)]
pub struct DestroyMembershipPayload {
  lobby_id: String,
}

pub async fn create_membership<R>(context: &Context, reader: &mut R) -> Result<Response>
where
  R: AsyncRead + Unpin,
{
  let uid = match context.authority() {
    Authority::None => return Ok(Response::not_found().cors(context.cors())),
    Authority::User { id, .. } => id,
  };
  let contents = read_size_async(reader, context.pending()).await?;
  let payload = deserialize::<DestroyMembershipPayload>(&contents)?;
  let rows = context
    .records()
    .query(JOIN_LOBBY, &[&payload.lobby_id, &uid])?;

  let row: Option<Result<(String, String, String)>> = rows.iter().nth(0).map(|row| {
    let member_id = row
      .try_get::<_, String>(0)
      .map_err(errors::humanize_error)?;
    let lobby_id = row
      .try_get::<_, String>(1)
      .map_err(errors::humanize_error)?;
    let user_id = row
      .try_get::<_, String>(2)
      .map_err(errors::humanize_error)?;
    Ok((member_id, lobby_id, user_id))
  });

  match row {
    Some(Ok((member_id, lobby_id, user_id))) => {
      info!(
        "user {} is now member {} of lobby {}",
        user_id, member_id, lobby_id
      );
      let out = interchange::http::NewLobbyMembership {
        member_id,
        user_id,
        lobby_id,
      };
      Response::ok_json(&out).map(|r| r.cors(context.cors()))
    }
    Some(Err(e)) => {
      warn!(
        "user {} attempted to join lobby {}, unable to insert new row - {}",
        uid, payload.lobby_id, e
      );
      Ok(Response::failed().cors(context.cors()))
    }
    None => {
      warn!(
        "user {} attempted to join lobby {}, unable to insert new row",
        uid, payload.lobby_id
      );
      Ok(Response::not_found().cors(context.cors()))
    }
  }
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

  let (member_id, lobby_id) = rows
    .iter()
    .nth(0)
    .and_then(|row| {
      Some((
        row.try_get::<_, String>(0).ok()?,
        row.try_get::<_, String>(1).ok()?,
      ))
    })
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
      lobby_id,
      result: None,
    })
    .await?;

  Ok(Response::default().cors(context.cors()))
}
