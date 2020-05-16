use async_std::task::block_on;
use gumdrop::{parse_args_default_or_exit, Options as Gumdrop};
use log::{debug, info, warn};
use std::io::Result;

use krumnet::records::Row;
use krumnet::{
  interchange::jobs::{Job, QueuedJob},
  names, Configuration, JobStore, RecordStore,
};

const MAX_WORKER_FAILS: u8 = 10;
const FIND_USER: &'static str = include_str!("data-store/find-user-by-id.sql");
const CREATE_LOBBY: &'static str = include_str!("data-store/create-lobby.sql");

#[derive(Debug, Gumdrop)]
struct Options {
  #[options(help = "configuration json file")]
  config: Configuration,

  #[options(help = "display the help text")]
  help: bool,
}

struct Context<'a> {
  records: &'a RecordStore,
}

#[derive(Debug)]
struct UserInfo {
  id: String,
  name: String,
  email: String,
}

fn parse_user(row: &Row) -> Option<UserInfo> {
  let id = row.try_get(0).ok()?;
  let name = row.try_get(1).ok()?;
  let email = row.try_get(2).ok()?;
  Some(UserInfo { id, email, name })
}

fn make_lobby(
  records: &RecordStore,
  job_id: &String,
  creator: &String,
) -> std::result::Result<String, String> {
  let mask = bit_vec::BitVec::from_elem(10, false);
  let name = names::get();

  let rows = records
    .query(FIND_USER, &[creator])
    .map_err(|_e| String::from("unable to query users for creator"))?;

  let user = rows
    .iter()
    .nth(0)
    .and_then(parse_user)
    .ok_or(String::from("unable to find user"))?;

  let rows = records
    .query(CREATE_LOBBY, &[job_id, &name, &mask, &user.id])
    .map_err(|e| {
      warn!("unable to create lobby - {}", e);
      String::from("unable to create")
    })?;

  rows
    .iter()
    .nth(0)
    .and_then(|row| row.try_get::<_, String>(0).ok())
    .ok_or(String::from("unable to parse as string"))
}

async fn create_lobby(job_id: &String, creator: &String, records: &RecordStore) -> QueuedJob {
  let result = make_lobby(records, job_id, creator);

  QueuedJob {
    id: job_id.clone(),
    job: Job::CreateLoby {
      result: Some(result),
      creator: creator.clone(),
    },
  }
}

impl<'a> Context<'a> {
  pub async fn execute(&self, job: &QueuedJob) -> QueuedJob {
    match &job.job {
      Job::CreateLoby { creator, .. } => create_lobby(&job.id, &creator, &self.records).await,
    }
  }
}

fn main() -> Result<()> {
  env_logger::init();
  let opts = parse_args_default_or_exit::<Options>();

  if opts.help {
    info!("{}", Options::usage());
    return Ok(());
  }

  block_on(async {
    debug!("starting worker process, opening job store");
    let jobs = JobStore::open(&opts.config).await?;
    let records = RecordStore::open(&opts.config).await?;
    let ctx = Context { records: &records };
    let mut fails = 0;
    debug!("job store successfully opened, starting dequeue");

    loop {
      let next = jobs.dequeue().await;

      match next {
        Ok(Some(job)) => {
          info!("pulled next job off queue - {:?}", job.id);
          let next = ctx.execute(&job).await;
          if let Err(e) = jobs.update(&job.id, &next).await {
            warn!("unable to update job - {}", e);
          }
          fails = 0;
        }
        Ok(None) => {
          debug!("nothing to work off, skppping");
          fails = 0;
        }
        Err(e) => {
          fails = fails + 1;

          if fails > MAX_WORKER_FAILS {
            warn!("final failure on job dequeue attempt - {}, exiting", e);
            break;
          }

          warn!("failed job store dequeue attempt - {}", e);
          continue;
        }
      }
    }

    Ok(())
  })
}
