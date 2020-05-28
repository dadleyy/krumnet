use krumnet::{
  interchange::jobs::{Job, QueuedJob},
  names,
  records::Row,
  RecordStore,
};
use log::{debug, info, warn};

fn parse_user(row: &Row) -> Option<UserInfo> {
  let id = row.try_get(0).ok()?;
  let name = row.try_get(1).ok()?;
  let email = row.try_get(2).ok()?;
  Some(UserInfo { id, email, name })
}

#[derive(Debug)]
struct UserInfo {
  id: String,
  name: String,
  email: String,
}

const FIND_USER: &'static str = include_str!("./data-store/find-user-by-id.sql");
const CREATE_LOBBY: &'static str = include_str!("./data-store/create-lobby.sql");
const LOAD_LOBBY_DETAILS: &'static str = include_str!("./data-store/load-lobby-details-by-id.sql");
const CREATE_GAME_FOR_LOBBY: &'static str = include_str!("./data-store/create-game-for-lobby.sql");
const CREATE_MEMBERSHIPS_FOR_GAME: &'static str =
  include_str!("./data-store/create-game-members.sql");

fn make_lobby(
  records: &RecordStore,
  job_id: &String,
  creator: &String,
) -> std::result::Result<String, String> {
  let mask = bit_vec::BitVec::from_elem(10, false);
  let name = names::get();

  let rows = records
    .query(FIND_USER, &[creator])
    .map_err(|_e| String::from("unable to query users for creator"))?;

  let user = rows
    .iter()
    .nth(0)
    .and_then(parse_user)
    .ok_or(String::from("unable to find user"))?;

  let rows = records
    .query(CREATE_LOBBY, &[job_id, &name, &mask, &user.id])
    .map_err(|e| {
      warn!("unable to create lobby - {}", e);
      String::from("unable to create")
    })?;

  rows
    .iter()
    .nth(0)
    .and_then(|row| row.try_get::<_, String>(0).ok())
    .ok_or(String::from("unable to parse as string"))
}

pub async fn create_lobby(job_id: &String, creator: &String, records: &RecordStore) -> QueuedJob {
  let result = make_lobby(records, job_id, creator);

  QueuedJob {
    id: job_id.clone(),
    job: Job::CreateLobby {
      result: Some(result),
      creator: creator.clone(),
    },
  }
}

async fn make_game(
  records: &RecordStore,
  job_id: &String,
  creator: &String,
  lobby_id: &String,
) -> std::result::Result<String, String> {
  let user = records
    .query(FIND_USER, &[creator])
    .map_err(|_e| String::from("unable to query users for creator"))?
    .iter()
    .nth(0)
    .and_then(parse_user)
    .ok_or(String::from("unable to find user"))?;

  let lid = records
    .query(LOAD_LOBBY_DETAILS, &[lobby_id, creator])
    .map_err(|_e| String::from("unable to query users for creator"))?
    .iter()
    .nth(0)
    .and_then(|row| {
      debug!("found matching lobby, everything is ok");
      row.try_get::<_, String>(0).ok()
    })
    .ok_or(String::from("unable to find lobby"))?;

  debug!("creating game for lobby '{}' (user '{}')", lid, user.email);
  let name = names::get();

  let gid = records
    .query(CREATE_GAME_FOR_LOBBY, &[&lid, &name, job_id])
    .map_err(|e| {
      warn!("create lobby query failed - {}", e);
      String::from("unable to query users for creator")
    })?
    .iter()
    .nth(0)
    .and_then(|row| {
      debug!("game created successfull, pulled first row from rounds");
      row.try_get::<_, String>(0).ok()
    })
    .ok_or(String::from("failed game creation"))?;

  info!("game '{}' created for lobby '{}'", gid, lid);

  records
    .query(CREATE_MEMBERSHIPS_FOR_GAME, &[&gid, &lid])
    .map_err(|e| {
      warn!("game membership creation failed - {}", e);
      String::from("unable to query users for creator")
    })?;

  Ok(String::from(gid))
}

pub async fn create_game(
  job_id: &String,
  creator: &String,
  lobby_id: &String,
  records: &RecordStore,
) -> QueuedJob {
  let result = make_game(records, job_id, creator, lobby_id).await;

  QueuedJob {
    id: job_id.clone(),
    job: Job::CreateGame {
      result: Some(result),
      lobby_id: lobby_id.clone(),
      creator: creator.clone(),
    },
  }
}
