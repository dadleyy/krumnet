use async_std::io::Read as AsyncRead;
use chrono::{DateTime, Utc};
use log::{debug, info, warn};
use serde::Deserialize;
use serde_json::from_slice as deserialize;
use sqlx::query_file;
use std::io::Result;
use std::marker::Unpin;

use crate::{
  errors,
  http::{query_values, Uri},
  interchange, read_size_async, Authority, Context, Response,
};

const NOT_ENOUGH_MEMBERS: &'static str = "errors.games.not_enough_members";
const INVALID_LOBBY: &'static str = "errors.games.invalid_lobby";

#[derive(Debug, Deserialize)]
struct EntryVotePayload {
  pub round_id: String,
  pub entry_id: String,
}

// Route
// POST /round-entry-votes
pub async fn create_entry_vote<R: AsyncRead + Unpin>(context: &Context, reader: &mut R) -> Result<Response> {
  let uid = match context.authority() {
    Authority::None => return Ok(Response::unauthorized().cors(context.cors())),
    Authority::User { id, .. } => id,
  };
  let contents = read_size_async(reader, context.pending()).await?;
  let payload = deserialize::<EntryVotePayload>(&contents)?;

  let authority = match authority_for_round(context, &payload.round_id, &uid).await? {
    Some(auth) => auth,
    None => {
      warn!("unauthorized vote by user '{}'", uid);
      return Ok(Response::unauthorized().cors(context.cors()));
    }
  };

  info!(
    "user {} voting for entry '{}' for round '{}'",
    authority.user_id, payload.entry_id, authority.round_id
  );

  let mut conn = context.records_connection().await?;

  let attempt = query_file!(
    "src/routes/games/data-store/create-round-entry-vote.sql",
    payload.entry_id,
    authority.member_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(errors::humanize_error)?
  .into_iter()
  .nth(0)
  .map(|row| row.id);

  let vote_id = match attempt {
    Some(e) => e,
    None => return Ok(Response::not_found().cors(context.cors())),
  };

  info!("vote creation attempt: {:?}, queing job", vote_id);
  let job_context = interchange::jobs::CheckRoundCompletion {
    round_id: authority.round_id.clone(),
    game_id: authority.game_id.clone(),
    result: None,
  };
  context
    .jobs()
    .queue(&interchange::jobs::Job::CheckRoundCompletion(job_context))
    .await?;

  return Ok(Response::default().cors(context.cors()));
}

struct RoundAuthority {
  lobby_id: String,
  game_id: String,
  member_id: String,
  user_id: String,
  round_id: String,
}

async fn authority_for_round(context: &Context, round_id: &String, user_id: &String) -> Result<Option<RoundAuthority>> {
  let mut conn = context.records_connection().await?;
  let possible = query_file!(
    "src/routes/games/data-store/game-for-entry-creation.sql",
    round_id,
    user_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(errors::humanize_error)?
  .into_iter()
  .nth(0)
  .map(|row| RoundAuthority {
    lobby_id: row.lobby_id,
    game_id: row.game_id,
    member_id: row.member_id,
    user_id: row.user_id,
    round_id: row.round_id,
  });
  Ok(possible)
}

#[derive(Debug, Deserialize)]
struct EntryPayload {
  pub round_id: String,
  pub entry: String,
}

// Route
// POST /round-entries
pub async fn create_entry<R: AsyncRead + Unpin>(context: &Context, reader: &mut R) -> Result<Response> {
  let uid = match context.authority() {
    Authority::None => return Ok(Response::unauthorized().cors(context.cors())),
    Authority::User { id, .. } => id,
  };

  let contents = read_size_async(reader, context.pending()).await?;
  let payload = deserialize::<EntryPayload>(&contents)?;

  let authority = match authority_for_round(context, &payload.round_id, &uid).await? {
    Some(auth) => auth,
    None => {
      warn!("unauthorized attempt to create entry by user '{}'", uid);
      return Ok(Response::unauthorized().cors(context.cors()));
    }
  };

  let mut conn = context.records_connection().await?;
  let created = query_file!(
    "src/routes/games/data-store/create-round-entry.sql",
    authority.round_id,
    authority.member_id,
    payload.entry,
    authority.game_id,
    authority.lobby_id,
    authority.user_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(errors::humanize_error)?
  .into_iter()
  .nth(0)
  .and_then(|row| Some((row.entry_id, row.entry, row.round_id)));

  debug!("creating round entry for user '{}' - {:?}", uid, created);

  match created {
    Some((_entry_id, entry, round_id)) => {
      debug!("successfully created entry - {:?}", entry);

      context
        .jobs()
        .queue(&interchange::jobs::Job::CheckRoundFulfillment(
          interchange::jobs::CheckRoundFulfillment { round_id, result: None },
        ))
        .await
        .map(|_id| Response::default().cors(context.cors()))
        .or_else(|e| {
          log_err(e);
          Ok(Response::default().cors(context.cors()))
        })
    }
    None => {
      warn!("round entry creation did not return information from inserted entry");
      return Ok(Response::default().cors(context.cors()));
    }
  }
}

#[derive(Deserialize)]
pub struct CreatePayload {
  pub lobby_id: String,
}

fn log_err<E: std::error::Error>(error: E) -> E {
  warn!("error - {}", error);
  error
}

async fn members_for_game(context: &Context, id: &String) -> Result<Vec<interchange::http::GameMember>> {
  let mut conn = context.records_connection().await?;

  query_file!("src/routes/games/data-store/load-game-members.sql", id)
    .fetch_all(&mut conn)
    .await
    .map_err(errors::humanize_error)?
    .into_iter()
    .map(|row| {
      Ok(interchange::http::GameMember {
        member_id: row.member_id,
        user_id: row.user_id,
        name: row.user_name,
        joined: row
          .created_at
          .ok_or_else(|| errors::e(format!("Unable to parse game member created timestamp")))?,
      })
    })
    .collect()
}

async fn rounds_for_game(context: &Context, id: &String) -> Result<Vec<interchange::http::GameRound>> {
  let mut conn = context.records_connection().await?;
  query_file!("src/routes/games/data-store/load-rounds.sql", id)
    .fetch_all(&mut conn)
    .await
    .map_err(errors::humanize_error)?
    .into_iter()
    .map(|row| {
      Ok(interchange::http::GameRound {
        id: row.id,
        position: row.pos,
        prompt: row.prompt,
        created: row
          .created_at
          .ok_or_else(|| errors::e("Unable to parse round created at timestamp"))?,
        started: row.started_at,
        fulfilled: row.fulfilled_at,
        completed: row.completed_at,
      })
    })
    .collect()
}

struct GameDetails {
  pub game_id: String,
  pub created_at: DateTime<Utc>,
  pub name: String,
  pub ended_at: Option<DateTime<Utc>>,
}

async fn placements_for_game(
  context: &Context,
  game_id: &String,
) -> Result<Vec<interchange::http::GameDetailPlacement>> {
  let mut conn = context.records_connection().await?;
  query_file!("src/routes/games/data-store/load-placements.sql", game_id)
    .fetch_all(&mut conn)
    .await
    .map_err(errors::humanize_error)?
    .into_iter()
    .map(|row| {
      Ok(interchange::http::GameDetailPlacement {
        id: row.id,
        user_name: row.user_name,
        user_id: row.user_id,
        place: row.placement,
        vote_count: row.vote_count,
      })
    })
    .collect()
}

async fn find_game(context: &Context, uid: &String, gid: &String) -> Result<Response> {
  let mut conn = context.records_connection().await?;
  let details = query_file!("src/routes/games/data-store/load-game-details.sql", gid, uid)
    .fetch_all(&mut conn)
    .await
    .map_err(errors::humanize_error)?
    .into_iter()
    .nth(0)
    .map(|row| {
      Ok(GameDetails {
        game_id: row.game_id,
        created_at: row
          .created_at
          .ok_or_else(|| errors::e(format!("Unable to parse created timestamp for game '{}'", gid)))?,
        name: row.game_name,
        ended_at: row.ended_at,
      })
    })
    .unwrap_or_else(|| Err(errors::e(format!("Unable to find game '{}'", gid))))?;

  debug!("found game '{}', created '{:?}'", details.game_id, details.created_at);

  let rounds = rounds_for_game(context, &details.game_id).await.map_err(log_err)?;
  let members = members_for_game(context, &details.game_id).await.map_err(log_err)?;
  let placements = placements_for_game(context, &details.game_id).await.map_err(log_err)?;

  let result = interchange::http::GameDetails {
    id: details.game_id.clone(),
    created: details.created_at.clone(),
    name: details.name.clone(),
    ended: details.ended_at.clone(),
    members,
    rounds,
    placements,
  };

  Response::ok_json(&result).map(|r| r.cors(context.cors()))
}

// Route
// GET /games
pub async fn find(context: &Context, uri: &Uri) -> Result<Response> {
  let uid = match context.authority() {
    Authority::User { id, .. } => id,
    Authority::None => return Ok(Response::unauthorized().cors(context.cors())),
  };

  let ids = query_values(uri, "ids[]");

  if ids.len() != 1 {
    debug!("find all games not implemented yet");
    return Ok(Response::not_found().cors(context.cors()));
  }

  let gid = ids.iter().nth(0).ok_or(errors::e("Invalid id"))?;
  debug!("attempting to find game from single id - {:?}", gid);
  find_game(context, uid, gid).await
}

// Route
// POST /games
pub async fn create<R>(context: &Context, reader: &mut R) -> Result<Response>
where
  R: AsyncRead + Unpin,
{
  let uid = match context.authority() {
    Authority::None => return Ok(Response::unauthorized().cors(context.cors())),
    Authority::User { id, .. } => id,
  };

  debug!("creating new game for user - {}", uid);

  let contents = read_size_async(reader, context.pending()).await?;
  let CreatePayload { lobby_id } = deserialize::<CreatePayload>(&contents)?;

  let mut conn = context.records_connection().await?;
  let maybe_lobby = query_file!("src/routes/lobbies/data-store/load-lobby-detail.sql", lobby_id, uid)
    .fetch_all(&mut conn)
    .await
    .map_err(errors::humanize_error)?
    .into_iter()
    .nth(0);

  if let None = maybe_lobby {
    warn!("no lobby '{}' for user '{}'", lobby_id, uid);
    return Ok(Response::bad_request(INVALID_LOBBY).cors(context.cors()));
  }

  let member_count = query_file!("src/routes/games/data-store/count-lobby-members.sql", lobby_id)
    .fetch_all(&mut conn)
    .await
    .map_err(errors::humanize_error)?
    .into_iter()
    .nth(0)
    .and_then(|row| row.member_count)
    .unwrap_or(0);

  if let 0..=1 = member_count {
    warn!("not enough members for '{}'", lobby_id);
    return Ok(Response::bad_request(NOT_ENOUGH_MEMBERS).cors(context.cors()));
  }

  info!("queuing new game job for lobby '{}'", lobby_id);

  let details = interchange::jobs::CreateGame {
    creator: uid.clone(),
    lobby_id: lobby_id.clone(),
    result: None,
  };

  context
    .jobs()
    .queue(&interchange::jobs::Job::CreateGame(details))
    .await
    .map(|job_id| interchange::http::JobHandle {
      id: job_id.clone(),
      result: None,
    })
    .and_then(|payload| Response::ok_json(payload))
    .map(|response| response.cors(context.cors()))
}
