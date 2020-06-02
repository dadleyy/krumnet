use chrono::{DateTime, Utc};
use log::{debug, warn};
use sqlx::query_file;
use std::io::{Error, Result};

use crate::{
  errors,
  http::{query_values, Uri},
  interchange, Authority, Context, Response,
};

fn log_err<E: std::error::Error>(error: E) -> Error {
  warn!("error - {}", error);
  errors::humanize_error(error)
}

struct RoundDetailRow {
  id: String,
  prompt: Option<String>,
  position: i32,
  created: DateTime<Utc>,
  completed: Option<DateTime<Utc>>,
  started: Option<DateTime<Utc>>,
  fulfilled: Option<DateTime<Utc>>,
}

async fn round_details(
  context: &Context,
  user_id: &String,
  round_id: &String,
) -> Result<RoundDetailRow> {
  let mut conn = context.records().q().await?;
  query_file!(
    "src/routes/rounds/data-store/load-round-details.sql",
    user_id,
    round_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(log_err)?
  .into_iter()
  .nth(0)
  .map(|row| {
    Ok(RoundDetailRow {
      id: row.round_id,
      prompt: row.prompt,
      position: row.pos,
      created: row
        .created_at
        .ok_or_else(|| errors::e("Unable to parse round created timestamp"))?,
      completed: row.completed_at,
      started: row.started_at,
      fulfilled: row.fulfilled_at,
    })
  })
  .unwrap_or_else(|| Err(errors::e(format!("Unable to find round '{}'", round_id))))
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
  let RoundDetailRow {
    id,
    prompt,
    position,
    created,
    fulfilled,
    completed,
    started,
  } = round_details(context, &uid, &rid).await?;

  debug!("found round row '{}', parsing into response", id);
  let entries = entries_for_round(context, &uid, &id).await?;
  let results = results_for_round(context, &id).await?;

  let details = interchange::http::GameRoundDetails {
    id,
    entries,
    results,
    position,
    fulfilled,
    prompt,
    created,
    completed,
    started,
  };

  Response::ok_json(details).map(|res| res.cors(context.cors()))
}

async fn results_for_round(
  context: &Context,
  round_id: &String,
) -> Result<Vec<interchange::http::GameRoundPlacement>> {
  let mut conn = context.records().q().await?;

  query_file!(
    "src/routes/rounds/data-store/load-round-results.sql",
    round_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(log_err)?
  .into_iter()
  .map(|row| {
    Ok(interchange::http::GameRoundPlacement {
      id: row.result_id,
      user_name: row.user_name,
      user_id: row.user_id,
      place: row.round_place,
    })
  })
  .collect()
}

async fn entries_for_round(
  context: &Context,
  active_user_id: &String,
  round_id: &String,
) -> Result<Vec<interchange::http::GameRoundEntry>> {
  let mut conn = context.records().q().await?;
  query_file!(
    "src/routes/rounds/data-store/load-round-entries.sql",
    round_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(log_err)?
  .into_iter()
  .map(|row| {
    let fulfilled = row.fulfilled;
    let entry = match (&row.user_id == active_user_id) || fulfilled.is_some() {
      true => row.entry,
      false => None,
    };

    Ok(interchange::http::GameRoundEntry {
      id: row.entry_id,
      round_id: row.round_id,
      member_id: row.member_id,
      created: row
        .created_at
        .ok_or_else(|| errors::e("Unable to load round entry created timestamp"))?,
      user_id: row.user_id,
      user_name: row.user_name,
      entry,
    })
  })
  .collect()
}
