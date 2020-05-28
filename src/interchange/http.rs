use crate::interchange::jobs::{Job, QueuedJob};
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct NewLobbyMembership {
  pub member_id: String,
  pub user_id: String,
  pub lobby_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GameRoundEntry {
  pub id: String,
  pub member_id: String,
  pub round_id: String,
  pub entry: Option<String>,
  #[serde(with = "chrono::serde::ts_milliseconds")]
  pub created: DateTime<Utc>,
  pub user_id: String,
  pub user_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GameRoundDetails {
  pub id: String,
  pub entries: Vec<GameRoundEntry>,
  pub prompt: Option<String>,
  pub position: u32,
  #[serde(with = "chrono::serde::ts_milliseconds_option")]
  pub started: Option<DateTime<Utc>>,
  #[serde(with = "chrono::serde::ts_milliseconds")]
  pub created: DateTime<Utc>,
  #[serde(with = "chrono::serde::ts_milliseconds_option")]
  pub completed: Option<DateTime<Utc>>,
  #[serde(with = "chrono::serde::ts_milliseconds_option")]
  pub fulfilled: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LobbyListLobby {
  pub id: String,
  pub name: String,
  #[serde(with = "chrono::serde::ts_milliseconds")]
  pub created: DateTime<Utc>,
  pub game_count: i64,
  pub member_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LobbyList {
  pub lobbies: Vec<LobbyListLobby>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GameMember {
  pub member_id: String,
  pub user_id: String,
  pub email: String,
  pub name: String,
  #[serde(with = "chrono::serde::ts_milliseconds")]
  pub joined: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GameRound {
  pub id: String,
  pub position: u32,
  pub prompt: Option<String>,
  #[serde(with = "chrono::serde::ts_milliseconds_option")]
  pub started: Option<DateTime<Utc>>,
  #[serde(with = "chrono::serde::ts_milliseconds")]
  pub created: DateTime<Utc>,
  #[serde(with = "chrono::serde::ts_milliseconds_option")]
  pub completed: Option<DateTime<Utc>>,
  #[serde(with = "chrono::serde::ts_milliseconds_option")]
  pub fulfilled: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GameDetails {
  pub id: String,
  pub name: String,
  #[serde(with = "chrono::serde::ts_milliseconds")]
  pub created: DateTime<Utc>,
  pub members: Vec<GameMember>,
  pub rounds: Vec<GameRound>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LobbyMember {
  pub member_id: String,
  pub user_id: String,
  pub name: String,
  pub email: String,
  pub invited_by: Option<String>,
  #[serde(with = "chrono::serde::ts_milliseconds_option")]
  pub joined_at: Option<DateTime<Utc>>,
  #[serde(with = "chrono::serde::ts_milliseconds_option")]
  pub left_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LobbyGame {
  pub id: String,
  pub name: String,
  pub rounds_remaining: i64,
  #[serde(with = "chrono::serde::ts_milliseconds")]
  pub created: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LobbyDetails {
  pub id: String,
  pub name: String,
  pub members: Vec<LobbyMember>,
  pub games: Vec<LobbyGame>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "data")]
pub enum JobResult {
  NewLobby { id: String },
  NewGame { id: String },
  Nothing,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "data")]
pub enum WrappedJobResult {
  Success(JobResult),
  Failure(String),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct JobHandle {
  pub id: String,
  pub result: Option<WrappedJobResult>,
}

fn without_result(id: String) -> JobHandle {
  JobHandle {
    id,
    result: Some(WrappedJobResult::Success(JobResult::Nothing)),
  }
}

impl From<QueuedJob> for JobHandle {
  fn from(job: QueuedJob) -> Self {
    let id = job.id.clone();

    match job.job {
      Job::CreateGame { result, .. } => {
        let result = result.map(|res| match res {
          Ok(id) => WrappedJobResult::Success(JobResult::NewGame { id }),
          Err(e) => WrappedJobResult::Failure(e),
        });
        JobHandle { id, result }
      }
      Job::CreateLobby { creator: _, result } => {
        let result = result.map(|res| match res {
          Ok(id) => WrappedJobResult::Success(JobResult::NewLobby { id }),
          Err(e) => WrappedJobResult::Failure(e),
        });
        JobHandle { id, result }
      }
      Job::CheckRoundFulfillment { .. }
      | Job::CleanupLobbyMembership { .. }
      | Job::CleanupGameMembership { .. } => without_result(id),
    }
  }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SessionUserData {
  pub id: String,
  pub email: String,
  pub name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SessionData {
  pub user: SessionUserData,
}
