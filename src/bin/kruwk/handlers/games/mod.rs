use krumnet::RecordStore;
use log::{debug, info, warn};

const COUNT_ENTRIES: &'static str = include_str!("./data-store/count-entries-for-round.sql");
const COUNT_MEMBERS: &'static str = include_str!("./data-store/count-members-for-round.sql");
const UPDATE_ROUND: &'static str = include_str!("./data-store/complete-round.sql");
const END_GAME: &'static str = include_str!("./data-store/mark-game-ended.sql");
const START_NEXT: &'static str = include_str!("./data-store/start-next.sql");

fn warn_and_stringify<E: std::error::Error>(e: E) -> String {
  warn!("{}", e);
  format!("{}", e)
}

pub fn count_members(round_id: &String, records: &RecordStore) -> Result<i64, String> {
  records
    .query(COUNT_MEMBERS, &[round_id])
    .map_err(warn_and_stringify)
    .and_then(|rows| {
      rows
        .into_iter()
        .nth(0)
        .ok_or(String::from("unable to find counts"))
    })
    .and_then(|row| row.try_get::<_, i64>(1).map_err(warn_and_stringify))
}

pub fn count_entries(round_id: &String, records: &RecordStore) -> Result<i64, String> {
  records
    .query(COUNT_ENTRIES, &[round_id])
    .map_err(warn_and_stringify)
    .and_then(|rows| {
      rows
        .into_iter()
        .nth(0)
        .ok_or(String::from("unable to find counts"))
    })
    .and_then(|row| row.try_get::<_, i64>(1).map_err(warn_and_stringify))
}

pub async fn check_round_fullfillment(
  round_id: &String,
  records: &RecordStore,
) -> Result<u8, String> {
  info!("checking fullfillment of round '{}'", round_id);

  let entry_count = count_entries(round_id, records)?;
  let member_count = count_members(round_id, records)?;

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
    .query(UPDATE_ROUND, &[round_id])
    .map_err(warn_and_stringify)?;

  let row = rows
    .iter()
    .nth(0)
    .ok_or(String::from("updated row missing"))?;

  let position = row.try_get::<_, i32>(0).map_err(warn_and_stringify)?;
  let game_id = row.try_get::<_, String>(1).map_err(warn_and_stringify)?;

  debug!("updated position {} in game '{}'", position, game_id);

  records
    .query(START_NEXT, &[&game_id, &position])
    .map_err(warn_and_stringify)?;

  Ok(diff)
}

#[cfg(test)]
mod test {
  use super::check_round_fullfillment;
  use async_std::task::block_on;
  use krumnet::{Configuration, RecordStore};
  use std::env;
  use std::io::Result;

  const CONFIG_VAR: &'static str = "KRUMNET_TEST_CONFIG_FILE";

  pub fn load_test_config() -> Result<Configuration> {
    let path = env::var(CONFIG_VAR).unwrap_or(String::from("krumnet-config.example.json"));
    Configuration::load(&path)
  }

  pub fn get_records() -> RecordStore {
    block_on(async {
      let config = load_test_config().expect("unable to load test config");
      RecordStore::open(&config)
        .await
        .expect("unable to connect to record store")
    })
  }

  #[test]
  fn test_not_found() {
    let records = get_records();
    let id = String::from("not-valid");
    let res = block_on(async { check_round_fullfillment(&id, &records).await });
    assert!(res.is_err());
    assert_eq!(format!("{}", res.unwrap_err()), "unable to find counts");
  }
}
