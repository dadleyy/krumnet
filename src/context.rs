use elaine::Head;
use log::info;
use std::io::Result;

use crate::{Authorization, AuthorizationUrls, RecordStore, SessionStore};

const USER_FOR_SESSION: &'static str = include_str!("data-store/user-for-session.sql");

pub trait SessionInterface: std::ops::Deref<Target = SessionStore> {}
impl<T> SessionInterface for T where T: std::ops::Deref<Target = SessionStore> {}

pub trait RecordInterface: std::ops::Deref<Target = RecordStore> {}
impl<T> RecordInterface for T where T: std::ops::Deref<Target = RecordStore> {}

pub struct Context<'a> {
  session: &'a SessionStore,
  records: &'a RecordStore,
  urls: &'a AuthorizationUrls,
  auth: Option<Authorization>,
}

impl<'a> Context<'a> {
  pub fn session(&'a self) -> &'a SessionStore {
    self.session
  }

  pub fn records(&'a self) -> &'a RecordStore {
    self.records
  }

  pub fn auth(&'a self) -> &'a Option<Authorization> {
    &self.auth
  }

  pub fn urls(&self) -> &'a AuthorizationUrls {
    self.urls
  }
}

// Attempts to exchange an authorization token for a user id from the session store, subsequently
// loading the actual user information from the record store.
pub async fn load_authorization<S: SessionInterface, R: RecordInterface>(
  token: String,
  session: S,
  records: R,
) -> Result<Option<Authorization>> {
  let uid = session.get(&token).await?;
  let tenant = records
    .query(USER_FOR_SESSION, &[&uid])?
    .iter()
    .nth(0)
    .and_then(|row| {
      let id = row.try_get::<_, String>(0).ok()?;
      let name = row.try_get::<_, String>(1).ok()?;
      let email = row.try_get::<_, String>(2).ok()?;
      info!("found user '{:?}' {:?} {:?}", id, name, email);
      Some(Authorization(id, name, email, token))
    });

  info!("loaded tenant from auth header: {:?}", tenant);
  Ok(tenant)
}

pub async fn for_request<'a, S, R, A>(
  session: S,
  records: R,
  urls: A,
  head: Head,
) -> Result<Context<'a>>
where
  S: std::ops::Deref<Target = SessionStore>,
  R: std::ops::Deref<Target = RecordStore>,
  A: std::ops::Deref<Target = AuthorizationUrls>,
{
  let auth = match head.find_header("Authorization") {
    Some(key) => load_authorization(key, session.deref(), records.deref()).await,
    None => Ok(None),
  }
  .unwrap_or_else(|e| {
    info!("unable to load authorization - {}", e);
    None
  });

  let _ctx = Context {
    session: session.deref(),
    records: records.deref(),
    urls: urls.deref(),
    auth,
  };
  Err(std::io::Error::new(std::io::ErrorKind::Other, ""))
}
