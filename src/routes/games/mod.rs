use async_std::io::Read as AsyncRead;
use chrono::{DateTime, Utc};
use log::{debug, info, warn};
use serde::Deserialize;
use serde_json::from_slice as deserialize;
use std::io::Result;
use std::marker::Unpin;

use crate::{
  errors,
  http::{query_values, Uri},
  interchange, read_size_async,
  routes::lobbies::LOAD_LOBBY_DETAILS,
  Authority, Context, Response,
};

const LOAD_GAME: &'static str = include_str!("data-store/load-game-details.sql");
const LOAD_MEMBERS: &'static str = include_str!("data-store/load-game-members.sql");
const LOAD_ROUNDS: &'static str = include_str!("data-store/load-rounds.sql");
const LOAD_PLACEMENTS: &'static str = include_str!("data-store/load-placements.sql");
const GAME_FOR_ENTRY: &'static str = include_str!("data-store/game-for-entry-creation.sql");
const CREATE_ENTRY: &'static str = include_str!("data-store/create-round-entry.sql");
const CREATE_VOTE: &'static str = include_str!("data-store/create-round-entry-vote.sql");

#[derive(Debug, Deserialize)]
struct EntryVotePayload {
  pub round_id: String,
  pub entry_id: String,
}

pub async fn create_entry_vote<R: AsyncRead + Unpin>(
  context: &Context,
  reader: &mut R,
) -> Result<Response> {
  let uid = match context.authority() {
    Authority::None => return Ok(Response::not_found().cors(context.cors())),
    Authority::User { id, .. } => id,
  };
  let contents = read_size_async(reader, context.pending()).await?;
  let payload = deserialize::<EntryVotePayload>(&contents)?;
  let authority = authority_for_round(context, &payload.round_id, &uid).await?;

  info!(
    "user {} voting for entry '{}' for round '{}'",
    authority.user_id, payload.entry_id, authority.round_id
  );

  let attempt = context
    .records()
    .query(CREATE_VOTE, &[&payload.entry_id, &authority.member_id])?
    .into_iter()
    .nth(0)
    .map(|row| {
      row
        .try_get::<_, String>("id")
        .map_err(errors::humanize_error)
    });

  let vote_id = match attempt {
    Some(e) => e?,
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

async fn authority_for_round(
  context: &Context,
  round_id: &String,
  user_id: &String,
) -> Result<RoundAuthority> {
  context
    .records()
    .query(GAME_FOR_ENTRY, &[round_id, user_id])?
    .into_iter()
    .nth(0)
    .map(|row| {
      let lobby_id = row.try_get("lobby_id").map_err(errors::humanize_error)?;
      let game_id = row.try_get("game_id").map_err(errors::humanize_error)?;
      let member_id = row.try_get("member_id").map_err(errors::humanize_error)?;
      let user_id = row.try_get("user_id").map_err(errors::humanize_error)?;
      let round_id = row.try_get("round_id").map_err(errors::humanize_error)?;
      Ok(RoundAuthority {
        lobby_id,
        game_id,
        member_id,
        user_id,
        round_id,
      })
    })
    .unwrap_or(Err(errors::e("unauthorized")))
}

#[derive(Debug, Deserialize)]
struct EntryPayload {
  pub round_id: String,
  pub entry: String,
}

pub async fn create_entry<R: AsyncRead + Unpin>(
  context: &Context,
  reader: &mut R,
) -> Result<Response> {
  let uid = match context.authority() {
    Authority::None => return Ok(Response::not_found().cors(context.cors())),
    Authority::User { id, .. } => id,
  };

  let contents = read_size_async(reader, context.pending()).await?;
  let payload = deserialize::<EntryPayload>(&contents)?;
  let authority = authority_for_round(context, &payload.round_id, &uid).await?;

  let created = context
    .records()
    .query(
      CREATE_ENTRY,
      &[
        &authority.round_id,
        &authority.member_id,
        &payload.entry,
        &authority.game_id,
        &authority.lobby_id,
        &authority.user_id,
      ],
    )
    .map_err(log_err)?
    .iter()
    .nth(0)
    .and_then(|row| {
      let entry_id = row.try_get::<_, String>(0).map_err(log_err).ok()?;
      let entry = row.try_get::<_, String>(1).map_err(log_err).ok()?;
      let round_id = row.try_get::<_, String>(2).map_err(log_err).ok()?;
      Some((entry_id, entry, round_id))
    });

  debug!("creating round entry for user '{}' - {:?}", uid, created);

  match created {
    Some((_entry_id, entry, round_id)) => {
      debug!("successfully created entry - {:?}", entry);

      context
        .jobs()
        .queue(&interchange::jobs::Job::CheckRoundFulfillment(
          interchange::jobs::CheckRoundFulfillment {
            round_id,
            result: None,
          },
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

fn members_for_game(context: &Context, id: &String) -> Result<Vec<interchange::http::GameMember>> {
  context
    .records()
    .query(LOAD_MEMBERS, &[&id])?
    .iter()
    .map(|r| {
      let member_id = r.try_get("member_id").map_err(errors::humanize_error)?;
      let joined = r.try_get("created_at").map_err(errors::humanize_error)?;
      let user_id = r.try_get("user_id").map_err(errors::humanize_error)?;
      let email = r.try_get("user_email").map_err(errors::humanize_error)?;
      let name = r.try_get("user_name").map_err(errors::humanize_error)?;

      debug!("found member '{}'", id);

      Ok(interchange::http::GameMember {
        member_id,
        user_id,
        email,
        name,
        joined,
      })
    })
    .collect()
}

fn rounds_for_game(context: &Context, id: &String) -> Result<Vec<interchange::http::GameRound>> {
  context
    .records()
    .query(LOAD_ROUNDS, &[&id])?
    .iter()
    .map(|row| {
      let id = row.try_get("id").map_err(errors::humanize_error)?;
      let position = row
        .try_get::<_, i32>("pos")
        .map_err(errors::humanize_error)? as u32;
      let prompt = row.try_get("prompt").map_err(errors::humanize_error)?;
      let created = row.try_get("created_at").map_err(errors::humanize_error)?;
      let started = row.try_get("started_at").map_err(errors::humanize_error)?;
      let completed = row
        .try_get("completed_at")
        .map_err(errors::humanize_error)?;
      let fulfilled = row
        .try_get("fulfilled_at")
        .map_err(errors::humanize_error)?;

      debug!("found round '{}' ({:?}, {:?})", id, position, completed);

      Ok(interchange::http::GameRound {
        id,
        position,
        prompt,
        created,
        started,
        fulfilled,
        completed,
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

fn placements_for_game(
  context: &Context,
  game_id: &String,
) -> Result<Vec<interchange::http::GameDetailPlacement>> {
  context
    .records()
    .query(LOAD_PLACEMENTS, &[game_id])?
    .into_iter()
    .map(|row| {
      let id = row.try_get("id").map_err(errors::humanize_error)?;
      let user_name = row.try_get("user_name").map_err(errors::humanize_error)?;
      let user_id = row.try_get("user_id").map_err(errors::humanize_error)?;
      let place = row.try_get("placement").map_err(errors::humanize_error)?;
      debug!("found placement - '{}'", id);
      Ok(interchange::http::GameDetailPlacement {
        id,
        user_name,
        user_id,
        place,
      })
    })
    .collect()
}

async fn find_game(context: &Context, uid: &String, gid: &String) -> Result<Response> {
  let details = context
    .records()
    .query(LOAD_GAME, &[gid, uid])?
    .iter()
    .nth(0)
    .map(|row| {
      let game_id = row.try_get("game_id").map_err(errors::humanize_error)?;
      let created_at = row.try_get("created_at").map_err(errors::humanize_error)?;
      let name = row.try_get("game_name").map_err(errors::humanize_error)?;
      let ended_at = row.try_get("ended_at").map_err(errors::humanize_error)?;
      Ok(GameDetails {
        game_id,
        created_at,
        name,
        ended_at,
      })
    })
    .unwrap_or(Err(errors::e("Unable to find game")))?;

  debug!(
    "found game '{}', created '{:?}'",
    details.game_id, details.created_at
  );

  let rounds = rounds_for_game(context, &details.game_id).map_err(log_err)?;
  let members = members_for_game(context, &details.game_id).map_err(log_err)?;
  let placements = placements_for_game(context, &details.game_id).map_err(log_err)?;

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

pub async fn find(context: &Context, uri: &Uri) -> Result<Response> {
  let uid = match context.authority() {
    Authority::User { id, .. } => id,
    Authority::None => return Ok(Response::not_found().cors(context.cors())),
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

pub async fn create<R>(context: &Context, reader: &mut R) -> Result<Response>
where
  R: AsyncRead + Unpin,
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

  let details = interchange::jobs::CreateGame {
    creator: uid.clone(),
    lobby_id: payload.lobby_id.clone(),
    result: None,
  };
  let job_id = context
    .jobs()
    .queue(&interchange::jobs::Job::CreateGame(details))
    .await?;

  Response::ok_json(interchange::http::JobHandle {
    id: job_id.clone(),
    result: None,
  })
  .map(|r| r.cors(context.cors()))
}
