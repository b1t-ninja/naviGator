use crossterm::{
  event::{
    self, Event, KeyCode, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
  },
  execute,
  terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
  Terminal,
  backend::CrosstermBackend,
  layout::{Constraint, Direction, Layout},
  style::{Color, Modifier, Style},
  widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::cmp::Reverse;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::way::way_finder::sub_dirs;

// kanagawa-dragon, as customised in ~/.config/nvim/lua/plugins/colorscheme.lua
const TEXT: Color = Color::Rgb(0xaf, 0x94, 0x79); // ui.float.fg
const FLOAT_BG: Color = Color::Rgb(0x13, 0x10, 0x0c); // ui.float.bg
const BORDER: Color = Color::Rgb(0x41, 0x38, 0x2f); // ui.float.fg_border
const GOLD: Color = Color::Rgb(0x90, 0x69, 0x35); // ui.special, float titles
const SEL_BG: Color = Color::Rgb(0x33, 0x2b, 0x23); // ui.pmenu.bg_sel

const PROMPT: &str = "❯ ";

/// The deepest dir all `ways` share. The search matches the part after it, so
/// typing `go` doesn't hit the `goflink` in every home path.
fn common_base(ways: &[PathBuf]) -> PathBuf {
  let mut base = ways.first().cloned().unwrap_or_default();
  while !ways.iter().all(|w| w.starts_with(&base)) && base.pop() {}
  base
}

/// What the search matches for each way: the part below the shared base. The base
/// itself is often one of the ways (`w -i ru/` lists `Rust` too); it would be left
/// with an empty key and drop out on the first letter, so it keeps its own name.
fn search_keys(ways: &[PathBuf]) -> Vec<String> {
  let base = common_base(ways);
  ways
    .iter()
    .map(|p| match p.strip_prefix(&base) {
      Ok(rel) if !rel.as_os_str().is_empty() => rel.to_string_lossy().into_owned(),
      _ => p.file_name().unwrap_or(p.as_os_str()).to_string_lossy().into_owned(),
    })
    .collect()
}

/// Indices of `keys` matching `query`, best first; all of them, in order, when empty.
fn filter(matcher: &SkimMatcherV2, keys: &[String], query: &str) -> Vec<usize> {
  if query.is_empty() {
    return (0..keys.len()).collect();
  }
  let mut hits: Vec<(i64, usize)> = keys
    .iter()
    .enumerate()
    .filter_map(|(i, key)| matcher.fuzzy_match(key, query).map(|score| (score, i)))
    .collect();
  // Stable, so equal scores keep the finder's order (top level first).
  hits.sort_by_key(|(score, _)| Reverse(*score));
  hits.into_iter().map(|(_, i)| i).collect()
}

/// What browsing into `dir` shows: the dir itself (so Enter can still pick it),
/// then its sub dirs by name, hidden ones last.
fn listing(dir: &Path) -> Vec<PathBuf> {
  let mut subs: Vec<PathBuf> = sub_dirs(dir).collect();
  subs.sort_by_cached_key(|p| {
    let name = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
    (name.starts_with('.'), name)
  });
  std::iter::once(dir.to_path_buf()).chain(subs).collect()
}

pub fn select_way(ways: &[PathBuf]) -> io::Result<Option<PathBuf>> {
  let mut tty = fs::OpenOptions::new()
    .read(true)
    .write(true)
    .open("/dev/tty")?;

  enable_raw_mode()?;
  let _restore = RestoreTerminal;
  // Lets terminals that speak the kitty protocol report Shift/Ctrl+Enter; others
  // ignore it and send a plain Enter, so Tab/Shift+Tab do the same there.
  execute!(tty, EnterAlternateScreen, PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES))?;

  let backend = CrosstermBackend::new(&mut tty);
  let mut terminal = Terminal::new(backend)?;

  let matcher = SkimMatcherV2::default();
  let mut ways = ways.to_vec();
  // Set once browsing: the dir whose listing `ways` is.
  let mut browsed: Option<PathBuf> = None;
  let mut keys = search_keys(&ways);
  let mut query = String::new();
  let mut shown = filter(&matcher, &keys, &query);
  let mut state = ListState::default();
  state.select(Some(0));
  // The list has focus until `/`; Esc in the search bar hands it back.
  let mut searching = false;

  loop {
    terminal.draw(|f| {
      let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(f.area());
      // The focused box gets the gold border, like a focused nvim float.
      let block = |title: String, focused: bool| {
        Block::default()
          .title(title)
          .title_style(Style::default().fg(GOLD))
          .borders(Borders::ALL)
          .border_style(Style::default().fg(if focused { GOLD } else { BORDER }))
      };

      let hint = if searching { "Search 🔍 (Esc: back to list) " } else { "Search 🔍 (/ to type) " };
      let search = Paragraph::new(format!("{PROMPT}{query}"))
        .style(Style::default().fg(TEXT).bg(FLOAT_BG))
        .block(block(hint.to_string(), searching));
      f.render_widget(search, chunks[0]);
      if searching {
        let typed = (PROMPT.chars().count() + query.chars().count()) as u16;
        f.set_cursor_position((chunks[0].x + 1 + typed, chunks[0].y + 1));
      }

      let items: Vec<ListItem> = shown
        .iter()
        .map(|&i| ListItem::new(ways[i].to_string_lossy().to_string()))
        .collect();
      let title = format!("Select your desired path 🐆 {}/{} ", shown.len(), ways.len());
      let list = List::new(items)
        .style(Style::default().fg(TEXT).bg(FLOAT_BG))
        .block(block(title, !searching))
        .highlight_style(Style::default().bg(SEL_BG).add_modifier(Modifier::BOLD))
        .highlight_symbol("➜ ")
        .repeat_highlight_symbol(true);
      f.render_stateful_widget(list, chunks[1], &mut state);
    })?;

    let Event::Key(key) = event::read()? else {
      continue;
    };
    if key.kind != KeyEventKind::Press {
      continue;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let step = |i: usize, up: bool| match (shown.len(), up) {
      (0, _) => None,
      (n, true) => Some(if i == 0 { n - 1 } else { i - 1 }),
      (n, false) => Some(if i + 1 >= n { 0 } else { i + 1 }),
    };
    let current = state.selected().unwrap_or(0);
    let pick = || state.selected().and_then(|s| shown.get(s)).map(|&i| ways[i].clone());
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    // Into the highlighted dir, or up from the browsed one (from the highlighted
    // one before browsing); the dir we came from stays highlighted.
    let (into, back) = match key.code {
      KeyCode::Enter if shift => (true, false),
      KeyCode::Enter if ctrl => (false, true),
      KeyCode::Tab | KeyCode::Right => (true, false),
      KeyCode::BackTab | KeyCode::Left => (false, true),
      KeyCode::Char('l') if !searching => (true, false),
      KeyCode::Char('h') if !searching => (false, true),
      _ => (false, false),
    };
    if into || back {
      let Some(way) = pick() else { continue };
      let from = browsed.clone().unwrap_or(way.clone());
      let (dir, highlight) = if into {
        (way, None)
      } else {
        match from.parent() {
          Some(up) => (up.to_path_buf(), Some(from.clone())),
          None => continue,
        }
      };
      ways = listing(&dir);
      keys = search_keys(&ways);
      browsed = Some(dir);
      query.clear();
      shown = filter(&matcher, &keys, &query);
      state.select(Some(highlight.and_then(|h| ways.iter().position(|w| *w == h)).unwrap_or(0)));
      continue;
    }
    // Keys that do the same in both modes.
    match key.code {
      KeyCode::Char('c') if ctrl => return Ok(None),
      KeyCode::Enter => match pick() {
        Some(way) => return Ok(Some(way)),
        None => continue,
      },
      KeyCode::Up => {
        state.select(step(current, true));
        continue;
      }
      KeyCode::Char('k' | 'p') if ctrl => {
        state.select(step(current, true));
        continue;
      }
      KeyCode::Down => {
        state.select(step(current, false));
        continue;
      }
      KeyCode::Char('j' | 'n') if ctrl => {
        state.select(step(current, false));
        continue;
      }
      _ => {}
    }

    if !searching {
      match key.code {
        KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
        KeyCode::Char('/') => searching = true,
        KeyCode::Char('k') => state.select(step(current, true)),
        KeyCode::Char('j') => state.select(step(current, false)),
        _ => {}
      }
      continue;
    }

    match key.code {
      KeyCode::Esc => {
        searching = false;
        continue;
      }
      KeyCode::Char('u') if ctrl => query.clear(),
      KeyCode::Char(_) if ctrl => continue,
      KeyCode::Char(c) => query.push(c),
      KeyCode::Backspace => {
        query.pop();
      }
      _ => continue,
    }
    shown = filter(&matcher, &keys, &query);
    state.select((!shown.is_empty()).then_some(0));
  }
}

/// Restores the terminal on every exit path, including early `?` returns.
struct RestoreTerminal;

impl Drop for RestoreTerminal {
  fn drop(&mut self) {
    if let Ok(mut tty) = fs::OpenOptions::new().write(true).open("/dev/tty") {
      let _ = execute!(tty, PopKeyboardEnhancementFlags, LeaveAlternateScreen);
    }
    let _ = disable_raw_mode();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn filters_below_the_shared_base() {
    let ways = ["/home/goflink/Go", "/home/goflink/Rust", "/home/goflink/Rust/cargo"].map(PathBuf::from);
    assert_eq!(common_base(&ways), PathBuf::from("/home/goflink"));
    let keys: Vec<String> = ["Go", "Rust", "Rust/cargo"].map(String::from).to_vec();
    let matcher = SkimMatcherV2::default();
    assert_eq!(filter(&matcher, &keys, ""), vec![0, 1, 2]);
    // `goflink` is in the base, so only real hits remain.
    assert_eq!(filter(&matcher, &keys, "go"), vec![0, 2]);
    assert!(filter(&matcher, &keys, "zzz").is_empty());

    // `w -i ru/`: the base is one of the ways and must stay findable by its name.
    let ways = ["/m/Rust", "/m/Rust/cargo", "/m/Rust/cargo/src"].map(PathBuf::from);
    assert_eq!(search_keys(&ways), ["Rust", "cargo", "cargo/src"]);
    assert_eq!(search_keys(&["/".into(), "/Users".into()]), ["/", "Users"]);

    // Browsing: the dir itself, then sub dirs by name, hidden last.
    let root = std::env::temp_dir().join(format!("way-listing-{}", std::process::id()));
    for d in [".git", "b", "A"] {
      fs::create_dir_all(root.join(d)).unwrap();
    }
    let listed = listing(&root);
    let _ = fs::remove_dir_all(&root);
    assert_eq!(listed, ["", "A", "b", ".git"].map(|d| root.join(d)));
  }
}
