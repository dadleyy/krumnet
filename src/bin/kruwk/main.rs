use async_std::task::block_on;
use gumdrop::{parse_args_default_or_exit, Options as Gumdrop};
use log::info;
use std::io::Result;

use krumnet::Configuration;

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
    Ok(())
  })
}
