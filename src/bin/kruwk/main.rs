use async_std::task::block_on;
use gumdrop::{parse_args_default_or_exit, Options as Gumdrop};
use log::info;
use std::io::Result;

use krumnet::{Configuration, RecordStore};

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
    info!("starting worker process");
    let records = RecordStore::open(&opts.config).await?;
    info!("record store opened successfully, starting worker");

    loop {
      match records.dequeue().await {
        Ok(Some(attempt)) => {
          info!("handling provisioning attempt {}", attempt.id);
        }
        _ => info!("unable to pull attempt off queue, moving on"),
      }
    }
  })
}
