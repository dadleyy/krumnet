use async_std::sync::Arc;
use elaine::Head;
use log::{debug, warn};
use sqlx::query_file;
use std::io::Result;

use crate::http::AUTHORIZATION;
use crate::{errors, Authority, Configuration, JobStore, RecordConnection, RecordStore, SessionStore};

pub struct Context {
  _auth: Authority,
  _session: Arc<SessionStore>,
  _records: Arc<RecordStore>,
  _jobs: Arc<JobStore>,
  _config: Configuration,
  _pending: usize,
}

impl Context {
  pub fn builder() -> ContextBuilder {
    ContextBuilder::default()
  }

  pub fn pending(&self) -> usize {
    self._pending
  }

  pub fn jobs(&self) -> &JobStore {
    &self._jobs
  }

  pub fn authority(&self) -> &Authority {
    &self._auth
  }

  pub fn session(&self) -> &SessionStore {
    &self._session
  }

  pub fn records(&self) -> &RecordStore {
    &self._records
  }

  pub async fn records_connection(&self) -> Result<RecordConnection> {
    self._records.acquire().await
  }

  pub fn config(&self) -> &Configuration {
    &self._config
  }

  pub fn cors(&self) -> String {
    self._config.krumi.cors_origin.clone()
  }
}

impl std::fmt::Debug for Context {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(formatter, "Context<Autority:{:?}>", &self._auth)
  }
}

#[derive(Default)]
pub struct ContextBuilder {
  _session: Option<Arc<SessionStore>>,
  _records: Option<Arc<RecordStore>>,
  _jobs: Option<Arc<JobStore>>,
  _config: Option<Configuration>,
}

// Attempts to exchange an authorization token for a user id from the session store, subsequently
// loading the actual user information from the record store.
pub async fn load_authorization(token: String, session: &SessionStore, records: &RecordStore) -> Result<Authority> {
  let uid = session.get(&token).await?;
  let mut conn = records.acquire().await?;
  let tenant = query_file!("src/data-store/user-for-session.sql", uid)
    .fetch_all(&mut conn)
    .await
    .map_err(errors::humanize_error)?
    .into_iter()
    .nth(0)
    .and_then(|row| {
      let id = row.user_id;

      debug!("found user '{:?}'", id);

      Some(Authority::User {
        id,
        token: token.clone(),
      })
    });

  Ok(tenant.unwrap_or(Authority::None))
}

async fn load_auth(head: &Head, session: &SessionStore, records: &RecordStore) -> Result<Authority> {
  if let Some(value) = head.find_header(AUTHORIZATION) {
    debug!("found authorization header - {}", value);
    return load_authorization(value, session, records).await.or_else(|e| {
      warn!("unable to load authorization - {}", e);
      Ok(Authority::None)
    });
  }

  debug!("no authorization header present");
  Ok(Authority::None)
}

impl ContextBuilder {
  pub fn configuration(self, config: &Configuration) -> Self {
    ContextBuilder {
      _config: Some(config.clone()),
      ..self
    }
  }

  pub fn records(self, records: Arc<RecordStore>) -> Self {
    ContextBuilder {
      _records: Some(records),
      ..self
    }
  }

  pub fn jobs(self, jobs: Arc<JobStore>) -> Self {
    ContextBuilder {
      _jobs: Some(jobs),
      ..self
    }
  }

  pub fn session(self, session: Arc<SessionStore>) -> Self {
    ContextBuilder {
      _session: Some(session),
      ..self
    }
  }

  pub fn with_authority(self, auth: Authority) -> Result<Context> {
    let _config = self._config.ok_or(errors::e("missing configuraiton from context"))?;

    let _records = self
      ._records
      .ok_or(errors::e("missing records configuration for context"))?;

    let _jobs = self._jobs.ok_or(errors::e("missing job configuration for context"))?;

    let _session = self
      ._session
      .ok_or(errors::e("missing session configuration for context"))?;

    Ok(Context {
      _auth: auth,
      _jobs,
      _config,
      _session,
      _records,
      _pending: 0,
    })
  }

  pub async fn for_request(self, head: &Head) -> Result<Context> {
    let records = self
      ._records
      .as_ref()
      .ok_or(errors::e("missing session configuration for context"))?;

    let session = self
      ._session
      .as_ref()
      .ok_or(errors::e("missing session configuration for context"))?;

    let auth = load_auth(head, session, records).await?;
    Ok(Context {
      _pending: head.len().unwrap_or_default(),
      ..self.with_authority(auth)?
    })
  }
}

#[cfg(test)]
mod test_helpers {
  use super::Context;
  use crate::configuration::load_test_config as load_config;
  use crate::{Authority, JobStore, RecordStore, SessionStore};
  use async_std::task::block_on;
  use std::sync::Arc;

  pub fn with_auth(auth: Authority) -> Context {
    block_on(async {
      let config = load_config().unwrap();
      let session = Arc::new(SessionStore::open(&config).await.unwrap());
      let records = Arc::new(RecordStore::open(&config).await.unwrap());
      let jobs = Arc::new(JobStore::open(&config).await.unwrap());
      Context::builder()
        .configuration(&config)
        .records(records)
        .session(session)
        .jobs(jobs)
        .with_authority(auth)
        .unwrap()
    })
  }
}

#[cfg(test)]
mod test {
  use super::test_helpers::with_auth;
  use crate::Authority;

  #[test]
  fn test_none_authority() {
    assert_eq!(with_auth(Authority::None).authority(), &Authority::None);
  }
}
