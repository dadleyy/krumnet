use async_std::task::block_on;
use gumdrop::{parse_args_default_or_exit, Options as Gumdrop};
use log::{debug, info, warn};
use std::io::Result;

use krumnet::{
  interchange::jobs::{Job, QueuedJob},
  Configuration, JobStore, RecordStore,
};

mod context;
mod handlers;

pub use context::Context;

const MAX_WORKER_FAILS: u8 = 10;

#[derive(Debug, Gumdrop)]
struct Options {
  #[options(help = "configuration json file", default = "krumnet-config.json")]
  config: Configuration,

  #[options(help = "display the help text")]
  help: bool,
}

async fn execute<'a>(ctx: &Context<'a>, job: &QueuedJob) -> QueuedJob {
  match &job.job {
    Job::CheckRoundFulfillment { round_id, .. } => {
      debug!("received round fulfillment check job - '{}'", round_id);
      let result = Some(handlers::games::check_round_fullfillment(round_id, &ctx.records).await);
      QueuedJob {
        id: job.id.clone(),
        job: Job::CheckRoundFulfillment {
          round_id: round_id.clone(),
          result,
        },
      }
    }
    Job::CreateLobby { creator, .. } => {
      debug!("passing create lobby job off to create lobby handler");
      handlers::lobbies::create_lobby(&job.id, &creator, &ctx.records).await
    }
    Job::CleanupLobbyMembership {
      member_id,
      lobby_id,
      ..
    } => {
      debug!(
        "handling lobby membership cleanup for membership '{}'",
        member_id
      );
      handlers::lobby_memberships::cleanup(&job.id, &member_id, &lobby_id, &ctx).await
    }
    Job::CreateGame {
      creator, lobby_id, ..
    } => {
      debug!("passing create game off to handler");
      handlers::lobbies::create_game(&job.id, &creator, &lobby_id, &ctx.records).await
    }

    Job::CleanupGameMembership(details) => {
      debug!("handler cleaning up game membership {}", details.member_id);
      QueuedJob {
        id: job.id.clone(),
        job: handlers::game_memberships::cleanup(&details, &ctx).await,
      }
    }

    Job::CheckRoundCompletion(details) => {
      debug!("handler checking round completion {}", details.round_id);
      QueuedJob {
        id: job.id.clone(),
        job: handlers::games::check_round_completion(&details, &ctx).await,
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
    let ctx = Context {
      records: &records,
      jobs: &jobs,
    };
    let mut fails = 0;
    debug!("job store successfully opened, starting dequeue");

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
