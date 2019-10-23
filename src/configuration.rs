#[derive(Clone, Debug)]
pub struct Configuration {
  pub google: GoogleCredentials,
}

#[derive(Clone, Debug)]
pub struct GoogleCredentials {
  pub client_id: String,
  pub client_secret: String,
  pub redirect_uri: String,
}

impl GoogleCredentials {
  pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
    GoogleCredentials {
      client_id,
      client_secret,
      redirect_uri,
    }
  }
}
