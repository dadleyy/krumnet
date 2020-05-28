use chrono::{DateTime, Utc};
use log::{debug, warn};
use std::io::{Error, Result};

use crate::{
  errors,
  http::{query_values, Uri},
  interchange, Authority, Context, Response,
};

const LOAD_ROUND_DETAILS: &'static str = include_str!("data-store/load-round-details.sql");
const LOAD_ENTRIES: &'static str = include_str!("data-store/load-round-entries.sql");

fn log_err<E: std::error::Error>(error: E) -> Error {
  warn!("error - {}", error);
  errors::humanize_error(error)
}

pub async fn find(context: &Context, uri: &Uri) -> Result<Response> {
  let uid = match context.authority() {
    Authority::User { id, .. } => id,
    Authority::None => return Ok(Response::not_found().cors(context.cors())),
  };

  let ids = query_values(uri, "ids[]");

  if ids.len() != 1 {
    debug!("find all rounds not implemented yet");
    return Ok(Response::not_found().cors(context.cors()));
  }

  let rid = ids.iter().nth(0).ok_or(errors::e("invalid id"))?;
  debug!("attempting to find round from single id - {:?}", rid);

  context
    .records()
    .query(LOAD_ROUND_DETAILS, &[&uid, &rid])?
    .iter()
    .nth(0)
    .map_or(Ok(Response::not_found().cors(context.cors())), |row| {
      let id = row.try_get("round_id").map_err(log_err)?;
      let prompt = row.try_get("prompt").map_err(log_err)?;
      let position = row.try_get::<_, i32>("pos").map_err(log_err)? as u32;
      let created = row.try_get("created_at").map_err(log_err)?;
      let completed = row.try_get("completed_at").map_err(log_err)?;
      let started = row.try_get("started_at").map_err(log_err)?;
      let fulfilled = row.try_get("fulfilled_at").map_err(log_err)?;

      debug!("found round row '{}', parsing into response", id);
      let entries = entries_for_round(context, &uid, &id)?;
      let details = interchange::http::GameRoundDetails {
        id,
        entries,
        position,
        fulfilled,
        prompt,
        created,
        completed,
        started,
      };
      Response::ok_json(details).map(|res| res.cors(context.cors()))
    })
}

fn entries_for_round(
  context: &Context,
  active_user_id: &String,
  round_id: &String,
) -> Result<Vec<interchange::http::GameRoundEntry>> {
  context
    .records()
    .query(LOAD_ENTRIES, &[round_id])?
    .iter()
    .map(|row| {
      let id = row.try_get("entry_id").map_err(log_err)?;
      let round_id = row.try_get("round_id").map_err(log_err)?;
      let member_id = row.try_get("member_id").map_err(log_err)?;
      let created = row
        .try_get::<_, DateTime<Utc>>("created_at")
        .map_err(log_err)?;
      let user_id = row.try_get("user_id").map_err(log_err)?;
      let user_name = row.try_get("user_name").map_err(log_err)?;

      let entry = match &user_id == active_user_id {
        true => Some(row.try_get("entry").map_err(log_err)?),
        false => None,
      };

      debug!("found round entry '{}'", id);

      Ok(interchange::http::GameRoundEntry {
        id,
        round_id,
        entry,
        member_id,
        created,
        user_id,
        user_name,
      })
    })
    .collect()
}
