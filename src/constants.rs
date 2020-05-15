pub const MAX_FILE_SIZE: usize = 1000000usize;

pub const GOOGLE_TOKEN_URL: &'static str = "https://www.googleapis.com/oauth2/v4/token";
pub const GOOGLE_AUTH_URL: &'static str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const GOOGLE_INFO_URL: &'static str = "https://openidconnect.googleapis.com/v1/userinfo";

pub const GOOGLE_AUTH_RESPONSE_TYPE_KEY: &'static str = "response_type";
pub const GOOGLE_AUTH_RESPONSE_TYPE_VALUE: &'static str = "code";
pub const GOOGLE_AUTH_CLIENT_ID_KEY: &'static str = "client_id";
pub const GOOGLE_AUTH_REDIRECT_URI_KEY: &'static str = "redirect_uri";
pub const GOOGLE_AUTH_SCOPE_KEY: &'static str = "scope";
pub const GOOGLE_AUTH_SCOPE_VALUE: &'static str = "email profile";

pub const KRUMI_SESSION_ID_KEY: &'static str = "session_id";

#[cfg(not(test))]
pub fn google_info_url() -> String {
  String::from(GOOGLE_INFO_URL)
}
#[cfg(test)]
pub fn google_info_url() -> String {
  String::from(&mockito::server_url())
}

#[cfg(not(test))]
pub fn google_auth_url() -> String {
  String::from(GOOGLE_AUTH_URL)
}
#[cfg(test)]
pub fn google_auth_url() -> String {
  String::from(&mockito::server_url())
}

#[cfg(not(test))]
pub fn google_token_url() -> String {
  String::from(GOOGLE_TOKEN_URL)
}
#[cfg(test)]
pub fn google_token_url() -> String {
  String::from(&mockito::server_url())
}
