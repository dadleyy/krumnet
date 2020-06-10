use chrono::{DateTime, Utc};
use log::{debug, info, warn};
use sqlx::{query_file, query_file_as, FromRow};
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

#[derive(FromRow)]
struct RoundDetailRow {
  round_id: String,
  prompt: Option<String>,
  pos: i32,
  created_at: DateTime<Utc>,
  completed_at: Option<DateTime<Utc>>,
  started_at: Option<DateTime<Utc>>,
  fulfilled_at: Option<DateTime<Utc>>,
}

async fn round_details(context: &Context, user_id: &String, round_id: &String) -> Result<RoundDetailRow> {
  let mut conn = context.records_connection().await?;
  query_file_as!(
    RoundDetailRow,
    "src/routes/rounds/data-store/load-round-details.sql",
    user_id,
    round_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(log_err)?
  .into_iter()
  .nth(0)
  .ok_or_else(|| errors::e(format!("Unable to find round '{}'", round_id)))
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
    round_id: id,
    prompt,
    pos: position,
    created_at: created,
    fulfilled_at: fulfilled,
    completed_at: completed,
    started_at: started,
  } = round_details(context, &uid, &rid).await?;

  debug!("found round row '{}', parsing into response", id);
  let entries = entries_for_round(context, &uid, &id).await?;
  let results = results_for_round(context, &id).await?;
  let votes = votes_for_round(context, &id).await?;

  let details = interchange::http::GameRoundDetails {
    id,
    entries,
    results,
    votes,
    position,
    fulfilled,
    prompt,
    created,
    completed,
    started,
  };

  Response::ok_json(details).map(|res| res.cors(context.cors()))
}

async fn votes_for_round(context: &Context, round_id: &String) -> Result<Vec<interchange::http::GameRoundVote>> {
  let mut conn = context.records_connection().await?;
  info!("loading votes for round '{}'", round_id);
  query_file_as!(
    interchange::http::GameRoundVote,
    "src/routes/rounds/data-store/load-round-votes.sql",
    round_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(log_err)
}

async fn results_for_round(context: &Context, round_id: &String) -> Result<Vec<interchange::http::GameRoundPlacement>> {
  let mut conn = context.records_connection().await?;

  query_file!("src/routes/rounds/data-store/load-round-results.sql", round_id)
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
  let mut conn = context.records_connection().await?;
  query_file!("src/routes/rounds/data-store/load-round-entries.sql", round_id)
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
