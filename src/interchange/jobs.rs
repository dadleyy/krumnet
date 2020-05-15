use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct QueuedJob {
  pub id: String,
  pub job: Job,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case", tag = "t", content = "c")]
pub enum Job {
  CreateLoby {
    creator: String,
    result: Option<String>,
  },
}
