mod args;
mod way;

use clap::Parser;
use crate::args::way_args::WayArgs;
use crate::way::way_finder::WayFinder;
use crate::way::way_selector::select_way;
use ansi_term::Colour::Red;

fn main() {
  let args = WayArgs::parse();
  let interactive = args.interactive;
  let finder = WayFinder::new(args);
  let ways = finder.find_way();

  if ways.is_empty() {
    eprintln!("{}: Could not find any matching directory", Red.bold().paint("[Error]"));
    std::process::exit(1);
  }

  let final_way = if interactive && ways.len() > 1 {
    match select_way(&ways) {
      Ok(Some(way)) => Some(way),
      Ok(None) => std::process::exit(0),
      Err(e) => {
        eprintln!("{}: TUI error: {}", Red.bold().paint("[Error]"), e);
        std::process::exit(1);
      }
    }
  } else {
    ways.into_iter().next()
  };

  if let Some(way) = final_way {
    println!("{}", way.display());
  } else {
    eprintln!("{}: Could not find any matching directory", Red.bold().paint("[Error]"));
    std::process::exit(1);
  }
}
