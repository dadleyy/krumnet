use super::utils::count_members;
use crate::{bg::context::Context, interchange};
use log::{debug, info, warn};
use sqlx::query_file;

fn warn_and_stringify<E: std::error::Error>(e: E) -> String {
  warn!("{}", e);
  format!("{}", e)
}

async fn count_entries(context: &Context, round_id: &String) -> Result<i64, String> {
  let mut conn = context
    .records
    .acquire()
    .await
    .map_err(warn_and_stringify)?;

  let result = query_file!(
    "src/bg/handlers/rounds/data-store/count-entries-for-round.sql",
    round_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(warn_and_stringify)?;

  result
    .into_iter()
    .nth(0)
    .and_then(|row| row.entry_count)
    .ok_or(format!("Unable to count entries for round '{}'", round_id))
}

async fn round_fulfillment_result(context: &Context, round_id: &String) -> Result<u8, String> {
  info!("checking fulfillment of round '{}'", round_id);
  let entry_count = count_entries(context, round_id).await?;
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
  let result = Some(round_fulfillment_result(context, &details.round_id).await);

  let details = interchange::jobs::CheckRoundFulfillment {
    round_id: details.round_id.clone(),
    result,
  };

  interchange::jobs::Job::CheckRoundFulfillment(details)
}

#[cfg(test)]
mod test {
  use super::{count_entries, round_fulfillment_result};
  use crate::bg::{context::Context, test_helpers};
  use async_std::task::block_on;
  use sqlx::query;

  struct TestContext {
    user_id: String,
    game_id: String,
    lobby_id: String,
  }

  async fn cleanup_test_context(context: &Context, test_context: &TestContext) {
    test_helpers::cleanup_game(&context, &test_context.game_id).await;
    test_helpers::cleanup_lobby(&context, &test_context.lobby_id).await;
    test_helpers::cleanup_user(&context, &test_context.user_id).await;
  }

  async fn get_round_id(context: &Context, game_id: &String, position: i32) -> String {
    let mut conn = context.records.acquire().await.expect("unable to connect");
    let q = query!(
          "select rounds.id from krumnet.game_rounds as rounds where rounds.game_id = $1 and rounds.position = $2",
          game_id,
          position
      );
    q.fetch_all(&mut conn)
      .await
      .expect("unable to query")
      .into_iter()
      .nth(0)
      .map(|row| row.id)
      .expect("unable to find row")
  }

  async fn test_context(name: &str) -> (Context, TestContext) {
    let (context, user_id) = test_helpers::get_test_context_with_user(name).await;
    let lobby_id = test_helpers::make_lobby(&context, &user_id).await;
    let game_id = test_helpers::make_game(&context, &user_id, &lobby_id).await;

    let test_context = TestContext {
      user_id,
      game_id,
      lobby_id,
    };

    (context, test_context)
  }

  async fn is_round_started(context: &Context, round_id: &String) -> bool {
    let mut conn = context.records.acquire().await.expect("unable to connect");
    let q = query!(
      "select rounds.started_at from krumnet.game_rounds as rounds where rounds.id = $1",
      round_id
    );

    q.fetch_all(&mut conn)
      .await
      .expect("unable to lookup")
      .into_iter()
      .nth(0)
      .map(|row| row.started_at.is_some())
      .expect("unable to find round")
  }

  async fn is_round_fulfilled(context: &Context, round_id: &String) -> bool {
    let mut conn = context.records.acquire().await.expect("unable to connect");
    let q = query!(
      "select rounds.fulfilled_at from krumnet.game_rounds as rounds where rounds.id = $1",
      round_id
    );

    q.fetch_all(&mut conn)
      .await
      .expect("unable to lookup")
      .into_iter()
      .nth(0)
      .map(|row| row.fulfilled_at.is_some())
      .expect("unable to find round")
  }

  async fn create_round_entry(context: &Context, test_context: &TestContext, round_id: &String) {
    let mut conn = context.records.acquire().await.expect("unable to connect");
    let q = query!(
      "
      insert into krumnet.game_round_entries (user_id, member_id, round_id, game_id, lobby_id)
      select members.user_id, members.id, $3, members.game_id, members.lobby_id
      from krumnet.game_memberships as members where members.user_id = $1 and members.game_id = $2
      ",
      &test_context.user_id,
      &test_context.game_id,
      round_id,
    );
    q.execute(&mut conn).await.expect("unable to insert");
  }

  #[test]
  fn count_entries_none() {
    block_on(async {
      let (context, test_context) = test_context("bg.round_fulfillment.count_entries_none").await;
      let round_id = get_round_id(&context, &test_context.game_id, 0).await;
      assert_eq!(count_entries(&context, &round_id).await.unwrap(), 0);
      cleanup_test_context(&context, &test_context).await;
    });
  }

  #[test]
  fn count_entries_some() {
    block_on(async {
      let (context, test_context) = test_context("bg.round_fulfillment.count_entries_some").await;
      let round_id = get_round_id(&context, &test_context.game_id, 0).await;
      create_round_entry(&context, &test_context, &round_id).await;
      assert_eq!(count_entries(&context, &round_id).await.unwrap(), 1);
      cleanup_test_context(&context, &test_context).await;
    });
  }

  #[test]
  fn err_on_bogus_round() {
    block_on(async {
      let context = test_helpers::get_test_context().await;
      let result = round_fulfillment_result(&context, &String::from("bogus")).await;
      assert_eq!(result.is_err(), true);
    });
  }

  #[test]
  fn nothing_when_empty() {
    block_on(async {
      let (context, test_context) = test_context("bg.round_fulfillment.nothing_when_empty").await;
      let round_id = get_round_id(&context, &test_context.game_id, 0).await;
      assert_eq!(is_round_fulfilled(&context, &round_id).await, false);
      let result = round_fulfillment_result(&context, &round_id).await;
      assert_eq!(result.is_ok(), true);
      assert_eq!(result.unwrap(), 1);
      assert_eq!(is_round_fulfilled(&context, &round_id).await, false);
      cleanup_test_context(&context, &test_context).await;
    });
  }

  #[test]
  fn fulfill_when_full() {
    block_on(async {
      let (context, test_context) = test_context("bg.round_fulfillment.fulfill_when_full").await;
      let round_id = get_round_id(&context, &test_context.game_id, 0).await;
      let next_round_id = get_round_id(&context, &test_context.game_id, 1).await;
      assert_eq!(is_round_fulfilled(&context, &round_id).await, false);
      assert_eq!(is_round_started(&context, &next_round_id).await, false);
      create_round_entry(&context, &test_context, &round_id).await;

      let result = round_fulfillment_result(&context, &round_id).await;
      assert_eq!(result.is_ok(), true);
      assert_eq!(is_round_fulfilled(&context, &round_id).await, true);
      assert_eq!(is_round_started(&context, &next_round_id).await, true);
      cleanup_test_context(&context, &test_context).await;
    });
  }
}
