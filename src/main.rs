mod args;
mod way;

use clap::Parser;
use crate::args::way_args::WayArgs;
use crate::way::way_finder::WayFinder;

fn main() {
  let args = WayArgs::parse();
  let finder = WayFinder::new(args);
  println!("{}", finder.find_way());
}
