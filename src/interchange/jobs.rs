use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case", tag = "t", content = "c")]
pub enum Job {
  CreateLobby {
    creator: String,
    result: Option<Result<String, String>>,
  },
  CreateGame {
    creator: String,
    lobby_id: String,
    result: Option<Result<String, String>>,
  },
  CheckRoundCompletion {
    round_id: String,
    result: Option<Result<u8, String>>,
  },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct DequeuedJob {
  pub id: String,
  pub time: SystemTime,
}

impl DequeuedJob {
  pub fn new(id: &String) -> Self {
    DequeuedJob {
      id: id.clone(),
      time: SystemTime::now(),
    }
  }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct QueuedJob {
  pub id: String,
  pub job: Job,
}

impl QueuedJob {
  pub fn user(&self) -> Option<String> {
    match &self.job {
      Job::CreateLobby { creator, result: _ } => Some(creator.clone()),
      Job::CreateGame { creator, .. } => Some(creator.clone()),
      Job::CheckRoundCompletion { .. } => None,
    }
  }
}
