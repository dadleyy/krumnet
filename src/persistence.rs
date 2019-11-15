use r2d2::{Pool as ConnectionPool, PooledConnection};
use r2d2_postgres::{PostgresConnectionManager as Postgres, TlsMode};
use std::io::{Error, ErrorKind};
use std::time::Duration;

use crate::Configuration;

pub struct RecordStore {
  pool: ConnectionPool<Postgres>,
}

impl RecordStore {
  pub fn open<C>(config: C) -> Result<Self, Error>
  where
    C: std::ops::Deref<Target = Configuration>,
  {
    let manager = Postgres::new(config.record_store.postgres_uri.as_str(), TlsMode::None)?;
    let pool = ConnectionPool::builder()
      .connection_timeout(Duration::new(1, 0))
      .build(manager)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    Ok(RecordStore { pool })
  }

  pub fn get(&self) -> Result<PooledConnection<Postgres>, Error> {
    self.pool.get().map_err(|e| Error::new(ErrorKind::Other, e))
  }
}
