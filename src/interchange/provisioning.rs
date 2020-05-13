use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum ProvisioningAttemptAuthority {
  User { id: String },
  System,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum ProvisioningAttempt {
  Lobby {
    authority: ProvisioningAttemptAuthority,
  },
}
