use krumnet::RecordStore;
use log::{debug, info, warn};

const QUERY: &'static str = include_str!("../data-store/count-entries-and-members-for-round.sql");
const UPDATE: &'static str = include_str!("../data-store/complete-round.sql");
const START: &'static str = include_str!("../data-store/start-next.sql");

fn warn_and_stringify<E: std::error::Error>(e: E) -> String {
  warn!("{}", e);
  format!("{}", e)
}

pub async fn check_round_completion(
  round_id: &String,
  records: &RecordStore,
) -> Result<u8, String> {
  info!("checking completion of round '{}'", round_id);

  let rows = records
    .query(QUERY, &[round_id])
    .map_err(|e| format!("failed - {}", e))?;

  let row = rows
    .iter()
    .nth(0)
    .ok_or(String::from("unable to find counts"))?;

  let (member_count, entry_count) = (
    row.try_get::<_, i64>(0).map_err(warn_and_stringify)?,
    row.try_get::<_, i64>(1).map_err(warn_and_stringify)?,
  );

  debug!(
    "found member count {} and entry count {}",
    member_count, entry_count
  );

  let diff = member_count as u8 - entry_count as u8;

  if diff != 0 {
    debug!("round has {} entries remaining, moving on", diff);
    return Ok(diff);
  }

  let rows = records
    .query(UPDATE, &[round_id])
    .map_err(warn_and_stringify)?;

  let row = rows
    .iter()
    .nth(0)
    .ok_or(String::from("updated row missing"))?;

  let position = row.try_get::<_, i32>(0).map_err(warn_and_stringify)?;
  let game_id = row.try_get::<_, String>(1).map_err(warn_and_stringify)?;

  debug!("updated position {} in game '{}'", position, game_id);

  records
    .query(START, &[&game_id, &position])
    .map_err(warn_and_stringify)?;

  debug!("done!");

  Ok(diff)
}
