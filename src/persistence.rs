use r2d2::{Pool as ConnectionPool, PooledConnection};
use r2d2_postgres::postgres::NoTls;
use r2d2_postgres::PostgresConnectionManager as Postgres;
use std::io::{Error, ErrorKind};
use std::time::Duration;

use crate::Configuration;

pub type Connection = PooledConnection<Postgres<NoTls>>;

pub struct RecordStore {
  pool: ConnectionPool<Postgres<NoTls>>,
}

impl std::fmt::Debug for RecordStore {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(formatter, "RecordStore")
  }
}

impl RecordStore {
  pub fn open<C>(config: C) -> Result<Self, Error>
  where
    C: std::ops::Deref<Target = Configuration>,
  {
    let parsed_config = config
      .record_store
      .postgres_uri
      .as_str()
      .parse()
      .map_err(|e| Error::new(ErrorKind::Other, e))?;
    let manager = Postgres::new(parsed_config, NoTls);

    let pool = ConnectionPool::builder()
      .connection_timeout(Duration::new(1, 0))
      .build(manager)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    Ok(RecordStore { pool })
  }

  pub fn get(&self) -> Result<Connection, Error> {
    self.pool.get().map_err(|e| Error::new(ErrorKind::Other, e))
  }
}
