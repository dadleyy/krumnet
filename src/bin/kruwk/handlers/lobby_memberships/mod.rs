use super::Context;
use krumnet::interchange;
use log::{debug, warn};

const CLOSE_LOBBY: &'static str = include_str!("./data-store/close-lobby.sql");
const LEAVE_GAMES: &'static str =
  include_str!("./data-store/leave-game-member-by-lobby-member.sql");
const COUNT_REMAINING_MEMBERS: &'static str =
  include_str!("./data-store/count-remaining-lobby-members.sql");

fn stringify_error<E: std::fmt::Display>(e: E) -> String {
  format!("{}", e)
}

fn count_lobby_members(lobby_id: &String, context: &Context<'_>) -> Result<i64, String> {
  let rows = context
    .records
    .query(COUNT_REMAINING_MEMBERS, &[lobby_id])
    .map_err(stringify_error)?;

  rows
    .iter()
    .nth(0)
    .map(|row| row.try_get::<_, i64>(0).map_err(stringify_error))
    .unwrap_or_else(|| {
      Err(format!(
        "unable to find matching rows for lobby '{}'",
        lobby_id
      ))
    })
}

pub async fn cleanup_inner(
  member_id: &String,
  lobby_id: &String,
  context: &Context<'_>,
) -> Result<String, String> {
  let count = count_lobby_members(lobby_id, context)?;

  debug!("lobby '{}' has {} remaining members", lobby_id, count);

  let rows = context
    .records
    .query(LEAVE_GAMES, &[member_id])
    .map_err(stringify_error)?;

  let left_games = rows
    .iter()
    .map(|row| {
      let game_id = row.try_get::<_, String>(0).map_err(stringify_error)?;
      let lobby_id = row.try_get::<_, String>(1).map_err(stringify_error)?;
      let game_member_id = row.try_get::<_, String>(2).map_err(stringify_error)?;
      let lobby_member_id = row.try_get::<_, String>(3).map_err(stringify_error)?;
      let user_id = row.try_get::<_, String>(4).map_err(stringify_error)?;
      Ok((game_id, lobby_id, game_member_id, lobby_member_id, user_id))
    })
    .collect::<Result<Vec<(String, String, String, String, String)>, String>>()?;

  for (game_id, lobby_id, game_member_id, _, user_id) in &left_games {
    let details = interchange::jobs::CleanupGameMembershipContext {
      user_id: user_id.clone(),
      game_id: game_id.clone(),
      member_id: game_member_id.clone(),
      lobby_id: lobby_id.clone(),
      result: None,
    };
    let game_member_cleanup = interchange::jobs::Job::CleanupGameMembership(details);
    debug!(
      "adding game membership cleanup job to queue - {:?}",
      game_member_cleanup
    );
    context
      .jobs
      .queue(&game_member_cleanup)
      .await
      .map_err(stringify_error)?;
  }

  debug!("left {} games", left_games.len());

  if count == 0 {
    debug!("lobby had no remaining members, closing games and lobby");
    context
      .records
      .query(CLOSE_LOBBY, &[lobby_id])
      .map_err(stringify_error)?;
  }

  Ok(String::from("done"))
}

pub async fn cleanup(
  job_id: &String,
  member_id: &String,
  lobby_id: &String,
  context: &Context<'_>,
) -> interchange::jobs::QueuedJob {
  debug!("job '{}', cleanup '{}'", job_id, member_id);

  let res = cleanup_inner(member_id, lobby_id, context)
    .await
    .map_err(|err| {
      warn!("unable to cleanup - {}", err);
      err
    });

  interchange::jobs::QueuedJob {
    id: job_id.clone(),
    job: interchange::jobs::Job::CleanupLobbyMembership {
      member_id: member_id.clone(),
      lobby_id: lobby_id.clone(),
      result: Some(res),
    },
  }
}
