use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case", tag = "t", content = "c")]
pub enum Job {
  CreateLoby {
    creator: String,
    result: Option<Result<String, String>>,
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
      Job::CreateLoby { creator, result: _ } => Some(creator.clone()),
    }
  }
}
