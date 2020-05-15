use async_std::task::block_on;
use gumdrop::{parse_args_default_or_exit, Options as Gumdrop};
use log::{debug, info, warn};
use std::io::Result;

use krumnet::{Configuration, JobStore};

const MAX_WORKER_FAILS: u8 = 10;

#[derive(Debug, Gumdrop)]
struct Options {
  #[options(help = "configuration json file")]
  config: Configuration,

  #[options(help = "display the help text")]
  help: bool,
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
    let mut fails = 0;
    debug!("job store successfully opened, starting dequeue");

    loop {
      let next = jobs.dequeue().await;

      match next {
        Ok(Some(job)) => {
          info!("pulled next job off queue - {:?}", job.id);
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
