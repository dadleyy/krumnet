use log::{debug, info, warn};
use sqlx::query_file;

use crate::interchange::jobs::CleanupGameMembership as CleanupContext;
use crate::{bg::context::Context, interchange};

fn log_and_serialize<E: std::error::Error>(error: E) -> String {
  warn!("{}", error);
  format!("{}", error)
}

async fn round_ids_without_entries(
  context: &Context,
  details: &CleanupContext,
) -> Result<Vec<String>, String> {
  let mut conn = context.records.acquire().await.map_err(log_and_serialize)?;
  query_file!(
    "src/bg/handlers/game_memberships/data-store/get-round-ids.sql",
    details.user_id,
    details.game_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(log_and_serialize)
  .map(|result| result.into_iter().map(|row| row.round_id).collect())
}

async fn cleanup_inner(context: &Context, details: &CleanupContext) -> Result<Vec<String>, String> {
  let round_ids = round_ids_without_entries(context, details).await?;

  if round_ids.len() == 0 {
    info!(
      "member '{}' left game w/ no outstanding entries",
      details.member_id
    );
    return Ok(round_ids);
  }

  info!("found rounds w/o entries - {:?}", round_ids);

  let mut conn = context.records.acquire().await.map_err(log_and_serialize)?;

  let round_ids = query_file!(
    "src/bg/handlers/game_memberships/data-store/create-empty-entries-for-game-member.sql",
    &details.user_id,
    &details.member_id,
    &round_ids
  )
  .fetch_all(&mut conn)
  .await
  .map_err(log_and_serialize)?
  .into_iter()
  .map(|row| row.round_id)
  .collect::<Vec<String>>();

  for id in &round_ids {
    let job_context = interchange::jobs::CheckRoundFulfillment {
      round_id: id.clone(),
      result: None,
    };
    let job = interchange::jobs::Job::CheckRoundFulfillment(job_context);

    info!("queing round completion check job for round {:?}", job);

    context.jobs.queue(&job).await.map_err(log_and_serialize)?;
  }

  Ok(round_ids)
}

pub async fn cleanup(details: &CleanupContext, context: &Context) -> interchange::jobs::Job {
  debug!("cleaning up game member '{}'", details.member_id);
  interchange::jobs::Job::CleanupGameMembership(interchange::jobs::CleanupGameMembership {
    result: Some(cleanup_inner(context, details).await),
    ..details.clone()
  })
}

#[cfg(test)]
mod tests {
  use super::{cleanup_inner, round_ids_without_entries};
  use crate::{
    bg::{context::Context, test_helpers},
    interchange,
  };
  use async_std::task::block_on;
  use sqlx::query;

  async fn count_entries(context: &Context, member_id: &String) -> i64 {
    let mut conn = context.records.acquire().await.expect("no record store");
    query!(
        "select count(*) as count from krumnet.game_round_entries as entries where entries.member_id = $1",
        member_id
    ).fetch_all(&mut conn)
        .await
        .expect("unable to count")
        .into_iter()
        .nth(0)
        .and_then(|row| row.count)
        .expect("unable to count")
  }

  async fn find_member(context: &Context, user_id: &String, game_id: &String) -> String {
    let mut conn = context.records.acquire().await.expect("no record store");

    query!("select id from krumnet.game_memberships as members where members.user_id = $1 and members.game_id = $2", user_id, game_id)
      .fetch_all(&mut conn)
      .await.expect("cant find member").into_iter().nth(0).map(|r| r.id).expect("unable to find member")
  }

  async fn make_entry(
    context: &Context,
    member_id: &String,
    user_id: &String,
    game_id: &String,
    position: i32,
  ) {
    let mut conn = context.records.acquire().await.expect("no record store");
    query!("
        insert into krumnet.game_round_entries (user_id, member_id, game_id, lobby_id, round_id, entry)
        select $1, $2, rounds.game_id, rounds.lobby_id, rounds.id, 'hi'
        from krumnet.game_rounds as rounds
        where rounds.game_id = cast($3 as varchar) and rounds.position = $4 limit 1
    ", user_id, member_id, game_id, position).execute(&mut conn).await.expect("unable to create entry");
  }

  async fn cleanup_job(context: &Context, job: &interchange::jobs::CleanupGameMembership) {
    test_helpers::cleanup_game(&context, &job.game_id).await;
    test_helpers::cleanup_lobby(&context, &job.lobby_id).await;
    test_helpers::cleanup_user(&context, &job.user_id).await;
  }

  async fn get_job_context(email: &str) -> (Context, interchange::jobs::CleanupGameMembership) {
    let context = test_helpers::get_test_context().await;
    let uid = test_helpers::make_user(&context, email).await;
    let lid = test_helpers::make_lobby(&context, &uid).await;
    let gid = test_helpers::make_game(&context, &uid, &lid).await;
    let mid = find_member(&context, &uid, &gid).await;
    let details = interchange::jobs::CleanupGameMembership {
      user_id: uid.clone(),
      member_id: mid.clone(),
      lobby_id: lid.clone(),
      game_id: gid.clone(),
      result: None,
    };
    (context, details)
  }

  #[test]
  fn invalid_round_ids() {
    block_on(async {
      let context = test_helpers::get_test_context().await;

      let details = interchange::jobs::CleanupGameMembership {
        user_id: String::from("not-exists"),
        member_id: String::from("not-exists"),
        lobby_id: String::from("not-exists"),
        game_id: String::from("not-exists"),
        result: None,
      };
      let result = round_ids_without_entries(&context, &details)
        .await
        .expect("failed query");

      assert_eq!(result, Vec::new() as Vec<String>);
    });
  }

  #[test]
  fn no_games_for_lobby() {
    block_on(async {
      let context = test_helpers::get_test_context().await;
      let uid = test_helpers::make_user(
        &context,
        "krumnet.bg.handlers.game_memeberships.no_games_for_lobby",
      )
      .await;
      let lid = test_helpers::make_lobby(&context, &uid).await;

      let details = interchange::jobs::CleanupGameMembership {
        user_id: uid.clone(),
        member_id: String::from("not-exists"),
        lobby_id: lid.clone(),
        game_id: String::from("not-exists"),
        result: None,
      };

      let result = round_ids_without_entries(&context, &details)
        .await
        .expect("failed query");

      assert_eq!(result, Vec::new() as Vec<String>);
      test_helpers::cleanup_lobby(&context, &lid).await;
      test_helpers::cleanup_user(&context, &uid).await;
    });
  }

  #[test]
  fn count_with_empty_rounds() {
    block_on(async {
      let (context, job) = get_job_context("bg.game_memeberships.count_with_empty_rounds").await;

      let result = round_ids_without_entries(&context, &job)
        .await
        .expect("failed query");

      assert_eq!(result.len(), 3);
      cleanup_job(&context, &job).await;
    });
  }

  #[test]
  fn count_with_some_entries() {
    block_on(async {
      let (context, job) = get_job_context("bg.game_memberships.count_with_some_entries").await;
      make_entry(&context, &job.member_id, &job.user_id, &job.game_id, 0).await;

      let result = round_ids_without_entries(&context, &job)
        .await
        .expect("failed query");

      assert_eq!(result.len(), 2);
      cleanup_job(&context, &job).await;
    });
  }

  #[test]
  fn count_with_most_entries() {
    block_on(async {
      let (context, job) = get_job_context("bg.game_memberships.count_most_entries").await;
      make_entry(&context, &job.member_id, &job.user_id, &job.game_id, 0).await;
      make_entry(&context, &job.member_id, &job.user_id, &job.game_id, 1).await;

      let result = round_ids_without_entries(&context, &job)
        .await
        .expect("failed query");

      assert_eq!(result.len(), 1);
      cleanup_job(&context, &job).await;
    });
  }

  #[test]
  fn count_with_all_entries() {
    block_on(async {
      let (context, job) = get_job_context("bg.game_memberships.count_all_entries").await;

      make_entry(&context, &job.member_id, &job.user_id, &job.game_id, 0).await;
      make_entry(&context, &job.member_id, &job.user_id, &job.game_id, 1).await;
      make_entry(&context, &job.member_id, &job.user_id, &job.game_id, 2).await;

      let result = round_ids_without_entries(&context, &job)
        .await
        .expect("failed query");

      assert_eq!(result.len(), 0);
      cleanup_job(&context, &job).await;
    });
  }

  #[test]
  fn cleanup_with_all_missing() {
    block_on(async {
      let (context, job) = get_job_context("bg.game_memberships.cleanup_with_all_missing").await;

      assert_eq!(count_entries(&context, &job.member_id).await, 0);

      cleanup_inner(&context, &job).await.expect("failed query");

      assert_eq!(count_entries(&context, &job.member_id).await, 3);

      cleanup_job(&context, &job).await
    });
  }

  #[test]
  fn cleanup_with_some_missing() {
    block_on(async {
      let (context, job) = get_job_context("bg.game_memberships.cleanup_with_some_missing").await;

      make_entry(&context, &job.member_id, &job.user_id, &job.game_id, 0).await;
      make_entry(&context, &job.member_id, &job.user_id, &job.game_id, 1).await;

      assert_eq!(count_entries(&context, &job.member_id).await, 2);

      let round_ids = cleanup_inner(&context, &job).await.expect("failed query");
      let mut conn = context
        .records
        .acquire()
        .await
        .expect("unable to get connection");

      let expected_ids = query!(
          "select rounds.id as id from krumnet.game_rounds as rounds where rounds.game_id = $1 and rounds.position = 2",
          &job.game_id
      ).fetch_all(&mut conn)
          .await
          .expect("unable to query for rounds")
          .into_iter()
          .map(|row| row.id)
          .collect::<Vec<String>>();

      assert_eq!(round_ids, expected_ids);
      assert_eq!(count_entries(&context, &job.member_id).await, 3);
      cleanup_job(&context, &job).await
    });
  }

  #[test]
  fn cleanup_with_some_missing_isolated() {
    block_on(async {
      let (context, job) =
        get_job_context("bg.game_memeberships.cleanup_with_some_missing_isolated").await;

      let ogid = test_helpers::make_game(&context, &job.user_id, &job.lobby_id).await;
      let omid = find_member(&context, &job.user_id, &ogid).await;

      make_entry(&context, &job.member_id, &job.user_id, &job.game_id, 0).await;
      make_entry(&context, &job.member_id, &job.user_id, &job.game_id, 1).await;

      assert_eq!(count_entries(&context, &omid).await, 0);
      assert_eq!(count_entries(&context, &job.member_id).await, 2);

      cleanup_inner(&context, &job).await.expect("failed query");

      assert_eq!(count_entries(&context, &job.member_id).await, 3);
      assert_eq!(count_entries(&context, &omid).await, 0);

      assert_eq!(true, true);
      test_helpers::cleanup_game(&context, &ogid).await;
      cleanup_job(&context, &job).await
    });
  }
}
