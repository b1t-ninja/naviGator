use std::path::PathBuf;
use clap::Parser;

/// Find your way around the terminal with ease 🧭
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct WayArgs {
  /// The current working directory
  #[arg(short, long)]
  pub cwd: PathBuf,
  /// The path to navigate to
  pub path: PathBuf,
}

