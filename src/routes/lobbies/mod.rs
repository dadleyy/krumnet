use chrono::{DateTime, Utc};
use std::io::Result;
use std::marker::Unpin;

use async_std::io::Read;
use bit_vec::BitVec;
use log::{debug, info, warn};
use serde::Deserialize;
use serde_json::from_slice as deserialize;

use crate::{
  errors,
  http::{query_values, Uri},
  interchange, read_size_async,
  records::Row,
  Authority, Context, Response,
};

pub const LOAD_LOBBY_DETAILS: &'static str = include_str!("./data-store/load-lobby-detail.sql");
pub const LOAD_LOBBY_MEMBERS: &'static str = include_str!("./data-store/load-lobby-members.sql");
pub const LOBBY_FOR_USER: &'static str = include_str!("./data-store/lobbies-for-user.sql");
pub const GAMES_FOR_LOBBY: &'static str = include_str!("./data-store/load-lobby-games.sql");

#[derive(Deserialize, Debug)]
pub struct Payload {
  kind: String,
}

fn parse_member_row(row: &Row) -> Result<interchange::http::LobbyMember> {
  let member_id = row.try_get("member_id").map_err(errors::humanize_error)?;
  let user_id = row.try_get("user_id").map_err(errors::humanize_error)?;
  let name = row.try_get("user_name").map_err(errors::humanize_error)?;
  let invited_by = row.try_get("invited_by").map_err(errors::humanize_error)?;
  let joined_at = row.try_get("joined_at").map_err(errors::humanize_error)?;
  let left_at = row.try_get("left_at").map_err(errors::humanize_error)?;

  Ok(interchange::http::LobbyMember {
    member_id,
    user_id,
    name,
    invited_by,
    joined_at,
    left_at,
  })
}

pub fn load_games(context: &Context, id: &String) -> Result<Vec<interchange::http::LobbyGame>> {
  let rows = context.records().query(GAMES_FOR_LOBBY, &[id])?;
  debug!("found {} game rows", rows.len());
  rows
    .iter()
    .map(|row| {
      let id = row.try_get("game_id").map_err(errors::humanize_error)?;
      let created = row.try_get("created_at").map_err(errors::humanize_error)?;
      let ended = row.try_get("ended_at").map_err(errors::humanize_error)?;
      let name = row.try_get("game_name").map_err(errors::humanize_error)?;
      let rounds_remaining = row.try_get("round_count").map_err(errors::humanize_error)?;
      Ok(interchange::http::LobbyGame {
        id,
        created,
        ended,
        name,
        rounds_remaining,
      })
    })
    .collect()
}

pub fn load_members(context: &Context, id: &String) -> Result<Vec<interchange::http::LobbyMember>> {
  let rows = context.records().query(LOAD_LOBBY_MEMBERS, &[id])?;
  debug!("found {} member rows", rows.len());
  rows.iter().map(parse_member_row).collect()
}

pub async fn details(context: &Context, id: &String) -> Result<Response> {
  let uid = match context.authority() {
    Authority::User { id: s, token: _ } => s,
    _ => return Ok(Response::not_found().cors(context.cors())),
  };

  debug!("looking for loby via '{}' for user '{}'", id, uid);

  context
    .records()
    .query(LOAD_LOBBY_DETAILS, &[&id, &uid])?
    .iter()
    .nth(0)
    .map(|row| {
      let id = row
        .try_get::<_, String>(0)
        .map_err(errors::humanize_error)?;
      let name = row
        .try_get::<_, String>(1)
        .map_err(errors::humanize_error)?;
      let _settings = row
        .try_get::<_, BitVec>(2)
        .map_err(errors::humanize_error)?;
      let _created = row
        .try_get::<_, DateTime<Utc>>(3)
        .map_err(errors::humanize_error)?;

      let matches = row.try_get::<_, i64>(4).map_err(errors::humanize_error)?;

      if matches == 0 {
        debug!("user '{}' is not a member of lobby '{}'", uid, id);
        return Ok(Response::not_found().cors(context.cors()));
      }

      debug!("found match for lobby '{}' details, loading members", name);
      let members = load_members(&context, &id)?;
      let games = load_games(&context, &id)?;
      let details = interchange::http::LobbyDetails {
        id,
        name,
        members,
        games,
      };
      Ok(Response::ok_json(&details)?.cors(context.cors()))
    })
    .unwrap_or_else(|| Ok(Response::not_found().cors(context.cors())))
}

pub async fn find(context: &Context, uri: &Uri) -> Result<Response> {
  let uid = match context.authority() {
    Authority::User { id, .. } => id,
    Authority::None => return Ok(Response::not_found().cors(context.cors())),
  };

  let ids = query_values(uri, "ids[]");

  if ids.len() == 1 {
    let lobby_id = ids.into_iter().nth(0).unwrap_or_default();
    debug!("loading single lobby for user '{}'", uid);
    return details(context, &lobby_id).await;
  }

  debug!("loading lobbies for user '{}'", uid);

  let lobbies = context
    .records()
    .query(LOBBY_FOR_USER, &[&uid])?
    .iter()
    .filter_map(|row| {
      let id = row.try_get::<_, String>(0).ok()?;
      let name = row.try_get::<_, String>(1).ok()?;
      let created = row
        .try_get::<_, DateTime<Utc>>(2)
        .map_err(|e| {
          warn!("unable to parse game created - {}", e);
          e
        })
        .ok()?;

      let member_count = row
        .try_get::<_, i64>(3)
        .map_err(|e| {
          warn!("unable to parse member count column - {}", e);
          e
        })
        .ok()?;

      let game_count = row
        .try_get::<_, i64>(4)
        .map_err(|e| {
          warn!("unable to parse game count column - {}", e);
          e
        })
        .ok()?;

      debug!("found lobby '{}'", id);

      Some(interchange::http::LobbyListLobby {
        id,
        name,
        created,
        game_count,
        member_count,
      })
    })
    .collect();

  debug!("finished collecting lobbies - {:?}", lobbies);

  Response::ok_json(interchange::http::LobbyList { lobbies })
    .map(|response| response.cors(context.cors()))
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
