use std::io::{Error, Result};

use log::{info, warn};
use sqlx::pool::PoolConnection;
use sqlx::postgres::{PgConnection, PgPool};

use crate::{errors, Configuration};

fn warn_and_return<E: std::error::Error>(error: E) -> Error {
  warn!("record store failure - {}", error);
  errors::humanize_error(error)
}

pub struct RecordStore {
  _pg: PgPool,
}

pub type Connection = PoolConnection<PgConnection>;

impl RecordStore {
  pub async fn open(configuration: &Configuration) -> Result<Self> {
    let uri = &configuration.record_store.postgres_uri;

    let pg = PgPool::builder()
      .max_size(5)
      .build(uri)
      .await
      .map_err(errors::humanize_error)?;

    info!("successfully connected to '{}'", uri);

    Ok(RecordStore { _pg: pg })
  }

  pub async fn acquire(&self) -> Result<Connection> {
    self._pg.acquire().await.map_err(warn_and_return)
  }
}
