use crate::interchange::jobs::{Job, QueuedJob};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "t", content = "c")]
pub enum JobResult {
  NewLobby { id: String },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "t", content = "c")]
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
      Job::CreateLoby { creator: _, result } => {
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
