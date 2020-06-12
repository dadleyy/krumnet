use super::utils::count_members;
use crate::{bg::context::Context, interchange};
use log::{debug, info, warn};
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

async fn mark_round_completed(context: &Context, round_id: &String) -> Result<(), String> {
  let mut conn = context
    .records
    .acquire()
    .await
    .map_err(warn_and_stringify)?;

  query_file!(
    "src/bg/handlers/rounds/data-store/complete-round.sql",
    round_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(warn_and_stringify)?;
  Ok(())
}

async fn create_round_placements(
  context: &Context,
  round_id: &String,
) -> Result<Vec<String>, String> {
  let mut conn = context
    .records
    .acquire()
    .await
    .map_err(warn_and_stringify)?;

  let placement_ids = query_file!(
    "src/bg/handlers/rounds/data-store/create-round-placements.sql",
    round_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(warn_and_stringify)?
  .into_iter()
  .map(|row| row.id)
  .collect();

  Ok(placement_ids)
}

async fn create_game_placements(
  context: &Context,
  game_id: &String,
) -> Result<Vec<String>, String> {
  let mut conn = context
    .records
    .acquire()
    .await
    .map_err(warn_and_stringify)?;

  let placement_ids = query_file!(
    "src/bg/handlers/rounds/data-store/create-game-placements.sql",
    game_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(warn_and_stringify)?
  .into_iter()
  .map(|row| row.id)
  .collect();

  Ok(placement_ids)
}

async fn mark_game_ended(context: &Context, game_id: &String) -> Result<(), String> {
  let mut conn = context
    .records
    .acquire()
    .await
    .map_err(warn_and_stringify)?;

  query_file!(
    "src/bg/handlers/rounds/data-store/mark-game-ended.sql",
    game_id
  )
  .execute(&mut conn)
  .await
  .map_err(warn_and_stringify)?;

  Ok(())
}

async fn round_completion_result(
  context: &Context,
  details: &interchange::jobs::CheckRoundCompletion,
) -> Result<interchange::jobs::CheckRoundCompletionResult, String> {
  info!("checking round completion for round '{}'", details.round_id);
  let member_count = count_members(&context, &details.round_id).await?;
  let vote_count = count_votes(&details.round_id, context).await?;

  if vote_count != member_count {
    info!(
      "round {} not complete ({}/{} votes)",
      details.round_id, vote_count, member_count
    );
    return Ok(interchange::jobs::CheckRoundCompletionResult::Incomplete);
  }

  debug!("round looks complete, marking");
  mark_round_completed(context, &details.round_id).await?;

  info!("creating round-placement for '{}'", details.round_id);
  let placement_ids = create_round_placements(context, &details.round_id).await?;

  info!("round '{}' placement results finished", details.round_id);

  let count = count_remaining_rounds(&details.game_id, context).await?;

  if count != 0 {
    info!("{} remaining rounds for game '{}'", count, details.game_id);
    return Ok(interchange::jobs::CheckRoundCompletionResult::Intermediate(
      placement_ids,
    ));
  }

  info!(
    "found {} members for round (votes: {:?}). {} remaining rounds",
    member_count, vote_count, count
  );

  let placement_ids = create_game_placements(context, &details.game_id).await?;

  info!("created placement results - {:?}", placement_ids);

  mark_game_ended(&context, &details.game_id).await?;
  Ok(interchange::jobs::CheckRoundCompletionResult::Final(
    placement_ids,
  ))
}

pub async fn check_round_completion(
  details: &interchange::jobs::CheckRoundCompletion,
  context: &Context,
) -> interchange::jobs::Job {
  let result = Some(round_completion_result(context, details).await);

  let completion = interchange::jobs::CheckRoundCompletion {
    result,
    ..details.clone()
  };

  interchange::jobs::Job::CheckRoundCompletion(completion)
}

#[cfg(test)]
mod test {
  use super::round_completion_result;
  use crate::{
    bg::handlers::rounds::check_round_fulfillment,
    bg::{context::Context, test_helpers},
    interchange,
  };
  use async_std::task::block_on;
  use sqlx::query;

  struct TestContext {
    user_id: String,
    game_id: String,
    lobby_id: String,
  }

  async fn get_test_context(name: &str) -> (Context, TestContext) {
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

  async fn cleanup_test_context(context: &Context, test_context: TestContext) {
    test_helpers::cleanup_game(&context, &test_context.game_id).await;
    test_helpers::cleanup_lobby(&context, &test_context.lobby_id).await;
    test_helpers::cleanup_user(&context, &test_context.user_id).await;
  }

  fn job_from_test_context(
    context: &TestContext,
    round_id: &String,
  ) -> interchange::jobs::CheckRoundCompletion {
    interchange::jobs::CheckRoundCompletion {
      round_id: round_id.clone(),
      game_id: context.game_id.clone(),
      result: None,
    }
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

  async fn create_round_entry(
    context: &Context,
    test_context: &TestContext,
    round_id: &String,
  ) -> String {
    let mut conn = context.records.acquire().await.expect("unable to connect");
    let q = query!(
      "
      insert into krumnet.game_round_entries (user_id, member_id, round_id, game_id, lobby_id)
      select members.user_id, members.id, $3, members.game_id, members.lobby_id
      from krumnet.game_memberships as members where members.user_id = $1 and members.game_id = $2
      returning id
      ",
      &test_context.user_id,
      &test_context.game_id,
      round_id,
    );
    q.fetch_all(&mut conn)
      .await
      .expect("unable to insert")
      .into_iter()
      .nth(0)
      .map(|row| row.id)
      .expect("unable to get id")
  }

  async fn get_game_placements(context: &Context, game_id: &String) -> Vec<String> {
    let mut conn = context.records.acquire().await.expect("unable to connect");
    let q = query!(
        "select placements.id from krumnet.game_member_placement_results as placements where placements.game_id = $1",
        game_id
    );
    q.fetch_all(&mut conn)
      .await
      .expect("unable to find placements")
      .into_iter()
      .map(|row| row.id)
      .collect()
  }

  async fn get_round_placements(context: &Context, round_id: &String) -> Vec<String> {
    let mut conn = context.records.acquire().await.expect("unable to connect");
    let q = query!(
        "select placements.id from krumnet.game_member_round_placement_results as placements where placements.round_id = $1",
        round_id
    );
    q.fetch_all(&mut conn)
      .await
      .expect("unable to find placements")
      .into_iter()
      .map(|row| row.id)
      .collect()
  }

  async fn create_round_vote(
    context: &Context,
    test_context: &TestContext,
    round_id: &String,
    entry_id: &String,
  ) {
    let mut conn = context.records.acquire().await.expect("unable to connect");
    let q = query!(
      "
      insert into krumnet.game_round_entry_votes (user_id, member_id, round_id, game_id, lobby_id, entry_id)
      select members.user_id, members.id, $3, members.game_id, members.lobby_id, $4
      from krumnet.game_memberships as members where members.user_id = $1 and members.game_id = $2
      ",
      &test_context.user_id,
      &test_context.game_id,
      round_id,
      entry_id,
    );
    q.execute(&mut conn).await.expect("unable to insert");
  }

  async fn fulfill(context: &Context, round_id: &String) {
    check_round_fulfillment(
      &interchange::jobs::CheckRoundFulfillment {
        round_id: round_id.clone(),
        result: None,
      },
      &context,
    )
    .await;
  }

  async fn complete(
    context: &Context,
    test_context: &TestContext,
    round_id: &String,
  ) -> Result<interchange::jobs::CheckRoundCompletionResult, String> {
    let job = job_from_test_context(&test_context, &round_id);
    round_completion_result(&context, &job).await
  }

  #[test]
  fn err_when_bogus() {
    block_on(async {
      let (context, test_context) = get_test_context("bg.handlers.round_completion.bogus").await;
      assert_eq!(true, true);
      cleanup_test_context(&context, test_context).await
    });
  }

  #[test]
  fn nothing_when_running() {
    block_on(async {
      let test_name = "bg.handlers.round_completion.nothing_when_running";
      let (context, test_context) = get_test_context(test_name).await;
      let round_id = get_round_id(&context, &test_context.game_id, 0).await;
      let job = job_from_test_context(&test_context, &round_id);
      let result = round_completion_result(&context, &job).await;
      assert_eq!(result.is_ok(), true);
      assert_eq!(
        result.unwrap(),
        interchange::jobs::CheckRoundCompletionResult::Incomplete
      );
      cleanup_test_context(&context, test_context).await
    });
  }

  #[test]
  fn crate_placements_when_full() {
    block_on(async {
      let test_name = "bg.handlers.round_completion.complete_and_place_when_full";
      let (context, test_context) = get_test_context(test_name).await;
      let round_id = get_round_id(&context, &test_context.game_id, 0).await;
      let placements = get_round_placements(&context, &round_id).await;
      assert_eq!(placements.len(), 0);
      let entry_id = create_round_entry(&context, &test_context, &round_id).await;
      create_round_vote(&context, &test_context, &round_id, &entry_id).await;
      let job = job_from_test_context(&test_context, &round_id);
      let result = round_completion_result(&context, &job).await;
      let placements = get_round_placements(&context, &round_id).await;
      assert_eq!(
        result.unwrap(),
        interchange::jobs::CheckRoundCompletionResult::Intermediate(placements)
      );
      cleanup_test_context(&context, test_context).await
    });
  }

  #[test]
  fn complete_when_all_full() {
    block_on(async {
      let test_name = "bg.handlers.round_completion.complete_when_all_full";
      let (context, test_context) = get_test_context(test_name).await;
      let rounds = (
        get_round_id(&context, &test_context.game_id, 0).await,
        get_round_id(&context, &test_context.game_id, 1).await,
        get_round_id(&context, &test_context.game_id, 2).await,
      );

      let entries = (
        create_round_entry(&context, &test_context, &rounds.0).await,
        create_round_entry(&context, &test_context, &rounds.1).await,
        create_round_entry(&context, &test_context, &rounds.2).await,
      );

      fulfill(&context, &rounds.0).await;
      fulfill(&context, &rounds.1).await;
      fulfill(&context, &rounds.2).await;

      create_round_vote(&context, &test_context, &rounds.0, &entries.0).await;
      create_round_vote(&context, &test_context, &rounds.1, &entries.1).await;
      create_round_vote(&context, &test_context, &rounds.2, &entries.2).await;

      let placements = get_round_placements(&context, &rounds.0).await;
      assert_eq!(placements.len(), 0);

      complete(&context, &test_context, &rounds.0)
        .await
        .expect("unable to complete");
      complete(&context, &test_context, &rounds.1)
        .await
        .expect("unable to complete");

      let result = complete(&context, &test_context, &rounds.2).await;
      let placements = get_game_placements(&context, &test_context.game_id).await;

      assert_eq!(result.is_ok(), true);
      assert_eq!(
        result.unwrap(),
        interchange::jobs::CheckRoundCompletionResult::Final(placements)
      );
      cleanup_test_context(&context, test_context).await
    });
  }
}
