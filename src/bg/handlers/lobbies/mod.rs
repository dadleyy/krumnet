use crate::{
  interchange::jobs::{CreateGame, CreateLobby, Job},
  names, RecordStore,
};
use log::{debug, info, warn};
use sqlx::query_file;

fn warn_and_stringify<E: std::fmt::Display>(err: E) -> String {
  warn!("{}", err);
  format!("{}", err)
}

#[derive(Debug)]
struct UserInfo {
  id: String,
  name: String,
  email: String,
}

async fn find_user(user_id: &String, records: &RecordStore) -> Result<UserInfo, String> {
  let mut conn = records.acquire().await.map_err(warn_and_stringify)?;
  query_file!(
    "src/bg/handlers/lobbies/data-store/find-user-by-id.sql",
    user_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(warn_and_stringify)?
  .into_iter()
  .nth(0)
  .map(|row| {
    Ok(UserInfo {
      id: row.id,
      name: row.name,
      email: row.email,
    })
  })
  .unwrap_or(Err(format!("Unable to find user '{}'", user_id)))
}

pub async fn make_lobby(
  records: &RecordStore,
  job_id: &String,
  creator: &String,
) -> std::result::Result<String, String> {
  let name = names::get();
  let user = find_user(creator, records).await?;
  let mut conn = records.acquire().await.map_err(warn_and_stringify)?;

  query_file!(
    "src/bg/handlers/lobbies/data-store/create-lobby.sql",
    job_id,
    name,
    user.id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(warn_and_stringify)?
  .into_iter()
  .nth(0)
  .map(|row| Ok(row.lobby_id))
  .unwrap_or(Err(format!("Lobby creation failed for job '{}'", job_id)))
}

pub async fn create_lobby(job_id: &String, details: &CreateLobby, records: &RecordStore) -> Job {
  let result = make_lobby(records, job_id, &details.creator).await;

  Job::CreateLobby(CreateLobby {
    result: Some(result),
    creator: details.creator.clone(),
  })
}

async fn make_game(
  records: &RecordStore,
  job_id: &String,
  creator: &String,
  lobby_id: &String,
) -> std::result::Result<String, String> {
  let user = find_user(creator, records).await?;
  debug!(
    "creating game for lobby '{}' (user '{}')",
    lobby_id, user.email
  );
  let name = names::get();

  let mut conn = records.acquire().await.map_err(warn_and_stringify)?;

  let gid = query_file!(
    "src/bg/handlers/lobbies/data-store/create-game-for-lobby.sql",
    lobby_id,
    name,
    job_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(warn_and_stringify)?
  .into_iter()
  .nth(0)
  .map(|row| row.game_id)
  .ok_or(format!("Unable to create game for lobby '{}'", lobby_id))?;

  info!("game '{}' created for lobby '{}'", gid, lobby_id);

  query_file!(
    "src/bg/handlers/lobbies/data-store/create-game-members.sql",
    gid,
    lobby_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(warn_and_stringify)?;

  Ok(String::from(gid))
}

pub async fn create_game(job_id: &String, details: &CreateGame, records: &RecordStore) -> Job {
  let result = make_game(records, job_id, &details.creator, &details.lobby_id).await;

  Job::CreateGame(CreateGame {
    result: Some(result),
    lobby_id: details.lobby_id.clone(),
    creator: details.creator.clone(),
  })
}
