use crate::interchange::jobs::{Job, QueuedJob};
use serde::Serialize;
use std::time::SystemTime;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LobbyMember {
  pub member_id: String,
  pub user_id: String,
  pub name: String,
  pub email: String,
  pub invited_by: Option<String>,
  pub joined_at: Option<SystemTime>,
  pub left_at: Option<SystemTime>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LobbyDetails {
  pub id: String,
  pub name: String,
  pub members: Vec<LobbyMember>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "data")]
pub enum JobResult {
  NewLobby { id: String },
  NewGame { id: String },
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
