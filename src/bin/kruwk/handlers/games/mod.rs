use super::Context;
use krumnet::{interchange, RecordStore};
use log::{debug, info, warn};

const COUNT_ENTRIES: &'static str = include_str!("./data-store/count-entries-for-round.sql");
const COUNT_VOTES: &'static str = include_str!("./data-store/count-votes-for-round.sql");
const COUNT_MEMBERS: &'static str = include_str!("./data-store/count-members-for-round.sql");
const COMPELTE_ROUND: &'static str = include_str!("./data-store/complete-round.sql");
const COUNT_REMAINING_ROUNDS: &'static str =
  include_str!("./data-store/count-remaining-rounds.sql");
const FULFILL_ROUND: &'static str = include_str!("./data-store/fulfill-round.sql");
const END_GAME: &'static str = include_str!("./data-store/mark-game-ended.sql");
const START_NEXT: &'static str = include_str!("./data-store/start-next.sql");
const CREATE_ROUND_PLACEMENTS: &'static str =
  include_str!("./data-store/create-round-placements.sql");
const CREATE_GAME_PLACEMENTS: &'static str =
  include_str!("./data-store/create-game-placements.sql");

fn warn_and_stringify<E: std::error::Error>(e: E) -> String {
  warn!("{}", e);
  format!("{}", e)
}

fn count_remaining_rounds(game_id: &String, context: &Context<'_>) -> Result<i64, String> {
  context
    .records
    .query(COUNT_REMAINING_ROUNDS, &[game_id])
    .map_err(warn_and_stringify)?
    .into_iter()
    .nth(0)
    .map(|row| row.try_get("remaining_rounds").map_err(warn_and_stringify))
    .unwrap_or(Err(format!("Unable to count remaining rows")))
}

fn round_completion_result(
  details: &interchange::jobs::CheckRoundCompletion,
  context: &Context<'_>,
) -> Result<Option<String>, String> {
  info!("checking round completion for round '{}'", details.round_id);
  let member_count = count_members(&details.round_id, context.records)?;

  let vote_count = context
    .records
    .query(COUNT_VOTES, &[&details.round_id])
    .map_err(warn_and_stringify)?
    .into_iter()
    .nth(0)
    .map(|row| row.try_get::<_, i64>("count").map_err(warn_and_stringify))
    .unwrap_or_else(|| {
      Err(format!(
        "Unable to get vote count for round {}",
        details.round_id
      ))
    })?;

  if vote_count != member_count {
    info!(
      "round {} not complete ({}/{} votes)",
      details.round_id, vote_count, member_count
    );
    return Ok(None);
  }

  context
    .records
    .query(COMPELTE_ROUND, &[&details.round_id])
    .map_err(warn_and_stringify)?
    .into_iter()
    .nth(0)
    .ok_or(format!(
      "Unable to mark round '{}' complete",
      details.round_id
    ))?;

  info!("creating round '{}' placement results", details.round_id);

  context
    .records
    .query(CREATE_ROUND_PLACEMENTS, &[&details.round_id])
    .map_err(warn_and_stringify)?;

  info!("round '{}' placement results finished", details.round_id);

  let count = count_remaining_rounds(&details.game_id, context)?;

  if count != 0 {
    info!("{} remaining rounds for game '{}'", count, details.game_id);
    return Ok(None);
  }

  info!(
    "found {} members for round (votes: {:?}). {} remaining rounds",
    member_count, vote_count, count
  );

  let placement_ids = context
    .records
    .query(CREATE_GAME_PLACEMENTS, &[&details.game_id])
    .map_err(warn_and_stringify)?
    .into_iter()
    .map(|row| row.try_get::<_, String>("id").map_err(warn_and_stringify))
    .collect::<Result<Vec<String>, String>>()?;

  info!("created placement results - {:?}", placement_ids);

  context
    .records
    .query(END_GAME, &[&details.game_id])
    .map_err(warn_and_stringify)?;

  Ok(Some(details.game_id.clone()))
}

pub async fn check_round_completion(
  details: &interchange::jobs::CheckRoundCompletion,
  context: &Context<'_>,
) -> interchange::jobs::Job {
  interchange::jobs::Job::CheckRoundCompletion(interchange::jobs::CheckRoundCompletion {
    result: Some(round_completion_result(details, context)),
    ..details.clone()
  })
}

pub fn count_members(round_id: &String, records: &RecordStore) -> Result<i64, String> {
  records
    .query(COUNT_MEMBERS, &[round_id])
    .map_err(warn_and_stringify)
    .and_then(|rows| {
      rows
        .into_iter()
        .nth(0)
        .ok_or(format!("Unable to count members for round '{}'", round_id))
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
        .ok_or(format!("Unable to count entries for round '{}'", round_id,))
    })
    .and_then(|row| row.try_get::<_, i64>(1).map_err(warn_and_stringify))
}

async fn check_round_fullfillment_inner(
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
    .query(FULFILL_ROUND, &[round_id])
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

pub async fn check_round_fullfillment(
  details: &interchange::jobs::CheckRoundFulfillment,
  records: &RecordStore,
) -> interchange::jobs::Job {
  let result = Some(check_round_fullfillment_inner(&details.round_id, records).await);
  interchange::jobs::Job::CheckRoundFulfillment(interchange::jobs::CheckRoundFulfillment {
    round_id: details.round_id.clone(),
    result,
  })
}

#[cfg(test)]
mod test {
  use super::check_round_fullfillment;
  use async_std::task::block_on;
  use krumnet::{interchange, Configuration, RecordStore};
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
    let job = interchange::jobs::CheckRoundFulfillment {
      round_id: id,
      result: None,
    };
    let res = block_on(async { check_round_fullfillment(&job, &records).await });
    assert_eq!(
      res,
      interchange::jobs::Job::CheckRoundFulfillment(interchange::jobs::CheckRoundFulfillment {
        result: Some(Err(String::from(
          "Unable to count entries for round 'not-valid'"
        ))),
        ..job
      })
    );
  }
}
