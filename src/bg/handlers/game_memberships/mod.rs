use log::{debug, info, warn};
use sqlx::query_file;

use crate::interchange::jobs::CleanupGameMembershipContext as CleanupContext;
use crate::{bg::context::Context, errors, interchange};

fn log_and_humanize<E>(error: E) -> std::io::Error
where
  E: std::error::Error,
{
  warn!("{}", error);
  errors::humanize_error(error)
}

async fn round_ids_without_entries(
  context: &Context,
  details: &CleanupContext,
) -> Result<Vec<String>, std::io::Error> {
  let mut conn = context.records.acquire().await.map_err(log_and_humanize)?;
  query_file!(
    "src/bg/handlers/game_memberships/data-store/get-round-ids.sql",
    details.user_id,
    details.game_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(log_and_humanize)
  .map(|result| result.into_iter().map(|row| row.round_id).collect())
}

pub async fn cleanup_inner(details: &CleanupContext, context: &Context) -> Result<String, String> {
  let mut conn = context
    .records
    .acquire()
    .await
    .map_err(log_and_humanize)
    .map_err(|e| format!("{}", e))?;

  let mut round_ids = query_file!(
    "src/bg/handlers/game_memberships/data-store/create-empty-entries-for-game-member.sql",
    &details.user_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(|e| {
    warn!("unable to create empty entries - {}", e);
    format!("{}", e)
  })?
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
      context::Context, handlers::lobbies::make_lobby as create_lobby,
      test_helpers::get_test_context,
    },
    interchange,
  };
  use async_std::task::block_on;
  use sqlx::query;

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

  async fn make_lobby(context: &Context, user_id: &String) -> String {
    create_lobby(&context.records, user_id, user_id)
      .await
      .expect("unable to create lobby")
  }

  async fn make_user(context: &Context) -> String {
    let mut conn = context.records.acquire().await.expect("no record store");
    query!(
        "
          insert into
              krumnet.users (default_email, name)
          values
              ('bg.handlers.game_memberships.no_missing_entries', 'bg.handlers.game_memberships.no_missing_entries')
          returning id
          "
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
  fn no_missing_entries() {
    block_on(async {
      let context = get_test_context().await;
      let uid = make_user(&context).await;
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
}
