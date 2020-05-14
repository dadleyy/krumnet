#[derive(Debug)]
pub enum Authority {
  User(String),
  None,
}

impl Default for Authority {
  fn default() -> Self {
    Authority::None
  }
}
