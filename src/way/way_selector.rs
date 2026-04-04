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
  enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen)?;
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  let mut state = ListState::default();
  state.select(Some(0));

  let items: Vec<ListItem> = ways
    .iter()
    .map(|p| ListItem::new(p.to_string_lossy().to_string()))
    .collect();

  loop {
    terminal.draw(|f| {
      let size = f.area();
      let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(size);

      let list = List::new(items.clone())
        .block(Block::default().title("Select your desired path 🔻 ").borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::Green).add_modifier(Modifier::BOLD))
        .highlight_symbol("🍓 ")
        .repeat_highlight_symbol(true);

      f.render_stateful_widget(list, chunks[0], &mut state);
    })?;

    if let Event::Key(key) = event::read()? {
      match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
          break;
        }
        KeyCode::Up | KeyCode::Char('k') => {
          let i = match state.selected() {
            Some(i) => {
              if i == 0 {
                items.len() - 1
              } else {
                i - 1
              }
            }
            None => 0,
          };
          state.select(Some(i));
        }
        KeyCode::Down | KeyCode::Char('j') => {
          let i = match state.selected() {
            Some(i) => {
              if i >= items.len() - 1 {
                0
              } else {
                i + 1
              }
            }
            None => 0,
          };
          state.select(Some(i));
        }
        KeyCode::Enter => {
          if let Some(i) = state.selected() {
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            return Ok(Some(ways[i].clone()));
          }
        }
        _ => {}
      }
    }
  }

  disable_raw_mode()?;
  execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
  Ok(None)
}
