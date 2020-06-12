use log::{debug, info, warn};
use sqlx::query_file;

use crate::interchange::jobs::CleanupGameMembershipContext as CleanupContext;
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

pub async fn cleanup_inner(details: &CleanupContext, context: &Context) -> Result<String, String> {
  let round_ids = round_ids_without_entries(context, details).await?;

  info!("found rounds w/o entries - {:?}", round_ids);

  let mut conn = context.records.acquire().await.map_err(log_and_serialize)?;

  let mut round_ids = query_file!(
    "src/bg/handlers/game_memberships/data-store/create-empty-entries-for-game-member.sql",
    &details.user_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(log_and_serialize)?
  .into_iter()
  .map(|row| row.round_id)
  .collect::<Vec<String>>();

  round_ids.dedup();

  debug!("ids - {:?}", round_ids);

  for id in &round_ids {
    let job =
      interchange::jobs::Job::CheckRoundFulfillment(interchange::jobs::CheckRoundFulfillment {
        round_id: id.clone(),
        result: None,
      });
    info!("queing round completion check job for round {:?}", job);
    context.jobs.queue(&job).await.map_err(|e| {
      warn!("unable to queue round completion job - {}", e);
      format!("{}", e)
    })?;
  }

  Ok(format!("{} auto entries created", round_ids.len()))
}

pub async fn cleanup(details: &CleanupContext, context: &Context) -> interchange::jobs::Job {
  debug!("cleaning up game member '{}'", details.member_id);
  interchange::jobs::Job::CleanupGameMembership(interchange::jobs::CleanupGameMembershipContext {
    result: Some(cleanup_inner(details, context).await),
    ..details.clone()
  })
}

#[cfg(test)]
mod tests {
  use super::round_ids_without_entries;
  use crate::{
    bg::{
      context::Context,
      handlers::lobbies::{make_game as create_game, make_lobby as create_lobby},
      test_helpers::get_test_context,
    },
    interchange,
  };
  use async_std::task::block_on;
  use sqlx::query;

  async fn find_member(context: &Context, user_id: &String, game_id: &String) -> String {
    let mut conn = context.records.acquire().await.expect("no record store");

    query!("select id from krumnet.game_memberships as members where members.user_id = $1 and members.game_id = $2", user_id, game_id)
      .fetch_all(&mut conn)
      .await.expect("cant find member").into_iter().nth(0).map(|r| r.id).expect("unable to find member")
  }

  async fn cleanup_user(context: &Context, user_id: &String) {
    let mut conn = context.records.acquire().await.expect("no record store");
    query!("delete from krumnet.users where id = $1", user_id)
      .execute(&mut conn)
      .await
      .expect("unable to delete");
  }

  async fn cleanup_lobby(context: &Context, lobby_id: &String) {
    let mut conn = context.records.acquire().await.expect("no record store");
    query!(
      "delete from krumnet.lobby_memberships where lobby_id = $1",
      lobby_id
    )
    .execute(&mut conn)
    .await
    .expect("unable to delete");
    query!("delete from krumnet.lobbies where id = $1", lobby_id)
      .execute(&mut conn)
      .await
      .expect("unable to delete");
  }

  async fn cleanup_game(context: &Context, game_id: &String) {
    let mut conn = context.records.acquire().await.expect("no record store");
    query!(
      "delete from krumnet.game_round_entries as entries where entries.game_id = $1",
      game_id
    )
    .execute(&mut conn)
    .await
    .expect("unable to delete game entries");

    query!(
      "delete from krumnet.game_rounds as rounds where rounds.game_id = $1",
      game_id
    )
    .execute(&mut conn)
    .await
    .expect("unable to delete game rounds");

    query!(
      "delete from krumnet.game_memberships as members where members.game_id = $1",
      game_id
    )
    .execute(&mut conn)
    .await
    .expect("unable to delete game members");

    query!(
      "delete from krumnet.games as games where games.id = $1",
      game_id
    )
    .execute(&mut conn)
    .await
    .expect("unable to delete game");
  }

  async fn make_game(context: &Context, user_id: &String, lobby_id: &String) -> String {
    create_game(&context.records, &String::from("job-id"), user_id, lobby_id)
      .await
      .expect("unable to create game")
  }

  async fn make_lobby(context: &Context, user_id: &String) -> String {
    create_lobby(&context.records, user_id, user_id)
      .await
      .expect("unable to create lobby")
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

  async fn make_user(context: &Context, email: &str) -> String {
    let mut conn = context.records.acquire().await.expect("no record store");
    query!(
      "insert into krumnet.users (default_email, name) values ($1, $1) returning id ",
      email
    )
    .fetch_all(&mut conn)
    .await
    .expect("unable to insert user")
    .into_iter()
    .nth(0)
    .map(|row| row.id)
    .expect("missing row id")
  }

  #[test]
  fn invalid_round_ids() {
    block_on(async {
      let context = get_test_context().await;
      let details = interchange::jobs::CleanupGameMembershipContext {
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
      let context = get_test_context().await;
      let uid = make_user(
        &context,
        "krumnet.bg.handlers.game_memeberships.no_games_for_lobby",
      )
      .await;
      let lid = make_lobby(&context, &uid).await;

      let details = interchange::jobs::CleanupGameMembershipContext {
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
      cleanup_lobby(&context, &lid).await;
      cleanup_user(&context, &uid).await;
    });
  }

  #[test]
  fn game_with_empty_rounds() {
    block_on(async {
      let context = get_test_context().await;
      let uid = make_user(
        &context,
        "krumnet.bg.handlers.game_memeberships.test.with_empty_rounds",
      )
      .await;
      let lid = make_lobby(&context, &uid).await;
      let gid = make_game(&context, &uid, &lid).await;

      let details = interchange::jobs::CleanupGameMembershipContext {
        user_id: uid.clone(),
        member_id: String::from("not-exists"),
        lobby_id: lid.clone(),
        game_id: gid.clone(),
        result: None,
      };

      let result = round_ids_without_entries(&context, &details)
        .await
        .expect("failed query");

      assert_eq!(result.len(), 3);
      cleanup_game(&context, &gid).await;
      cleanup_lobby(&context, &lid).await;
      cleanup_user(&context, &uid).await;
    });
  }

  #[test]
  fn game_with_some_entries() {
    block_on(async {
      let context = get_test_context().await;
      let uid = make_user(
        &context,
        "krumnet.bg.handlers.game_memeberships.test.with_some_entries",
      )
      .await;
      let lid = make_lobby(&context, &uid).await;
      let gid = make_game(&context, &uid, &lid).await;
      let mid = find_member(&context, &uid, &gid).await;

      make_entry(&context, &mid, &uid, &gid, 0).await;

      let details = interchange::jobs::CleanupGameMembershipContext {
        user_id: uid.clone(),
        member_id: mid.clone(),
        lobby_id: lid.clone(),
        game_id: gid.clone(),
        result: None,
      };

      let result = round_ids_without_entries(&context, &details)
        .await
        .expect("failed query");

      assert_eq!(result.len(), 2);
      cleanup_game(&context, &gid).await;
      cleanup_lobby(&context, &lid).await;
      cleanup_user(&context, &uid).await;
    });
  }

  #[test]
  fn game_with_most_entries() {
    block_on(async {
      let context = get_test_context().await;
      let uid = make_user(
        &context,
        "krumnet.bg.handlers.game_memeberships.test.with_most_entries",
      )
      .await;
      let lid = make_lobby(&context, &uid).await;
      let gid = make_game(&context, &uid, &lid).await;
      let mid = find_member(&context, &uid, &gid).await;

      make_entry(&context, &mid, &uid, &gid, 0).await;
      make_entry(&context, &mid, &uid, &gid, 1).await;

      let details = interchange::jobs::CleanupGameMembershipContext {
        user_id: uid.clone(),
        member_id: mid.clone(),
        lobby_id: lid.clone(),
        game_id: gid.clone(),
        result: None,
      };

      let result = round_ids_without_entries(&context, &details)
        .await
        .expect("failed query");

      assert_eq!(result.len(), 1);
      cleanup_game(&context, &gid).await;
      cleanup_lobby(&context, &lid).await;
      cleanup_user(&context, &uid).await;
    });
  }

  #[test]
  fn game_with_all_entries() {
    block_on(async {
      let context = get_test_context().await;
      let uid = make_user(
        &context,
        "krumnet.bg.handlers.game_memeberships.test.with_all_entries",
      )
      .await;
      let lid = make_lobby(&context, &uid).await;
      let gid = make_game(&context, &uid, &lid).await;
      let mid = find_member(&context, &uid, &gid).await;

      make_entry(&context, &mid, &uid, &gid, 0).await;
      make_entry(&context, &mid, &uid, &gid, 1).await;
      make_entry(&context, &mid, &uid, &gid, 2).await;

      let details = interchange::jobs::CleanupGameMembershipContext {
        user_id: uid.clone(),
        member_id: mid.clone(),
        lobby_id: lid.clone(),
        game_id: gid.clone(),
        result: None,
      };

      let result = round_ids_without_entries(&context, &details)
        .await
        .expect("failed query");

      assert_eq!(result.len(), 0);
      cleanup_game(&context, &gid).await;
      cleanup_lobby(&context, &lid).await;
      cleanup_user(&context, &uid).await;
    });
  }
}
