use std::fmt::Display;
use std::io::Result;

use log::info;

use async_std::net::TcpStream;
use async_std::sync::RwLock;
use kramer::{Arity, Command, HashCommand, Insertion, ListCommand, Response, ResponseValue, Side};
use serde::{Deserialize, Serialize};
use serde_json::{from_str as deserialize, to_string as serialize};
use uuid::Uuid;

use crate::interchange::provisioning::ProvisioningAttempt;
use crate::Configuration;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct QueuedProvisioningAttempt {
  pub id: String,
  attempt: ProvisioningAttempt,
}

pub struct Provisioner {
  _stream: RwLock<TcpStream>,
  _keys: (String, String),
}

impl Provisioner {
  pub async fn open<C>(config: C) -> Result<Self>
  where
    C: std::ops::Deref<Target = Configuration>,
  {
    let stream = TcpStream::connect(config.record_store.redis_uri.as_str()).await?;
    let queue = config.record_store.provisioning_queue.clone();
    let map = config.record_store.provisioning_map.clone();
    Ok(Provisioner {
      _keys: (queue, map),
      _stream: RwLock::new(stream),
    })
  }

  pub async fn command<K: Display, V: Display>(&self, cmd: &Command<K, V>) -> Result<Response> {
    let mut stream = self._stream.write().await;
    kramer::execute(&mut (*stream), cmd).await
  }

  async fn dequeue_next_id(&self) -> Result<Option<String>> {
    let (queue_key, _) = &self._keys;
    let cmd = Command::List::<_, &str>(ListCommand::Pop(Side::Left, queue_key, Some((None, 10))));
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

  async fn find(&self, id: &String) -> Result<Option<QueuedProvisioningAttempt>> {
    let (_, map_key) = &self._keys;
    let lookup = Command::Hashes::<_, &str>(HashCommand::Get(map_key, Some(Arity::One(id))));
    let res = self.command(&lookup).await?;

    if let Response::Item(ResponseValue::String(serialized)) = res {
      info!("pulled provisioning map entry - {:?}", serialized);
      let attempt = deserialize::<QueuedProvisioningAttempt>(serialized.as_str())?;
      return Ok(Some(attempt));
    }

    info!(
      "strange response from provisioning map for '{}' - {:?}",
      id, res
    );
    Ok(None)
  }

  pub async fn dequeue(&self) -> Result<Option<QueuedProvisioningAttempt>> {
    let next = self.dequeue_next_id().await?;
    match next {
      Some(id) => self.find(&id).await,
      None => Ok(None),
    }
  }

  pub async fn queue(&self, attempt: ProvisioningAttempt) -> Result<String> {
    let (queue_key, map_key) = &self._keys;

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
}
