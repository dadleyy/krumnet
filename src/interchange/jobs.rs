use serde::{Deserialize, Serialize};
use std::time::SystemTime;

// Queued when a user leaves a lobby or game explicitly, jobs of this kind will attempt to create
// game round entries for any rounds that do not already have one. On success, the job's result
// will be populated with an array of round ids that were filled.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct CleanupGameMembership {
  pub user_id: String,
  pub member_id: String,
  pub lobby_id: String,
  pub game_id: String,
  pub result: Option<Result<Vec<String>, String>>,
}

// Queued when a round entry is created, jobs of this kind will attempt to count how many entries
// are _missing_ from the round. If that number is 0, the job will set the fulfilled date on the
// round and attempt to start the next.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct CheckRoundFulfillment {
  pub round_id: String,
  pub result: Option<Result<u8, String>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct CheckRoundCompletion {
  pub round_id: String,
  pub game_id: String,
  pub result: Option<Result<Option<String>, String>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct CreateGame {
  pub creator: String,
  pub lobby_id: String,
  pub result: Option<Result<String, String>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct CleanupLobbyMembership {
  pub member_id: String,
  pub lobby_id: String,
  pub result: Option<Result<String, String>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct CreateLobby {
  pub creator: String,
  pub result: Option<Result<String, String>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case", tag = "t", content = "c")]
pub enum Job {
  CreateLobby(CreateLobby),
  CheckRoundFulfillment(CheckRoundFulfillment),
  CreateGame(CreateGame),
  CleanupLobbyMembership(CleanupLobbyMembership),
  CheckRoundCompletion(CheckRoundCompletion),
  CleanupGameMembership(CleanupGameMembership),
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
      Job::CreateLobby(CreateLobby { creator, .. }) => Some(creator.clone()),
      Job::CreateGame(CreateGame { creator, .. }) => Some(creator.clone()),
      Job::CheckRoundFulfillment { .. }
      | Job::CleanupLobbyMembership { .. }
      | Job::CheckRoundCompletion(_)
      | Job::CleanupGameMembership(_) => None,
    }
  }
}
