use super::Context;
use krumnet::{interchange, RecordStore};
use log::{debug, info, warn};
use sqlx::query_file;

fn warn_and_stringify<E: std::error::Error>(e: E) -> String {
  warn!("{}", e);
  format!("{}", e)
}

async fn count_remaining_rounds(game_id: &String, context: &Context<'_>) -> Result<i64, String> {
  let mut conn = context
    .records
    .acquire()
    .await
    .map_err(warn_and_stringify)?;
  query_file!(
    "src/bin/kruwk/handlers/games/data-store/count-remaining-rounds.sql",
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

async fn count_votes(round_id: &String, context: &Context<'_>) -> Result<i64, String> {
  let mut conn = context
    .records
    .acquire()
    .await
    .map_err(warn_and_stringify)?;
  query_file!(
    "src/bin/kruwk/handlers/games/data-store/count-votes-for-round.sql",
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
  context: &Context<'_>,
) -> Result<Option<String>, String> {
  info!("checking round completion for round '{}'", details.round_id);
  let member_count = count_members(&details.round_id, context.records).await?;
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
    "src/bin/kruwk/handlers/games/data-store/complete-round.sql",
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
    "src/bin/kruwk/handlers/games/data-store/create-round-placements.sql",
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
    "src/bin/kruwk/handlers/games/data-store/create-game-placements.sql",
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
    "src/bin/kruwk/handlers/games/data-store/mark-game-ended.sql",
    details.game_id
  )
  .execute(&mut conn)
  .await
  .map_err(warn_and_stringify)?;

  Ok(Some(details.game_id.clone()))
}

pub async fn check_round_completion(
  details: &interchange::jobs::CheckRoundCompletion,
  context: &Context<'_>,
) -> interchange::jobs::Job {
  interchange::jobs::Job::CheckRoundCompletion(interchange::jobs::CheckRoundCompletion {
    result: Some(round_completion_result(details, context).await),
    ..details.clone()
  })
}

async fn count_members(round_id: &String, records: &RecordStore) -> Result<i64, String> {
  let mut conn = records.acquire().await.map_err(warn_and_stringify)?;
  query_file!(
    "src/bin/kruwk/handlers/games/data-store/count-members-for-round.sql",
    round_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(warn_and_stringify)?
  .into_iter()
  .nth(0)
  .and_then(|row| row.member_count)
  .ok_or(format!("Unable to count members for round '{}'", round_id))
}

async fn count_entries(round_id: &String, records: &RecordStore) -> Result<i64, String> {
  let mut conn = records.acquire().await.map_err(warn_and_stringify)?;
  query_file!(
    "src/bin/kruwk/handlers/games/data-store/count-entries-for-round.sql",
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

async fn check_round_fullfillment_inner(
  round_id: &String,
  records: &RecordStore,
) -> Result<u8, String> {
  info!("checking fullfillment of round '{}'", round_id);
  let entry_count = count_entries(round_id, records).await?;
  let member_count = count_members(round_id, records).await?;

  debug!(
    "found member count {} and entry count {}",
    member_count, entry_count
  );

  let diff = member_count as u8 - entry_count as u8;

  if diff != 0 {
    debug!("round has {} entries remaining, moving on", diff);
    return Ok(diff);
  }

  let mut conn = records.acquire().await.map_err(warn_and_stringify)?;
  let (position, game_id) = query_file!(
    "src/bin/kruwk/handlers/games/data-store/fulfill-round.sql",
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
    "src/bin/kruwk/handlers/games/data-store/start-next.sql",
    game_id,
    position
  )
  .fetch_all(&mut conn)
  .await
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
