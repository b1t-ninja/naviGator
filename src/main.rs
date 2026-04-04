mod args;
mod way;

use clap::Parser;
use crate::args::way_args::WayArgs;
use crate::way::way_finder::WayFinder;
use ansi_term::Colour::Red;

fn main() {
  let args = WayArgs::parse();
  let finder = WayFinder::new(args);
  if let Some(way) = finder.find_way() {
    println!("{}", way.display());
  } else {
    eprintln!("{}: Could not find any matching directory", Red.bold().paint("[Error]"));
    std::process::exit(1);
  }
}
