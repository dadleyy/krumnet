use async_std::task::block_on;
use gumdrop::{parse_args_default_or_exit, Options as Gumdrop};
use log::{debug, info, warn};
use std::env::args;
use std::io::Result;
use std::process::exit;

use krumnet::{
  interchange::jobs::{Job, QueuedJob},
  version, Configuration, JobStore, RecordStore,
};

mod context;
mod handlers;

pub use context::Context;

use handlers::{game_memberships, games, lobbies, lobby_memberships};

const MAX_WORKER_FAILS: u8 = 10;

#[derive(Debug, Gumdrop)]
struct Options {
  #[options(help = "configuration json file")]
  config: Configuration,

  #[options(help = "display the help text")]
  help: bool,

  #[options(help = "display the version and exit")]
  version: bool,
}

async fn execute<'a>(ctx: &Context<'a>, job: &QueuedJob) -> QueuedJob {
  let job_result = match &job.job {
    Job::CheckRoundFulfillment(details) => {
      games::check_round_fullfillment(&details, &ctx.records).await
    }
    Job::CreateLobby(details) => lobbies::create_lobby(&job.id, &details, &ctx.records).await,
    Job::CleanupLobbyMembership(details) => {
      lobby_memberships::cleanup(&job.id, &details, &ctx).await
    }
    Job::CreateGame(details) => lobbies::create_game(&job.id, &details, &ctx.records).await,
    Job::CleanupGameMembership(details) => game_memberships::cleanup(&details, &ctx).await,
    Job::CheckRoundCompletion(details) => games::check_round_completion(&details, &ctx).await,
  };

  QueuedJob {
    id: job.id.clone(),
    job: job_result,
  }
}

fn main() -> Result<()> {
  env_logger::builder().format_timestamp_millis().init();

  if let Err(e) = dotenv::dotenv() {
    debug!("unable to load dotenv - {}", e);
  }

  let opts = parse_args_default_or_exit::<Options>();

  if opts.version {
    let args = args().collect::<Vec<_>>();
    println!("{} version - {}", args[0], version::version());
    exit(0);
  }

  info!("starting worker process (version {})", version::version());

  block_on(async {
    let jobs = JobStore::open(&opts.config).await?;
    let records = RecordStore::open(&opts.config).await?;

    let ctx = Context {
      records: &records,
      jobs: &jobs,
    };

    let mut fails = 0;

    info!("backend stores connected successfully, starting dequeue");

    loop {
      let next = jobs.dequeue().await;

      match next {
        Ok(Some(job)) => {
          info!("pulled next job off queue - {:?}", job.id);
          let next = execute(&ctx, &job).await;
          if let Err(e) = jobs.update(&job.id, &next).await {
            warn!("unable to update job - {}", e);
          }
          fails = 0;
        }
        Ok(None) => {
          info!("nothing to work off, skppping");
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
