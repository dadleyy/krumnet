use std::io::Result;
use std::marker::Unpin;
use std::time::SystemTime;

use async_std::io::Read;
use bit_vec::BitVec;
use log::{debug, info};
use serde::Deserialize;
use serde_json::from_slice as deserialize;

use crate::{
  errors, http::Uri, interchange, read_size_async, records::Row, Authority, Context, Response,
};

pub const LOAD_LOBBY_DETAILS: &'static str = include_str!("./data-store/load-lobby-detail.sql");
pub const LOAD_LOBBY_MEMBERS: &'static str = include_str!("./data-store/load-lobby-members.sql");

#[derive(Deserialize, Debug)]
pub struct Payload {
  kind: String,
}

fn parse_member_row(row: &Row) -> Option<interchange::http::LobbyMember> {
  let member_id = row.try_get::<_, String>(0).ok()?;
  let user_id = row.try_get::<_, String>(1).ok()?;
  let email = row.try_get::<_, String>(2).ok()?;
  let name = row.try_get::<_, String>(3).ok()?;
  let invited_by = row.try_get::<_, Option<String>>(4).ok()?;
  let joined_at = row.try_get::<_, Option<SystemTime>>(5).ok()?;
  let left_at = row.try_get::<_, Option<SystemTime>>(6).ok()?;

  Some(interchange::http::LobbyMember {
    member_id,
    user_id,
    name,
    email,
    invited_by,
    joined_at,
    left_at,
  })
}

pub fn load_members(context: &Context, id: &String) -> Result<Vec<interchange::http::LobbyMember>> {
  let rows = context.records().query(LOAD_LOBBY_MEMBERS, &[id])?;
  debug!("found {} rows", rows.len());
  Ok(rows.iter().filter_map(parse_member_row).collect())
}

pub async fn details(context: &Context, uri: &Uri) -> Result<Response> {
  let uid = match context.authority() {
    Authority::User { id: s, token: _ } => s,
    _ => return Ok(Response::not_found().cors(context.cors())),
  };
  let id = uri.path().trim_start_matches("/lobbies/");

  debug!("looking for loby via '{}' for user '{}'", id, uid);

  context
    .records()
    .query(LOAD_LOBBY_DETAILS, &[&id, &uid])?
    .iter()
    .nth(0)
    .map(|r| {
      let id = r.try_get::<_, String>(0).map_err(errors::humanize_error)?;
      let name = r.try_get::<_, String>(1).map_err(errors::humanize_error)?;
      let _settings = r.try_get::<_, BitVec>(2).map_err(errors::humanize_error)?;
      let _created = r
        .try_get::<_, SystemTime>(3)
        .map_err(errors::humanize_error)?;

      let matches = r.try_get::<_, i64>(4).map_err(errors::humanize_error)?;

      if matches == 0 {
        debug!("user '{}' is not a member of lobby '{}'", uid, id);
        return Ok(Response::not_found().cors(context.cors()));
      }

      debug!("found match for lobby '{}' details, loading members", name);
      let members = load_members(&context, &id)?;
      let details = interchange::http::LobbyDetails { id, name, members };
      Ok(Response::ok_json(&details)?.cors(context.cors()))
    })
    .unwrap_or_else(|| Ok(Response::not_found().cors(context.cors())))
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

  let job_id = context
    .jobs()
    .queue(&interchange::jobs::Job::CreateLobby {
      creator: uid.clone(),
      result: None,
    })
    .await?;

  let payload = deserialize::<Payload>(&contents)?;
  debug!("buffer after read {:?}", payload);

  Response::ok_json(interchange::http::JobHandle {
    id: job_id.clone(),
    result: None,
  })
  .map(|r| r.cors(context.cors()))
}
