use crate::bg::context::Context;
use log::warn;
use sqlx::query_file;

fn warn_and_stringify<E: std::error::Error>(e: E) -> String {
  warn!("{}", e);
  format!("{}", e)
}

pub async fn count_entries(context: &Context, round_id: &String) -> Result<i64, String> {
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
pub async fn count_members(context: &Context, round_id: &String) -> Result<i64, String> {
  let mut conn = context
    .records
    .acquire()
    .await
    .map_err(warn_and_stringify)?;

  query_file!(
    "src/bg/handlers/rounds/data-store/count-members-for-round.sql",
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

#[cfg(test)]
mod tests {
  use super::count_members;
  use crate::bg::{context::Context, test_helpers};
  use async_std::task::block_on;
  use sqlx::query;

  struct TestContext {
    lobby_id: String,
    game_id: String,
    user_id: String,
  }

  async fn context_and_game(user_id: &str) -> (Context, TestContext) {
    let (context, user_id) = test_helpers::get_test_context_with_user(user_id).await;
    let lobby_id = test_helpers::make_lobby(&context, &user_id).await;
    let game_id = test_helpers::make_game(&context, &user_id, &lobby_id).await;
    (
      context,
      TestContext {
        user_id,
        game_id,
        lobby_id,
      },
    )
  }

  async fn round_for_game(context: &Context, game_id: &String, position: i32) -> String {
    let mut conn = context.records.acquire().await.expect("unable to connect");

    let q = query!(
        "select rounds.id as round_id from krumnet.game_rounds as rounds where rounds.game_id = $1 and rounds.position = $2",
        game_id,
        position
    );

    q.fetch_all(&mut conn)
      .await
      .expect("unable to query")
      .into_iter()
      .nth(0)
      .map(|row| row.round_id)
      .expect("unable tof find")
  }

  #[test]
  fn err_when_missing() {
    block_on(async {
      let context = test_helpers::get_test_context().await;
      let result = count_members(&context, &String::from("bogus")).await;
      assert_eq!(result.is_err(), true);
      assert_eq!(
        result.unwrap_err(),
        String::from("Unable to count members for round 'bogus'")
      );
    });
  }

  #[test]
  fn count_when_present() {
    block_on(async {
      let (context, deets) = context_and_game("bg.rounds.utils.count_when_present").await;
      let oid = test_helpers::make_user(&context, "other user").await;
      let round_id = round_for_game(&context, &deets.game_id, 0).await;
      let result = count_members(&context, &round_id).await;
      assert_eq!(result.unwrap(), 1);
      test_helpers::cleanup_game(&context, &deets.game_id).await;
      test_helpers::cleanup_lobby(&context, &deets.lobby_id).await;
      test_helpers::cleanup_user(&context, &deets.user_id).await;
      test_helpers::cleanup_user(&context, &oid).await;
    });
  }
}
