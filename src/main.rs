mod args;
mod way;

use clap::Parser;
use crate::args::way_args::WayArgs;
use crate::way::way_finder::WayFinder;

fn main() {
  let args = WayArgs::parse();
  let finder = WayFinder::new(args);
  if let Some(way) = finder.find_way() {
    println!("{}", way.display());
  } else {
    eprintln!("No matching directories found");
    std::process::exit(1);
  }
}
