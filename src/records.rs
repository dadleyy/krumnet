use std::io::Result;
use std::time::Duration;

use log::info;
use r2d2::Pool as ConnectionPool;
use r2d2_postgres::postgres::types::ToSql;
use r2d2_postgres::postgres::{NoTls, ToStatement};
use r2d2_postgres::PostgresConnectionManager as Postgres;
use sqlx::pool::PoolConnection;
use sqlx::postgres::{PgConnection, PgPool};

pub use r2d2_postgres::postgres::Row;

use crate::{errors, Configuration};

pub struct RecordStore {
  _pool: ConnectionPool<Postgres<NoTls>>,
  _pg: PgPool,
}

impl RecordStore {
  pub async fn open(configuration: &Configuration) -> Result<Self> {
    let parsed_config = configuration
      .record_store
      .postgres_uri
      .as_str()
      .parse()
      .map_err(errors::humanize_error)?;

    let pg = PgPool::builder()
      .max_size(5)
      .build(&configuration.record_store.postgres_uri)
      .await
      .map_err(errors::humanize_error)?;

    let manager = Postgres::new(parsed_config, NoTls);

    let pool = ConnectionPool::builder()
      .connection_timeout(Duration::new(1, 0))
      .build(manager)
      .map_err(errors::humanize_error)?;

    info!("connection pool successfully created, ready to execute queries");

    Ok(RecordStore {
      _pool: pool,
      _pg: pg,
    })
  }

  pub async fn q(&self) -> Result<PoolConnection<PgConnection>> {
    self._pg.acquire().await.map_err(errors::humanize_error)
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
