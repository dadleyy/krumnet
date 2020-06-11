use log::{debug, info, warn};
use sqlx::query_file;

use crate::interchange::jobs::CleanupGameMembershipContext;
use crate::{bg::context::Context, interchange};

pub async fn cleanup_inner(
  details: &CleanupGameMembershipContext,
  context: &Context,
) -> Result<String, String> {
  let mut conn = context.records.acquire().await.map_err(|e| {
    warn!("unable to aquire database connection - {}", e);
    format!("{}", e)
  })?;

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

pub async fn cleanup(
  details: &CleanupGameMembershipContext,
  context: &Context,
) -> interchange::jobs::Job {
  debug!("cleaning up game member '{}'", details.member_id);
  interchange::jobs::Job::CleanupGameMembership(interchange::jobs::CleanupGameMembershipContext {
    result: Some(cleanup_inner(details, context).await),
    ..details.clone()
  })
}

#[cfg(test)]
mod tests {
  use crate::bg::test_helpers::get_test_context;
  use async_std::task::block_on;

  #[test]
  fn create_empty_entries() {
    block_on(async {
      let ctx = get_test_context().await;
    });
  }
}
