#[derive(Clone, Debug)]
pub struct GoogleCredentials {
    client_id: String,
    client_secret: String,
}

impl GoogleCredentials {
    pub fn new(id: String, secret: String) -> Self {
        GoogleCredentials {
            client_id: id,
            client_secret: secret,
        }
    }
}
