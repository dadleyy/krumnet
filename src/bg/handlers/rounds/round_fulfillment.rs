use super::utils::count_members;
use crate::{bg::context::Context, interchange};
use log::{debug, info, warn};
use sqlx::query_file;

fn warn_and_stringify<E: std::error::Error>(e: E) -> String {
  warn!("{}", e);
  format!("{}", e)
}

pub async fn count_entries(round_id: &String, context: &Context) -> Result<i64, String> {
  let mut conn = context
    .records
    .acquire()
    .await
    .map_err(warn_and_stringify)?;
  query_file!(
    "src/bg/handlers/rounds/data-store/count-entries-for-round.sql",
    round_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(warn_and_stringify)?
  .into_iter()
  .nth(0)
  .and_then(|row| row.entry_count)
  .ok_or(format!("Unable to count entries for round '{}'", round_id,))
}

async fn check_round_fulfillment_inner(round_id: &String, context: &Context) -> Result<u8, String> {
  info!("checking fulfillment of round '{}'", round_id);
  let entry_count = count_entries(round_id, context).await?;
  let member_count = count_members(context, round_id).await?;

  debug!(
    "found member count {} and entry count {}",
    member_count, entry_count
  );

  let diff = member_count as u8 - entry_count as u8;

  if diff != 0 {
    debug!("round has {} entries remaining, moving on", diff);
    return Ok(diff);
  }

  let mut conn = context
    .records
    .acquire()
    .await
    .map_err(warn_and_stringify)?;

  let (position, game_id) = query_file!(
    "src/bg/handlers/rounds/data-store/fulfill-round.sql",
    round_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(warn_and_stringify)?
  .into_iter()
  .nth(0)
  .map(|row| (row.position, row.game_id))
  .ok_or(format!("Unable to mark round '{}' fulfilled", round_id))?;

  debug!("updated position {} in game '{}'", position, game_id);

  query_file!(
    "src/bg/handlers/rounds/data-store/start-next.sql",
    game_id,
    position
  )
  .fetch_all(&mut conn)
  .await
  .map_err(warn_and_stringify)?;

  Ok(diff)
}

pub async fn check_round_fulfillment(
  details: &interchange::jobs::CheckRoundFulfillment,
  context: &Context,
) -> interchange::jobs::Job {
  let result = Some(check_round_fulfillment_inner(&details.round_id, context).await);
  interchange::jobs::Job::CheckRoundFulfillment(interchange::jobs::CheckRoundFulfillment {
    round_id: details.round_id.clone(),
    result,
  })
}
