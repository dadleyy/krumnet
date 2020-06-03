extern crate async_std;
extern crate dotenv;
extern crate gumdrop;

use async_std::task;
use gumdrop::{parse_args_default_or_exit, Options as Gumdrop};
use log::{debug, info};
use std::env::args;
use std::process::exit;

use krumnet::{serve, version, Configuration};

#[derive(Debug, Gumdrop)]
struct Options {
  #[options(help = "configuration json file")]
  config: Configuration,

  #[options(help = "display the help text")]
  help: bool,

  #[options(help = "print the version and exit")]
  version: bool,
}

fn main() {
  env_logger::builder().format_timestamp_millis().init();

  if let Err(e) = dotenv::dotenv() {
    debug!("unable to load .env - {}", e);
  }

  let opts = parse_args_default_or_exit::<Options>();

  if opts.version {
    let args = args().collect::<Vec<_>>();
    println!("{} version - {}", args[0], version::version());
    exit(0);
  }

  info!(
    "starting server '{:?}' (version {})",
    opts.config.addr,
    version::version()
  );

  let out = task::block_on(serve(opts.config));

  if let Err(e) = out {
    info!("[error] exiting with error: {:?}", e);
  }
}
