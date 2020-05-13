use std::fmt::Display;
use std::io::{Error, ErrorKind, Result};
use std::time::Duration;

use log::info;

use async_std::net::TcpStream;
use async_std::sync::RwLock;
use kramer::{Arity, Command, HashCommand, Insertion, ListCommand, Response, ResponseValue, Side};
use r2d2::Pool as ConnectionPool;
use r2d2_postgres::postgres::types::ToSql;
use r2d2_postgres::postgres::{NoTls, Row, ToStatement};
use r2d2_postgres::PostgresConnectionManager as Postgres;
use serde::{Deserialize, Serialize};
use serde_json::{from_str as deserialize, to_string as serialize};
use uuid::Uuid;

use crate::interchange::provisioning::ProvisioningAttempt;
use crate::{errors, Configuration};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct QueuedProvisioningAttempt {
  pub id: String,
  attempt: ProvisioningAttempt,
}

pub struct RecordStore {
  _storage_keys: (String, String),
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
    let queue = config.record_store.provisioning_queue.clone();
    let map = config.record_store.provisioning_map.clone();

    info!(
      "record store initialized w/ provisioning queue '{}' & map '{}'",
      queue, map
    );

    Ok(RecordStore {
      _pool: pool,
      _stream: RwLock::new(stream),
      _storage_keys: (queue, map),
    })
  }

  pub async fn command<K: Display, V: Display>(&self, cmd: &Command<K, V>) -> Result<Response> {
    let mut stream = self._stream.write().await;
    kramer::execute(&mut (*stream), cmd).await
  }

  pub async fn dequeue(&self) -> Result<Option<QueuedProvisioningAttempt>> {
    let (queue_key, _) = &self._storage_keys;
    let cmd = Command::List::<_, &str>(ListCommand::Pop(Side::Left, queue_key, Some((None, 10))));
    let res = self.command(&cmd).await?;

    match res {
      Response::Array(contents) => {
        if let Some(ResponseValue::String(serialized)) = contents.iter().nth(1) {
          let attempt = deserialize::<QueuedProvisioningAttempt>(serialized.as_str())?;
          info!("popped off queue with contents {:?}", contents);
          return Ok(Some(attempt));
        }

        Ok(None)
      }
      _ => Ok(None),
    }
  }

  pub async fn queue(&self, attempt: ProvisioningAttempt) -> Result<String> {
    let (queue_key, map_key) = &self._storage_keys;

    let queued = QueuedProvisioningAttempt {
      id: Uuid::new_v4().to_string(),
      attempt,
    };

    let serialized = serialize(&queued)?;

    let queue_cmd = Command::List(ListCommand::Push(
      (Side::Right, Insertion::Always),
      queue_key,
      Arity::One(&queued.id),
    ));

    let map_cmd = Command::Hashes(HashCommand::Set(
      map_key,
      Arity::One((&queued.id, serialized)),
      Insertion::Always,
    ));

    self.command(&map_cmd).await?;
    self.command(&queue_cmd).await?;

    Ok(queued.id)
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
