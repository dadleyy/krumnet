use async_std::io::Read as AsyncRead;
use chrono::{DateTime, Utc};
use log::{debug, warn};
use serde::Deserialize;
use serde_json::from_slice as deserialize;
use std::io::Result;
use std::marker::Unpin;

use super::lobbies::LOAD_LOBBY_DETAILS;
use crate::{
  errors,
  http::{query_values, Uri},
  interchange, read_size_async, Authority, Context, Response,
};

const LOAD_GAME: &'static str = include_str!("data-store/load-game-details.sql");
const LOAD_MEMBERS: &'static str = include_str!("data-store/load-game-members.sql");
const LOAD_ROUNDS: &'static str = include_str!("data-store/load-rounds.sql");
const LOAD_ENTRIES: &'static str = include_str!("data-store/load-round-entries.sql");
const LOAD_ROUND_DETAILS: &'static str = include_str!("data-store/load-round-details.sql");
const GAME_FOR_ENTRY: &'static str = include_str!("data-store/game-for-entry-creation.sql");
const CREATE_ENTRY: &'static str = include_str!("data-store/create-job-entry.sql");

#[derive(Deserialize)]
pub struct CreatePayload {
  pub lobby_id: String,
}

fn log_err<E: std::error::Error>(e: E) -> E {
  warn!("error - {}", e);
  e
}

pub async fn find_game(context: &Context, uid: &String, gid: &String) -> Result<Response> {
  let (id, created, name) = match context
    .records()
    .query(LOAD_GAME, &[gid, uid])?
    .iter()
    .nth(0)
    .and_then(|r| {
      let id = r.try_get::<_, String>(0).ok()?;
      let created = r
        .try_get::<_, DateTime<Utc>>(1)
        .map_err(|e| {
          warn!("unable to parse time value as datetime - {}", e);
          errors::e("bad date time")
        })
        .ok()?;
      let name = r.try_get::<_, String>(2).ok()?;

      Some((id, created, name))
    }) {
    Some(contents) => contents,
    None => return Ok(Response::not_found().cors(context.cors())),
  };

  debug!("found game '{}', created '{:?}'", id, created);

  let rounds = context
    .records()
    .query(LOAD_ROUNDS, &[&id])?
    .iter()
    .filter_map(|row| {
      let id = row.try_get(0).map_err(log_err).ok()?;
      let position = row.try_get::<_, i32>(1).map_err(log_err).ok()? as u32;
      let prompt = row.try_get(2).map_err(log_err).ok()?;
      let created = row.try_get(3).map_err(log_err).ok()?;
      let started = row.try_get(4).map_err(log_err).ok()?;
      let completed = row.try_get(5).map_err(log_err).ok()?;

      debug!("found round '{}' ({:?}, {:?})", id, position, completed);

      Some(interchange::http::GameRound {
        id,
        position,
        prompt,
        created,
        started,
        completed,
      })
    })
    .collect();

  let members = context
    .records()
    .query(LOAD_MEMBERS, &[&id])?
    .iter()
    .filter_map(|r| {
      let member_id = r.try_get::<_, String>(0).ok()?;
      let joined = r.try_get::<_, DateTime<Utc>>(2).ok()?;
      let user_id = r.try_get::<_, String>(3).ok()?;
      let email = r.try_get::<_, String>(4).ok()?;
      let name = r.try_get::<_, String>(5).ok()?;
      debug!("found member '{}'", id);
      Some(interchange::http::GameMember {
        member_id,
        user_id,
        email,
        name,
        joined,
      })
    })
    .collect();

  debug!("found members[{:?}] rounds[{:?}]", members, &rounds);

  let result = interchange::http::GameDetails {
    id,
    created,
    name,
    members,
    rounds,
  };

  Response::ok_json(&result).map(|r| r.cors(context.cors()))
}

pub async fn find(context: &Context, uri: &Uri) -> Result<Response> {
  let uid = match context.authority() {
    Authority::User { id, .. } => id,
    Authority::None => return Ok(Response::not_found().cors(context.cors())),
  };

  let ids = query_values(uri, "ids[]");

  if ids.len() == 0 {
    debug!("find all games not implemented yet");
    return Ok(Response::not_found().cors(context.cors()));
  }

  if ids.len() == 1 {
    let gid = ids.iter().nth(0).ok_or(errors::e("invalid id"))?;
    debug!("attempting to find game from single id - {:?}", gid);
    return find_game(context, uid, gid).await;
  }

  debug!("attempting to find game from ids - {:?}", ids);
  Ok(Response::not_found().cors(context.cors()))
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

pub async fn rounds(context: &Context, uri: &Uri) -> Result<Response> {
  let uid = match context.authority() {
    Authority::User { id, .. } => id,
    Authority::None => return Ok(Response::not_found().cors(context.cors())),
  };

  let ids = query_values(uri, "ids[]");

  if ids.len() == 0 {
    debug!("find all games not implemented yet");
    return Ok(Response::not_found().cors(context.cors()));
  }

  if ids.len() == 1 {
    let rid = ids.iter().nth(0).ok_or(errors::e("invalid id"))?;
    debug!("attempting to find round from single id - {:?}", rid);
    return context
      .records()
      .query(LOAD_ROUND_DETAILS, &[&uid, &rid])?
      .iter()
      .nth(0)
      .and_then(|row| {
        let id = row.try_get(0).map_err(log_err).ok()?;
        let prompt = row.try_get(1).map_err(log_err).ok()?;
        let position = row.try_get::<_, i32>(2).map_err(log_err).ok()? as u32;
        let created = row.try_get(3).map_err(log_err).ok()?;
        let completed = row.try_get(4).map_err(log_err).ok()?;
        let started = row.try_get(5).map_err(log_err).ok()?;

        debug!("found round row '{}', parsing into response", id);
        let entries = entries_for_round(context, &id).map_err(log_err).ok()??;
        let details = interchange::http::GameRoundDetails {
          id,
          entries,
          position,
          prompt,
          created,
          completed,
          started,
        };
        Some(Response::ok_json(details).map(|res| res.cors(context.cors())))
      })
      .unwrap_or_else(|| {
        debug!("unable to find matching row for '{}'", rid);
        Ok(Response::not_found().cors(context.cors()))
      });
  }

  debug!("loading rounds for user '{}'", uid);
  Ok(Response::default().cors(context.cors()))
}

fn entries_for_round(
  context: &Context,
  round_id: &String,
) -> Result<Option<Vec<interchange::http::GameRoundEntry>>> {
  let uid = match context.authority() {
    Authority::User { id, .. } => id,
    Authority::None => return Ok(None),
  };

  debug!(
    "loading round entries for user '{}' & round '{}'",
    uid, round_id
  );

  Ok(Some(
    context
      .records()
      .query(LOAD_ENTRIES, &[uid, round_id])?
      .iter()
      .filter_map(|row| {
        let id = row.try_get::<_, String>(0).map_err(log_err).ok()?;
        let round_id = row.try_get::<_, String>(1).map_err(log_err).ok()?;
        let member_id = row.try_get::<_, String>(2).map_err(log_err).ok()?;
        let entry = row.try_get::<_, String>(3).map_err(log_err).ok()?;
        let created = row.try_get::<_, DateTime<Utc>>(4).map_err(log_err).ok()?;
        let user_id = row.try_get::<_, String>(5).map_err(log_err).ok()?;
        let user_name = row.try_get::<_, String>(6).map_err(log_err).ok()?;
        let user_email = row.try_get::<_, String>(7).map_err(log_err).ok()?;
        debug!("found round entry '{}'", id);
        Some(interchange::http::GameRoundEntry {
          id,
          round_id,
          member_id,
          entry,
          created,
          user_id,
          user_name,
          user_email,
        })
      })
      .collect(),
  ))
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
  let authority = match context
    .records()
    .query(GAME_FOR_ENTRY, &[&payload.round_id, &uid])?
    .iter()
    .nth(0)
    .and_then(|row| {
      let game_id = row.try_get::<_, String>(0).map_err(log_err).ok()?;
      let round_id = row.try_get::<_, String>(1).map_err(log_err).ok()?;
      let member_id = row.try_get::<_, String>(2).map_err(log_err).ok()?;
      let user_id = row.try_get::<_, String>(3).map_err(log_err).ok()?;
      Some((game_id, round_id, member_id, user_id))
    }) {
    None => {
      warn!(
        "unable to find game for user '{}' by round '{}'",
        uid, payload.round_id
      );
      return Ok(Response::not_found().cors(context.cors()));
    }
    Some(game) => game,
  };

  let created = context
    .records()
    .query(CREATE_ENTRY, &[&authority.1, &authority.2, &payload.entry])
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
      warn!("successfully created entry - {:?}", entry);

      context
        .jobs()
        .queue(&interchange::jobs::Job::CheckRoundCompletion {
          round_id,
          result: None,
        })
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

pub async fn entries(context: &Context, uri: &Uri) -> Result<Response> {
  let rid = query_values(uri, "round_id")
    .iter()
    .nth(0)
    .map(|s| s.clone())
    .unwrap_or_default();

  let rows = match entries_for_round(context, &rid)? {
    None => return Ok(Response::not_found().cors(context.cors())),
    Some(rows) => rows,
  };

  Response::ok_json(&rows).map(|r| r.cors(context.cors()))
}
