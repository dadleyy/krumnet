use super::Context;
use krumnet::interchange;
use log::{debug, info, warn};
use sqlx::query_file;

fn stringify_error<E: std::fmt::Display>(e: E) -> String {
  warn!("lobby membership error - {}", e);
  format!("{}", e)
}

async fn count_lobby_members(lobby_id: &String, context: &Context<'_>) -> Result<i64, String> {
  let mut conn = context.records.acquire().await.map_err(stringify_error)?;
  query_file!(
    "src/bin/kruwk/handlers/lobby_memberships/data-store/count-remaining-lobby-members.sql",
    lobby_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(stringify_error)?
  .into_iter()
  .nth(0)
  .and_then(|row| {
    debug!("found lobby member count for lobby '{}': {:?}", lobby_id, row);
    row.count.map(Ok)
  })
  .unwrap_or(Err(format!("Unable to find matching lobbies for '{}'", lobby_id)))
}

struct LeftGame {
  game_id: String,
  user_id: String,
  lobby_id: String,
  game_member_id: String,
}

async fn leave_games(lobby_member_id: &String, context: &Context<'_>) -> Result<Vec<LeftGame>, String> {
  let mut conn = context.records.acquire().await.map_err(stringify_error)?;
  query_file!(
    "src/bin/kruwk/handlers/lobby_memberships/data-store/leave-game-member-by-lobby-member.sql",
    lobby_member_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(stringify_error)?
  .into_iter()
  .map(|row| {
    Ok(LeftGame {
      game_id: row.game_id,
      user_id: row.user_id,
      lobby_id: row.lobby_id,
      game_member_id: row.game_member_id,
    })
  })
  .collect::<Result<Vec<LeftGame>, String>>()
}

async fn close_lobby(lobby_id: &String, context: &Context<'_>) -> Result<String, String> {
  let mut conn = context.records.acquire().await.map_err(stringify_error)?;
  query_file!(
    "src/bin/kruwk/handlers/lobby_memberships/data-store/close-lobby.sql",
    lobby_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(stringify_error)?
  .into_iter()
  .nth(0)
  .map(|row| Ok(row.id))
  .unwrap_or(Err(format!("Unable to set closed timestamp for lobby '{}'", lobby_id)))
}

pub async fn cleanup_inner(member_id: &String, lobby_id: &String, context: &Context<'_>) -> Result<String, String> {
  let count = count_lobby_members(lobby_id, context).await?;
  let left_games = leave_games(member_id, context).await?;

  let jobs = left_games.iter().map(|g| {
    let details = interchange::jobs::CleanupGameMembershipContext {
      user_id: g.user_id.clone(),
      game_id: g.game_id.clone(),
      member_id: g.game_member_id.clone(),
      lobby_id: g.lobby_id.clone(),
      result: None,
    };
    interchange::jobs::Job::CleanupGameMembership(details)
  });

  for job in jobs {
    debug!("adding game membership cleanup job to queue - {:?}", job);
    context.jobs.queue(&job).await.map_err(stringify_error)?;
  }

  if count == 0 {
    info!("lobby '{}' had no remaining members, closing games and lobby", lobby_id);
    return close_lobby(lobby_id, context).await;
  }

  info!("lobby '{}' has {} remaining members", lobby_id, count);
  Ok(String::from("done"))
}

pub async fn cleanup(
  job_id: &String,
  details: &interchange::jobs::CleanupLobbyMembership,
  context: &Context<'_>,
) -> interchange::jobs::Job {
  debug!("job '{}', cleanup '{}'", job_id, details.member_id);

  let res = cleanup_inner(&details.member_id, &details.lobby_id, context)
    .await
    .map_err(|err| {
      warn!("unable to cleanup - {}", err);
      err
    });

  interchange::jobs::Job::CleanupLobbyMembership(interchange::jobs::CleanupLobbyMembership {
    member_id: details.member_id.clone(),
    lobby_id: details.lobby_id.clone(),
    result: Some(res),
  })
}
