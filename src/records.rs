use std::io::Result;

use log::info;
use sqlx::pool::PoolConnection;
use sqlx::postgres::{PgConnection, PgPool};

use crate::{errors, Configuration};

pub struct RecordStore {
  _pg: PgPool,
}

impl RecordStore {
  pub async fn open(configuration: &Configuration) -> Result<Self> {
    let pg = PgPool::builder()
      .max_size(5)
      .build(&configuration.record_store.postgres_uri)
      .await
      .map_err(errors::humanize_error)?;

    info!("connection pool successfully created, ready to execute queries");

    Ok(RecordStore { _pg: pg })
  }

  pub async fn q(&self) -> Result<PoolConnection<PgConnection>> {
    self._pg.acquire().await.map_err(errors::humanize_error)
  }
}
