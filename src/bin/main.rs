extern crate async_std;
extern crate gumdrop;

use gumdrop::{parse_args_default_or_exit, Options as Gumdrop};

use async_std::task;
use krumnet::configuration::Configuration;
use krumnet::run;

#[derive(Debug, Gumdrop)]
struct Options {
  #[options(help = "configuration toml file")]
  config: Configuration,

  #[options(help = "display the help text")]
  help: bool,
}

fn main() {
  let opts = parse_args_default_or_exit::<Options>();

  if opts.help {
    println!("{}", Options::usage());
    return;
  }

  println!("[debug] starting server '{:?}'", opts.config.addr);

  if let Err(e) = task::block_on(run(opts.config)) {
    println!("[error] exiting with error: {:?}", e);
  }
}
