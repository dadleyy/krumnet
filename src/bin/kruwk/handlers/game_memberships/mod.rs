use super::Context;
use log::{debug, info, warn};

use krumnet::interchange;
use krumnet::interchange::jobs::CleanupGameMembershipContext;

const SUBMIT_EMPTY_ENTRIES: &'static str =
  include_str!("./data-store/create-empty-entries-for-game-member.sql");

pub async fn cleanup_inner(
  details: &CleanupGameMembershipContext,
  context: &Context<'_>,
) -> Result<String, String> {
  let rows = context
    .records
    .query(SUBMIT_EMPTY_ENTRIES, &[&details.user_id])
    .map_err(|e| {
      warn!("unable to create empty entries - {}", e);
      format!("{}", e)
    })?;

  let mut round_ids = rows
    .iter()
    .map(|row| {
      row.try_get::<_, String>(2).map_err(|e| {
        warn!("unable to parse game id for auto entry {}", e);
        format!("{}", e)
      })
    })
    .collect::<Result<Vec<String>, String>>()?;

  round_ids.dedup();

  for id in &round_ids {
    let job = interchange::jobs::Job::CheckRoundCompletion {
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
