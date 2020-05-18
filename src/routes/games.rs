use chrono::{DateTime, Utc};
use log::{debug, warn};
use serde::Deserialize;
use serde_json::from_slice as deserialize;
use std::io::Result;

use super::lobbies::LOAD_LOBBY_DETAILS;
use crate::{
  errors,
  http::{query as qs, Uri},
  interchange, read_size_async, Authority, Context, Response,
};

const LOAD_GAME: &'static str = include_str!("data-store/load-game-details.sql");
const LOAD_MEMBERS: &'static str = include_str!("data-store/load-game-members.sql");
const LOAD_ROUNDS: &'static str = include_str!("data-store/load-rounds.sql");

#[derive(Deserialize)]
pub struct CreatePayload {
  pub lobby_id: String,
}

fn log_err<E: std::error::Error>(e: E) -> E {
  warn!("error - {}", e);
  e
}

pub async fn find_game(context: &Context, uid: &String, gid: &String) -> Result<Response> {
  let (id, created, name) = context
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
    })
    .ok_or(errors::e("Unable to parse game data"))?;

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
  let query = uri.query().unwrap_or_default().as_bytes();
  let ids = qs::parse(query)
    .filter_map(|(k, v)| {
      if k == "ids[]" {
        Some(String::from(v))
      } else {
        None
      }
    })
    .collect::<Vec<String>>();

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
