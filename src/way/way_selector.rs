use std::fs;
use std::io;
use std::path::PathBuf;
use crossterm::{
  event::{self, Event, KeyCode},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
  backend::CrosstermBackend,
  layout::{Constraint, Direction, Layout},
  style::{Color, Modifier, Style},
  widgets::{Block, Borders, List, ListItem, ListState},
  Terminal,
};

pub fn select_way(ways: &[PathBuf]) -> io::Result<Option<PathBuf>> {
  let mut tty = fs::OpenOptions::new()
    .read(true)
    .write(true)
    .open("/dev/tty")?;

  enable_raw_mode()?;
  execute!(tty, EnterAlternateScreen)?;

  let backend = CrosstermBackend::new(&mut tty);
  let mut terminal = Terminal::new(backend)?;

  let mut state = ListState::default();
  state.select(Some(0));

  let result = (|| -> io::Result<Option<PathBuf>> {
    loop {
      terminal.draw(|f| {
        let size = f.area();
        let chunks = Layout::default()
          .direction(Direction::Vertical)
          .constraints([Constraint::Percentage(100)])
          .split(size);

        let list_items: Vec<ListItem> = ways
          .iter()
          .map(|p| ListItem::new(p.to_string_lossy().to_string()))
          .collect();

        let list = List::new(list_items)
          .block(Block::default().title("Select your desired path 🔻 ").borders(Borders::ALL))
          .highlight_style(Style::default().bg(Color::Green).add_modifier(Modifier::BOLD))
          .highlight_symbol("🍓 ")
          .repeat_highlight_symbol(true);

        f.render_stateful_widget(list, chunks[0], &mut state);
      })?;

      if let Event::Key(key) = event::read()? {
        match key.code {
          KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
          KeyCode::Up | KeyCode::Char('k') => {
            let i = state.selected().unwrap_or(0);
            let new_i = if i == 0 { ways.len() - 1 } else { i - 1 };
            state.select(Some(new_i));
          }
          KeyCode::Down | KeyCode::Char('j') => {
            let i = state.selected().unwrap_or(0);
            let new_i = if i >= ways.len() - 1 { 0 } else { i + 1 };
            state.select(Some(new_i));
          }
          KeyCode::Enter => {
            return Ok(state.selected().map(|i| ways[i].clone()));
          }
          _ => {}
        }
      }
    }
  })();

  execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
  disable_raw_mode()?;

  result
}
