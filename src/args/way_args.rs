use std::path::PathBuf;
use clap::Parser;

/// Find your way around the terminal with ease 🧭
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct WayArgs {
  /// The path to navigate to
  pub path: PathBuf,
  /// Enable fuzzy matching
  #[arg(short, long)]
  pub fuzzy: bool,
  /// Interactive selection
  #[arg(short, long)]
  pub interactive: bool,
}

