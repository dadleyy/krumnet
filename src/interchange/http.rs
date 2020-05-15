use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct JobHandle {
  pub id: String,
}

#[derive(Debug, Serialize)]
pub struct SessionUserData {
  pub id: String,
  pub email: String,
  pub name: String,
}

#[derive(Debug, Serialize)]
pub struct SessionData {
  pub user: SessionUserData,
}
