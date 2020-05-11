use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SessionPayload {
  pub id: String,
  pub email: String,
  pub name: String,
}
