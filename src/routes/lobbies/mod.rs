use chrono::{DateTime, Utc};
use std::io::Result;
use std::marker::Unpin;

use async_std::io::Read;
use log::{debug, info};
use serde::Deserialize;
use serde_json::from_slice as deserialize;
use sqlx::query_file;

use crate::{
  errors,
  http::{query_values, Uri},
  interchange, read_size_async, Authority, Context, Response,
};

#[derive(Deserialize, Debug)]
pub struct Payload {
  kind: String,
}

async fn load_games(context: &Context, id: &String) -> Result<Vec<interchange::http::LobbyGame>> {
  let mut conn = context.records_connection().await?;

  query_file!("src/routes/lobbies/data-store/load-lobby-games.sql", id)
    .fetch_all(&mut conn)
    .await
    .map_err(errors::humanize_error)?
    .into_iter()
    .map(|row| {
      Ok::<_, std::io::Error>(interchange::http::LobbyGame {
        id: row.game_id,
        created: row
          .created_at
          .ok_or(errors::e("Unable to parse game created at timestamp"))?,
        ended: row.ended_at,
        name: row.game_name,
        rounds_remaining: row.round_count.ok_or(errors::e("Unable to parse game round count"))?,
      })
    })
    .collect()
}

struct LobbyDetailRow {
  pub id: String,
  pub name: String,
  pub created: DateTime<Utc>,
}

async fn lobby_details_for_user(
  context: &Context,
  lobby_id: &String,
  user_id: &String,
) -> Result<Option<LobbyDetailRow>> {
  let mut conn = context.records_connection().await?;
  let details = query_file!("src/routes/lobbies/data-store/load-lobby-detail.sql", lobby_id, user_id)
    .fetch_all(&mut conn)
    .await
    .map_err(errors::humanize_error)?
    .into_iter()
    .nth(0)
    .and_then(|row| {
      Some(LobbyDetailRow {
        id: row.lobby_id,
        name: row.lobby_name,
        created: row.created_at?,
      })
    });

  Ok(details)
}

async fn load_members(context: &Context, id: &String) -> Result<Vec<interchange::http::LobbyMember>> {
  let mut conn = context.records_connection().await?;
  query_file!("src/routes/lobbies/data-store/load-lobby-members.sql", id)
    .fetch_all(&mut conn)
    .await
    .map_err(errors::humanize_error)?
    .into_iter()
    .map(|row| {
      Ok(interchange::http::LobbyMember {
        member_id: row.member_id,
        user_id: row.user_id,
        name: row.user_name,
        invited_by: row.invited_by,
        joined_at: row.joined_at,
        left_at: row.left_at,
      })
    })
    .collect()
}

pub async fn details(context: &Context, id: &String) -> Result<Response> {
  let uid = match context.authority() {
    Authority::User { id: s, token: _ } => s,
    _ => return Ok(Response::unauthorized().cors(context.cors())),
  };

  debug!("looking for loby via '{}' for user '{}'", id, uid);
  let deets = match lobby_details_for_user(context, id, uid).await? {
    Some(details) => details,
    None => return Ok(Response::not_found().cors(context.cors())),
  };

  debug!("found lobby '{}' details, loading members", deets.name);
  let members = load_members(&context, &id).await?;
  let games = load_games(&context, &id).await?;
  let details = interchange::http::LobbyDetails {
    id: deets.id,
    name: deets.name,
    members,
    games,
  };
  Ok(Response::ok_json(&details)?.cors(context.cors()))
}

// Route
// GET /lobbies
pub async fn find(context: &Context, uri: &Uri) -> Result<Response> {
  let uid = match context.authority() {
    Authority::User { id, .. } => id,
    Authority::None => return Ok(Response::unauthorized().cors(context.cors())),
  };

  let ids = query_values(uri, "ids[]");

  if ids.len() == 1 {
    let lobby_id = ids.into_iter().nth(0).unwrap_or_default();
    debug!("loading single lobby for user '{}'", uid);
    return details(context, &lobby_id).await;
  }

  debug!("loading lobbies for user '{}'", uid);
  let mut conn = context.records_connection().await?;

  query_file!("src/routes/lobbies/data-store/lobbies-for-user.sql", uid)
    .fetch_all(&mut conn)
    .await
    .map_err(errors::humanize_error)?
    .into_iter()
    .map(|row| {
      Ok::<_, std::io::Error>(interchange::http::LobbyListLobby {
        id: row.lobby_id,
        name: row.lobby_name,
        created: row
          .created_at
          .ok_or_else(|| errors::e("Unable to parse created_at for lobby"))?,
        game_count: row
          .game_count
          .ok_or_else(|| errors::e("Unable to parse game count for lobby"))?,
        member_count: row
          .member_count
          .ok_or_else(|| errors::e("Unable to parse member count for lobby"))?,
      })
    })
    .collect::<Result<Vec<interchange::http::LobbyListLobby>>>()
    .and_then(|lobbies| Response::ok_json(interchange::http::LobbyList { lobbies }))
    .map(|response| response.cors(context.cors()))
}

// Route
// POST /lobbies
pub async fn create<R>(context: &Context, reader: &mut R) -> Result<Response>
where
  R: Read + Unpin,
{
  let uid = match context.authority() {
    Authority::User { id: s, token: _ } => s,
    _ => return Ok(Response::unauthorized().cors(context.cors())),
  };

  // TODO - does this need to be something?
  let contents = read_size_async(reader, context.pending()).await?;
  deserialize::<Payload>(&contents)?;

  info!("new lobby for user '{}'", uid);

  let job_id = context
    .jobs()
    .queue(&interchange::jobs::Job::CreateLobby(interchange::jobs::CreateLobby {
      creator: uid.clone(),
      result: None,
    }))
    .await?;

  Response::ok_json(interchange::http::JobHandle {
    id: job_id.clone(),
    result: None,
  })
  .map(|r| r.cors(context.cors()))
}
