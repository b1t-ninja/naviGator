use std::collections::HashMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::process;
mod args;
mod way;

use crate::args::way_args::WayArgs;
use crate::way::way_finder::find_way;
use crate::way::way_selector::select_way;
use ansi_term::{ANSIString, ANSIStrings, Colour};

// kanagawa-dragon, as customised in ~/.config/nvim/lua/plugins/colorscheme.lua
const BRICK: Colour = Colour::RGB(0x98, 0x53, 0x44); // badge background
const BADGE_FG: Colour = Colour::RGB(0x16, 0x13, 0x0f); // ui.fg_reverse
const NAME: Colour = Colour::RGB(0xba, 0x9b, 0x7d); // the path
const GOLD: Colour = Colour::RGB(0x90, 0x69, 0x35); // commands to try
const DIM: Colour = Colour::RGB(0x7f, 0x70, 0x62); // hint text
const EDGE: Colour = Colour::RGB(0x41, 0x38, 0x2f); // the │ gutter
use clap::Parser;

fn main() {
  let args = WayArgs::parse();
  let history = history_file();
  let cwd = logical_cwd();
  let ways = find_way(&args.path(), args.fuzzy, &cwd, &load_visits(&history));

  if ways.is_empty() {
    let path = args.path();
    // A typed-out file: `cd` can't enter it, so go to the folder it lives in.
    let file = cwd.join(&path);
    if let Some(dir) = file.parent().filter(|_| file.is_file()) {
      let note = [DIM.paint("That's a file, not a folder. Taking you to "), NAME.paint(tilde(dir))];
      eprintln!("  {} {}", EDGE.paint("│"), ANSIStrings(&note));
      let _ = std::io::stdout().write_all(&[dir.as_os_str().as_bytes(), b"\n"].concat());
      return;
    }
    // An absolute path ignores the cwd, so don't claim we searched from it.
    let from = if path.is_absolute() { PathBuf::from("/") } else { cwd.clone() };
    let tried = if args.fuzzy { "fuzzy" } else { "starts with" };
    let hint = if args.fuzzy {
      [DIM.paint("Look for yourself with "), GOLD.paint("n -i ./".to_string()), DIM.paint(". I don't guess")]
    } else {
      [DIM.paint("Unsure of the spelling? "), GOLD.paint(format!("n -f {}", path.display())), DIM.paint(" is more forgiving")]
    };
    let searched = [
      DIM.paint("I looked from "),
      NAME.paint(tilde(&from)),
      DIM.paint(format!(" ({tried}). Nothing. You stay where you are")),
    ];
    report("nothing there", &tilde(&path), &[&searched, &hint]);
    process::exit(1);
  }

  let way = if args.interactive && ways.len() > 1 {
    match select_way(&ways) {
      // Only explicit picks are learned: counting default jumps would keep
      // whatever already ranks first on top forever.
      Ok(Some(way)) => {
        record_visit(&history, &way);
        way
      }
      // Cancelled: non-zero, like Ctrl-C (128 + SIGINT), so `way … && cd` stays put.
      Ok(None) => process::exit(130),
      Err(e) => {
        let hint = [
          GOLD.paint("-i"),
          DIM.paint(" needs a terminal. Drop it and I just take the best match"),
        ];
        report("selector", &e.to_string(), &[&hint]);
        process::exit(1);
      }
    }
  } else {
    ways[0].clone()
  };

  // Raw bytes, not `display()`, so `cd` gets the exact name back.
  let _ = std::io::stdout().write_all(&[way.as_os_str().as_bytes(), b"\n"].concat());
}

/// ` ✖ nothing there  ~/Documents/xyz`, then one `│`-indented line per hint. Hints come
/// in painted pieces: a painted string ends in a reset, so nesting would drop colours.
fn report(badge: &str, subject: &str, hints: &[&[ANSIString]]) {
  let badge = BADGE_FG.on(BRICK).bold().paint(format!(" ✖ {badge} "));
  eprintln!("{badge} {}", NAME.bold().paint(subject));
  for hint in hints {
    eprintln!("  {} {}", EDGE.paint("│"), ANSIStrings(hint));
  }
}

/// `/Users/me/Documents` → `~/Documents`, for messages only.
fn tilde(path: &Path) -> String {
  let home = env::var_os("HOME").map(PathBuf::from);
  match home.as_deref().and_then(|h| path.strip_prefix(h).ok()) {
    Some(rest) if rest.as_os_str().is_empty() => "~".to_string(),
    Some(rest) => format!("~/{}", rest.display()),
    None => path.display().to_string(),
  }
}

/// The shell's `$PWD` keeps symlinks, so `..` goes where `cd ..` would.
fn logical_cwd() -> PathBuf {
  let physical = env::current_dir().expect("Failed to retrieve the parent process dir");
  env::var_os("PWD")
    .map(PathBuf::from)
    .filter(|pwd| pwd.canonicalize().ok() == physical.canonicalize().ok())
    .unwrap_or(physical)
}

fn history_file() -> Option<PathBuf> {
  // XDG: an empty or relative value counts as unset, else picks land in whatever dir `n` ran from.
  let state = env::var_os("XDG_STATE_HOME")
    .map(PathBuf::from)
    .filter(|p| p.is_absolute())
    .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))?;
  Some(state.join("nanami/history"))
}

/// One picked path per line, stored as raw bytes; the count per path is its rank bonus.
fn load_visits(history: &Option<PathBuf>) -> HashMap<PathBuf, i64> {
  let mut visits = HashMap::new();
  if let Some(content) = history.as_ref().and_then(|h| fs::read(h).ok()) {
    for line in content.split(|b| *b == b'\n').filter(|l| !l.is_empty()) {
      let path = PathBuf::from(std::ffi::OsString::from_vec(line.to_vec()));
      *visits.entry(path).or_insert(0) += 1;
    }
  }
  visits
}

// ponytail: append-only, grows ~100 bytes per jump; compact into counts if it ever gets big.
fn record_visit(history: &Option<PathBuf>, way: &Path) {
  let bytes = way.as_os_str().as_bytes();
  // A newline in the name would split into two bogus lines; just don't learn it.
  let Some(history) = history.as_ref().filter(|_| !bytes.contains(&b'\n')) else {
    return;
  };
  // History only improves ranking, so failing to write it must not stop the jump.
  let _ = history
    .parent()
    .map_or(Ok(()), fs::create_dir_all)
    .and_then(|_| OpenOptions::new().create(true).append(true).open(history))
    .and_then(|mut f| f.write_all(&[bytes, b"\n"].concat()));
}
