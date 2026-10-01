use clap::Parser;
use clap::builder::styling::{Color, RgbColor, Style, Styles};
use std::ffi::OsString;
use std::path::PathBuf;

// kanagawa-dragon, as customised in ~/.config/nvim/lua/plugins/colorscheme.lua
const fn rgb(r: u8, g: u8, b: u8) -> Style {
  Style::new().fg_color(Some(Color::Rgb(RgbColor(r, g, b))))
}
const GOLD: Style = rgb(0x90, 0x69, 0x35); // headers, like nvim float titles
const NAME: Style = rgb(0xba, 0x9b, 0x7d); // commands and flags
const DIM: Style = rgb(0x7f, 0x70, 0x62); // placeholders and descriptions
const BRICK: Style = rgb(0x98, 0x53, 0x44); // errors
const MOSS: Style = rgb(0x61, 0x7f, 0x48); // valid values in error messages

const STYLES: Styles = Styles::styled()
  .header(GOLD.bold())
  .usage(GOLD.bold())
  .literal(NAME.bold())
  .placeholder(DIM)
  .error(BRICK.bold())
  .valid(MOSS)
  .invalid(BRICK.bold());

// The examples are a plain string, so they carry their own escape codes (same
// GOLD/NAME/DIM as above); clap strips them when the output isn't a terminal.
macro_rules! header {
  ($text:literal) => {
    concat!("\x1b[1;38;2;144;105;53m", $text, "\x1b[0m\n")
  };
}
macro_rules! example {
  ($cmd:literal, $what:literal) => {
    concat!("  \x1b[1;38;2;186;155;125m", $cmd, "\x1b[0m  \x1b[38;2;127;112;98m", $what, "\x1b[0m\n")
  };
}
macro_rules! key {
  ($key:literal, $what:literal) => {
    concat!("  \x1b[1;38;2;186;155;125m", $key, "\x1b[0m \x1b[38;2;127;112;98m", $what, "\x1b[0m")
  };
}

const EXAMPLES: &str = concat!(
  header!("Examples:"),
  example!("n R/n/sr ", "Rust/Nanami/src. I strike each part on its own"),
  example!("n app sup", "Application Support. Quotes are overtime"),
  example!("n conf   ", ".config. The dot is optional"),
  example!("n -f dcs ", "Documents, fuzzy. For when you can't spell it either"),
  example!("n ..     ", "one dir up, like cd .."),
  example!("n -i ../ ", "you pick how far up. I wait"),
  example!("n -i ru/ ", "Rust and everything below it, 3 levels. No further"),
  example!("n -i .   ", "the hidden dirs here. You choose"),
  example!("n        ", "home. Where I'd rather be"),
  "\n",
  header!("Selector:"),
  key!("j k", "move"),
  key!("/", "search"),
  key!("Esc", "back to the list / clock out"),
  key!("Enter", "go there"),
  key!("l h, Tab S-Tab", "browse into / up"),
);

/// 🐆 Tell me roughly where. I take you there, then I clock out.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None, bin_name = "n", styles = STYLES, after_help = EXAMPLES)]
#[command(disable_help_flag = true, disable_version_flag = true)]
pub struct WayArgs {
  /// Where to go. I match each part against the dirs starting with it. `..`, `.`, `/`
  /// and `~` work like in cd, a trailing `/` has me search deeper, none sends you home
  #[arg(num_args = 0.., value_name = "PATH")]
  path: Vec<OsString>,
  /// Fuzzy-match each part instead of 'starts with'. Sloppier input, same result
  #[arg(short, long)]
  pub fuzzy: bool,
  /// Show me every match in a searchable list and you pick. No guessing
  #[arg(short, long)]
  pub interactive: bool,
  /// This briefing
  #[arg(short, long, action = clap::ArgAction::Help)]
  help: Option<bool>,
  /// Which version of me you hired
  #[arg(short = 'V', long, action = clap::ArgAction::Version)]
  version: Option<bool>,
}

impl WayArgs {
  /// `n Application Support` is treated like `n "Application Support"`.
  pub fn path(&self) -> PathBuf {
    if self.path.is_empty() {
      return std::env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from);
    }
    PathBuf::from(self.path.join(OsString::from(" ").as_os_str()))
  }
}
