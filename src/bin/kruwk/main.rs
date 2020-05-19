use async_std::task::block_on;
use gumdrop::{parse_args_default_or_exit, Options as Gumdrop};
use log::{debug, info, warn};
use std::io::Result;

use krumnet::{
  interchange::jobs::{Job, QueuedJob},
  Configuration, JobStore, RecordStore,
};

mod handlers;

const MAX_WORKER_FAILS: u8 = 10;

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

impl<'a> Context<'a> {
  pub async fn execute(&self, job: &QueuedJob) -> QueuedJob {
    match &job.job {
      Job::CheckRoundCompletion { round_id, .. } => {
        debug!("received round completion check job - '{}'", round_id);
        let result = Some(handlers::games::check_round_completion(round_id, &self.records).await);
        QueuedJob {
          id: job.id.clone(),
          job: Job::CheckRoundCompletion {
            round_id: round_id.clone(),
            result,
          },
        }
      }
      Job::CreateLobby { creator, .. } => {
        debug!("passing create lobby job off to create lobby handler");
        handlers::lobbies::create_lobby(&job.id, &creator, &self.records).await
      }
      Job::CreateGame {
        creator, lobby_id, ..
      } => {
        debug!("passing create game off to handler");
        handlers::lobbies::create_game(&job.id, &creator, &lobby_id, &self.records).await
      }
    }
  }
}

fn main() -> Result<()> {
  env_logger::builder().format_timestamp_millis().init();
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
