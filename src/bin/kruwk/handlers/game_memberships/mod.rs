use super::Context;
use log::{debug, info, warn};
use sqlx::query_file;

use krumnet::interchange;
use krumnet::interchange::jobs::CleanupGameMembershipContext;

pub async fn cleanup_inner(
  details: &CleanupGameMembershipContext,
  context: &Context<'_>,
) -> Result<String, String> {
  let mut conn = context.records.q().await.map_err(|e| {
    warn!("unable to aquire database connection - {}", e);
    format!("{}", e)
  })?;

  let mut round_ids = query_file!(
    "src/bin/kruwk/handlers/game_memberships/data-store/create-empty-entries-for-game-member.sql",
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
    let job = interchange::jobs::Job::CheckRoundFulfillment {
      round_id: id.clone(),
      result: None,
    };
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
  context: &Context<'_>,
) -> interchange::jobs::Job {
  debug!("cleaning up game member '{}'", details.member_id);
  interchange::jobs::Job::CleanupGameMembership(interchange::jobs::CleanupGameMembershipContext {
    result: Some(cleanup_inner(details, context).await),
    ..details.clone()
  })
}
