extern crate async_std;
extern crate gumdrop;

use gumdrop::{parse_args_default_or_exit, Options as Gumdrop};

use async_std::task;
use krumnet::{serve, Configuration};
use log::info;

#[derive(Debug, Gumdrop)]
struct Options {
  #[options(help = "configuration json file")]
  config: Configuration,

  #[options(help = "display the help text")]
  help: bool,
}

fn main() {
  env_logger::builder().format_timestamp_millis().init();
  let opts = parse_args_default_or_exit::<Options>();

  if opts.help {
    info!("{}", Options::usage());
    return;
  }

  info!("[debug] starting server '{:?}'", opts.config.addr);

  let out = task::block_on(serve(opts.config));

  if let Err(e) = out {
    info!("[error] exiting with error: {:?}", e);
  }
}
