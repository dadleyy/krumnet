use elaine::Head;
use log::info;
use std::io::{Error, ErrorKind, Result};
use std::sync::Arc;

use crate::errors::humanize_error;
use crate::http::{header, HeaderMap, HeaderValue};
use crate::{Authorization, AuthorizationUrls, RecordStore, SessionStore};

const USER_FOR_SESSION: &'static str = include_str!("data-store/user-for-session.sql");

pub trait SessionInterface: std::ops::Deref<Target = SessionStore> {}
impl<T> SessionInterface for T where T: std::ops::Deref<Target = SessionStore> {}

pub trait RecordInterface: std::ops::Deref<Target = RecordStore> {}
impl<T> RecordInterface for T where T: std::ops::Deref<Target = RecordStore> {}

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

pub struct StaticContext {
  session: Arc<SessionStore>,
  records: Arc<RecordStore>,
  urls: Arc<AuthorizationUrls>,
  auth: Option<Authorization>,
}

impl StaticContext {
  pub fn session(&self) -> &SessionStore {
    &self.session
  }

  pub fn auth(&self) -> &Option<Authorization> {
    &self.auth
  }

  pub fn records(&self) -> &RecordStore {
    &self.records
  }

  pub fn urls(&self) -> &AuthorizationUrls {
    &self.urls
  }

  pub fn cors(&self) -> Result<HeaderMap> {
    let mut headers = HeaderMap::with_capacity(5);

    headers.insert(
      header::ACCESS_CONTROL_ALLOW_ORIGIN,
      HeaderValue::from_str(&self.urls.cors_origin).map_err(humanize_error)?,
    );
    headers.insert(
      header::ACCESS_CONTROL_ALLOW_HEADERS,
      HeaderValue::from_str(header::AUTHORIZATION.as_str()).map_err(humanize_error)?,
    );

    Ok(headers)
  }
}

pub struct StaticContextBuilder {
  session: Option<Arc<SessionStore>>,
  records: Option<Arc<RecordStore>>,
  urls: Option<Arc<AuthorizationUrls>>,
}

impl StaticContextBuilder {
  pub fn new() -> Self {
    StaticContextBuilder {
      session: None,
      records: None,
      urls: None,
    }
  }

  pub fn urls(self, urls: Arc<AuthorizationUrls>) -> Self {
    StaticContextBuilder {
      urls: Some(urls),
      ..self
    }
  }

  pub fn records(self, records: Arc<RecordStore>) -> Self {
    StaticContextBuilder {
      records: Some(records),
      ..self
    }
  }

  pub fn session(self, session: Arc<SessionStore>) -> Self {
    StaticContextBuilder {
      session: Some(session),
      ..self
    }
  }

  pub async fn for_request(self, head: &Head) -> Result<StaticContext> {
    if let (Some(session), Some(records)) = (self.session.as_ref(), self.records.as_ref()) {
      let auth = match head.find_header(header::AUTHORIZATION) {
        Some(key) => load_authorization(key, session.clone(), records.clone()).await,
        None => Ok(None),
      }
      .unwrap_or_else(|e| {
        info!("unable to load authorization - {}", e);
        None
      });

      return self.build(auth);
    }

    Err(Error::new(
      ErrorKind::Other,
      "attempted to build static context w/o session or records",
    ))
  }

  pub fn build(self, auth: Option<Authorization>) -> Result<StaticContext> {
    let records = self
      .records
      .ok_or(Error::new(ErrorKind::Other, "no record store provided"))?;

    let session = self
      .session
      .ok_or(Error::new(ErrorKind::Other, "no session store provided"))?;

    let urls = self.urls.ok_or(Error::new(
      ErrorKind::Other,
      "no authorization urls provided",
    ))?;

    Ok(StaticContext {
      session,
      records,
      urls,
      auth,
    })
  }
}

#[cfg(test)]
mod test_helpers {
  use super::{StaticContext, StaticContextBuilder};
  use crate::configuration::test_helpers::load_config;
  use crate::{Authorization, AuthorizationUrls, RecordStore, SessionStore};
  use async_std::task::block_on;
  use std::sync::Arc;

  pub fn with_auth(auth: Option<Authorization>) -> StaticContext {
    block_on(async {
      let config = load_config().unwrap();
      let session = Arc::new(SessionStore::open(&config).await.unwrap());
      let records = Arc::new(RecordStore::open(&config).await.unwrap());
      let urls = Arc::new(AuthorizationUrls::open(&config).await.unwrap());
      StaticContextBuilder::new()
        .records(records)
        .session(session)
        .urls(urls)
        .build(auth)
        .unwrap()
    })
  }
}

#[cfg(test)]
mod test {
  use super::test_helpers::with_auth;
  use crate::Authorization;

  #[test]
  fn with_auth_some() {
    let ctx = with_auth(Some(Authorization(
      "s-123".to_string(),
      "tester".to_string(),
      "test@tester.com".to_string(),
      "token-123".to_string(),
    )));
    assert!(ctx.auth().is_some());
  }

  #[test]
  fn with_auth_none() {
    let ctx = with_auth(None);
    assert!(ctx.auth().is_none());
  }
}
