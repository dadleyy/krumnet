use async_std::sync::Arc;
use elaine::Head;
use std::io::Result;

use crate::{errors, Authority, Configuration, RecordStore, SessionStore};

pub struct Context {
  _auth: Authority,
  _session: Arc<SessionStore>,
  _records: Arc<RecordStore>,
  _config: Configuration,
}

impl Context {
  pub fn builder() -> ContextBuilder {
    ContextBuilder::default()
  }

  pub fn session(&self) -> &SessionStore {
    &self._session
  }

  pub fn records(&self) -> &RecordStore {
    &self._records
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
  _config: Option<Configuration>,
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

  pub fn session(self, session: Arc<SessionStore>) -> Self {
    ContextBuilder {
      _session: Some(session),
      ..self
    }
  }

  pub fn for_request(self, head: &Head) -> Result<Context> {
    let _config = self
      ._config
      .ok_or(errors::e("missing configuraiton from context"))?;

    let _records = self
      ._records
      .ok_or(errors::e("missing records configuration for context"))?;

    let _session = self
      ._session
      .ok_or(errors::e("missing session configuration for context"))?;

    Ok(Context {
      _auth: Authority::default(),
      _config,
      _session,
      _records,
    })
  }
}
