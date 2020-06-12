pub mod context;
pub mod handlers;

#[cfg(test)]
pub mod test_helpers {
  use crate::{
    bg::{
      context::Context,
      handlers::lobbies::{make_game as create_game, make_lobby as create_lobby},
    },
    configuration::test_helpers::load_test_config,
    JobStore, RecordStore,
  };
  use async_std::sync::Arc;
  use sqlx::query;

  pub async fn cleanup_user(context: &Context, user_id: &String) {
    let mut conn = context.records.acquire().await.expect("no record store");
    query!("delete from krumnet.users where id = $1", user_id)
      .execute(&mut conn)
      .await
      .expect("unable to delete");
  }

  pub async fn cleanup_lobby(context: &Context, lobby_id: &String) {
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

  pub async fn cleanup_game(context: &Context, game_id: &String) {
    let mut conn = context.records.acquire().await.expect("no record store");

    query!(
      "delete from krumnet.game_member_round_placement_results as results where results.game_id = $1",
      game_id
    )
    .execute(&mut conn)
    .await
    .expect("unable to delete game votes");

    query!(
      "delete from krumnet.game_member_placement_results as results where results.game_id = $1",
      game_id
    )
    .execute(&mut conn)
    .await
    .expect("unable to delete game votes");

    query!(
      "delete from krumnet.game_round_entry_votes as votes where votes.game_id = $1",
      game_id
    )
    .execute(&mut conn)
    .await
    .expect("unable to delete game votes");

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

  pub async fn make_game(context: &Context, user_id: &String, lobby_id: &String) -> String {
    create_game(&context.records, &String::from("job-id"), user_id, lobby_id)
      .await
      .expect("unable to create game")
  }

  pub async fn make_lobby(context: &Context, user_id: &String) -> String {
    create_lobby(&context.records, user_id, user_id)
      .await
      .expect("unable to create lobby")
  }

  pub async fn make_user(context: &Context, id: &str) -> String {
    let mut conn = context.records.acquire().await.expect("no record store");
    query!(
      "insert into krumnet.users (default_email, name) values ($1, $1) returning id ",
      id
    )
    .fetch_all(&mut conn)
    .await
    .expect("unable to insert user")
    .into_iter()
    .nth(0)
    .map(|row| row.id)
    .expect("missing row id")
  }

  pub async fn get_test_context() -> Context {
    let config = load_test_config().expect("unable to load test config");

    let records = RecordStore::open(&config)
      .await
      .expect("unable to open record store");

    let jobs = JobStore::open(&config)
      .await
      .expect("unable to open job store");

    Context {
      records: Arc::new(records),
      jobs: Arc::new(jobs),
    }
  }

  pub async fn get_test_context_with_user(id: &str) -> (Context, String) {
    let context = get_test_context().await;
    let user_id = make_user(&context, id).await;
    (context, user_id)
  }
}
