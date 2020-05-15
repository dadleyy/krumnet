#[derive(Debug, PartialEq)]
pub enum Authority {
  User { id: String, token: String },
  None,
}

impl Default for Authority {
  fn default() -> Self {
    Authority::None
  }
}
