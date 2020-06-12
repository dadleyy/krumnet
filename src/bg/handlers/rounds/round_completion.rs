use super::utils::count_members;
use crate::{bg::context::Context, interchange};
use log::{info, warn};
use sqlx::query_file;

fn warn_and_stringify<E: std::error::Error>(e: E) -> String {
  warn!("{}", e);
  format!("{}", e)
}

async fn count_remaining_rounds(game_id: &String, context: &Context) -> Result<i64, String> {
  let mut conn = context
    .records
    .acquire()
    .await
    .map_err(warn_and_stringify)?;
  query_file!(
    "src/bg/handlers/rounds/data-store/count-remaining-rounds.sql",
    game_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(warn_and_stringify)?
  .into_iter()
  .nth(0)
  .and_then(|row| row.remaining_rounds.map(Ok))
  .unwrap_or(Err(format!("Unable to count remaining rows")))
}

async fn count_votes(round_id: &String, context: &Context) -> Result<i64, String> {
  let mut conn = context
    .records
    .acquire()
    .await
    .map_err(warn_and_stringify)?;
  query_file!(
    "src/bg/handlers/rounds/data-store/count-votes-for-round.sql",
    round_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(warn_and_stringify)?
  .into_iter()
  .nth(0)
  .and_then(|row| row.count.map(Ok))
  .unwrap_or(Err(format!("Unable to count remaining rows")))
}

async fn round_completion_result(
  details: &interchange::jobs::CheckRoundCompletion,
  context: &Context,
) -> Result<Option<String>, String> {
  info!("checking round completion for round '{}'", details.round_id);
  let member_count = count_members(&details.round_id, &context).await?;
  let vote_count = count_votes(&details.round_id, context).await?;

  if vote_count != member_count {
    info!(
      "round {} not complete ({}/{} votes)",
      details.round_id, vote_count, member_count
    );
    return Ok(None);
  }

  let mut conn = context
    .records
    .acquire()
    .await
    .map_err(warn_and_stringify)?;
  query_file!(
    "src/bg/handlers/rounds/data-store/complete-round.sql",
    details.round_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(warn_and_stringify)?;

  info!(
    "creating round placement results for '{}'",
    details.round_id
  );

  query_file!(
    "src/bg/handlers/rounds/data-store/create-round-placements.sql",
    details.round_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(warn_and_stringify)?;

  info!("round '{}' placement results finished", details.round_id);

  let count = count_remaining_rounds(&details.game_id, context).await?;

  if count != 0 {
    info!("{} remaining rounds for game '{}'", count, details.game_id);
    return Ok(None);
  }

  info!(
    "found {} members for round (votes: {:?}). {} remaining rounds",
    member_count, vote_count, count
  );

  let placement_ids = query_file!(
    "src/bg/handlers/rounds/data-store/create-game-placements.sql",
    details.game_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(warn_and_stringify)?
  .into_iter()
  .map(|row| row.id)
  .collect::<Vec<String>>();

  info!("created placement results - {:?}", placement_ids);

  query_file!(
    "src/bg/handlers/rounds/data-store/mark-game-ended.sql",
    details.game_id
  )
  .execute(&mut conn)
  .await
  .map_err(warn_and_stringify)?;

  Ok(Some(details.game_id.clone()))
}

pub async fn check_round_completion(
  details: &interchange::jobs::CheckRoundCompletion,
  context: &Context,
) -> interchange::jobs::Job {
  interchange::jobs::Job::CheckRoundCompletion(interchange::jobs::CheckRoundCompletion {
    result: Some(round_completion_result(details, context).await),
    ..details.clone()
  })
}
