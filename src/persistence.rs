use std::io::{Error, ErrorKind, Result};
use std::time::Duration;

use async_std::net::TcpStream;
use async_std::sync::RwLock;
use r2d2::Pool as ConnectionPool;
use r2d2_postgres::postgres::types::ToSql;
use r2d2_postgres::postgres::{NoTls, Row, ToStatement};
use r2d2_postgres::PostgresConnectionManager as Postgres;

use crate::{errors, Configuration};

pub struct RecordStore {
  _pool: ConnectionPool<Postgres<NoTls>>,
  _stream: RwLock<TcpStream>,
}

impl std::fmt::Debug for RecordStore {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(formatter, "RecordStore")
  }
}

impl RecordStore {
  pub async fn open<C>(config: C) -> Result<Self>
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

    let stream = TcpStream::connect(config.record_store.redis_uri.as_str()).await?;

    Ok(RecordStore {
      _pool: pool,
      _stream: RwLock::new(stream),
    })
  }

  pub fn execute<T: ToStatement + ?Sized>(&self, q: &T, p: &[&(dyn ToSql + Sync)]) -> Result<u64> {
    let mut conn = self._pool.get().map_err(errors::humanize_error)?;
    conn.execute(q, p).map_err(errors::humanize_error)
  }

  pub fn query<T: ToStatement + ?Sized>(
    &self,
    q: &T,
    p: &[&(dyn ToSql + Sync)],
  ) -> Result<Vec<Row>> {
    let mut conn = self._pool.get().map_err(errors::humanize_error)?;
    conn.query(q, p).map_err(errors::humanize_error)
  }
}
