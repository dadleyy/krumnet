use async_std::net::TcpStream;
use async_std::sync::RwLock;
use kramer::{Arity, Command, HashCommand, Insertion, ListCommand, Response, ResponseValue, Side};
use log::{debug, info};
use serde_json::{from_str as deserialize, to_string as serialize};
use std::fmt::Display;
use std::io::Result;
use uuid::Uuid;

use crate::interchange::jobs::{DequeuedJob, Job, QueuedJob};
use crate::Configuration;

pub struct JobStore {
  _stream: RwLock<TcpStream>,
  _keys: (String, String, String),
  _queue_delay: u64,
}

fn dequeue_cmd(queue_key: &String, delay: u64) -> Command<&str, &str> {
  Command::List::<_, &str>(ListCommand::Pop(Side::Left, queue_key, Some((None, delay))))
}

impl JobStore {
  async fn command<K: Display, V: Display>(&self, cmd: &Command<K, V>) -> Result<Response> {
    let mut stream = self._stream.write().await;
    kramer::execute(&mut (*stream), cmd).await
  }

  pub async fn lookup(&self, id: &String) -> Result<Option<QueuedJob>> {
    self.deserialize_entry(id).await
  }

  async fn dequeue_next_id(&self) -> Result<Option<String>> {
    let (queue_key, _, _) = &self._keys;
    let cmd = dequeue_cmd(queue_key, self._queue_delay);
    let res = self.command(&cmd).await?;

    match res {
      Response::Array(contents) => {
        if let Some(ResponseValue::String(serialized)) = contents.iter().nth(1) {
          info!("found serialized queue entry - '{}'", serialized);
          return Ok(Some(serialized.clone()));
        }

        info!(
          "strange value popped from provisioning queue - {:?}",
          contents
        );
        Ok(None)
      }
      _ => Ok(None),
    }
  }

  async fn deserialize_entry(&self, id: &String) -> Result<Option<QueuedJob>> {
    let (_, map_key, _) = &self._keys;
    let lookup = Command::Hashes::<_, &str>(HashCommand::Get(map_key, Some(Arity::One(id))));
    let res = self.command(&lookup).await?;

    if let Response::Item(ResponseValue::String(serialized)) = res {
      info!("pulled provisioning map entry - {:?}", serialized);
      let attempt = deserialize::<QueuedJob>(serialized.as_str())?;
      return Ok(Some(attempt));
    }

    info!(
      "strange response from provisioning map for '{}' - {:?}",
      id, res
    );
    Ok(None)
  }

  pub async fn update(&self, id: &String, job: &QueuedJob) -> Result<String> {
    let (_, map_key, _) = &self._keys;
    let serialized = serialize(&job)?;
    let map_cmd = Command::Hashes(HashCommand::Set(
      map_key,
      Arity::One((&id, serialized)),
      Insertion::Always,
    ));
    self.command(&map_cmd).await.map(|_| id.clone())
  }

  pub async fn dequeue(&self) -> Result<Option<QueuedJob>> {
    let (_, _, dequeue_key) = &self._keys;
    let next = self.dequeue_next_id().await?;
    match next {
      Some(id) => {
        debug!("popped id '{}' off queue, writing dequeue job", id);
        let serialized = serialize(&DequeuedJob::new(&id))?;

        let cmd = Command::Hashes(HashCommand::Set(
          dequeue_key,
          Arity::One((&id, serialized)),
          Insertion::Always,
        ));

        self.command(&cmd).await?;
        self.deserialize_entry(&id).await
      }
      None => Ok(None),
    }
  }

  pub async fn queue(&self, job: &Job) -> Result<String> {
    let uid = Uuid::new_v4().to_string();

    let queued = QueuedJob {
      id: uid.clone(),
      job: job.clone(),
    };
    let serialized = serialize(&queued)?;

    debug!("serialized job '{}' - '{}'", uid, serialized);

    let (queue_key, map_key, _) = &self._keys;

    let queue_cmd = Command::List(ListCommand::Push(
      (Side::Right, Insertion::Always),
      queue_key,
      Arity::One(&uid),
    ));

    let map_cmd = Command::Hashes(HashCommand::Set(
      map_key,
      Arity::One((&uid, serialized)),
      Insertion::Always,
    ));

    self.command(&map_cmd).await?;
    self.command(&queue_cmd).await?;

    debug!("job '{}' inserted", uid);
    Ok(uid)
  }

  pub async fn open<C>(configuration: C) -> Result<Self>
  where
    C: std::ops::Deref<Target = Configuration>,
  {
    let stream = TcpStream::connect(configuration.job_store.redis_uri.as_str()).await?;
    let (queue, map, dequeue) = (
      &configuration.job_store.queue_key,
      &configuration.job_store.map_key,
      &configuration.job_store.dequeue_key,
    );

    let delay = if configuration.job_store.queue_delay > 0 {
      configuration.job_store.queue_delay
    } else {
      10
    };

    info!("job store ready, queue[{}] map[{}]", queue, map);

    Ok(JobStore {
      _queue_delay: delay,
      _stream: RwLock::new(stream),
      _keys: (queue.clone(), map.clone(), dequeue.clone()),
    })
  }
}
