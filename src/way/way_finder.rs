use std::path::PathBuf;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use crate::args::way_args::WayArgs;

#[derive(Debug)]
pub struct WayFinder {
  args: WayArgs
}

/// Creates a new WayFinder instance with the provided arguments.
impl WayFinder {
  pub fn new(args: WayArgs) -> Self {
    Self { args }
  }

  pub fn find_all_dirs_in_cwd(&self) -> impl Iterator<Item = PathBuf> + '_ {
    std::fs::read_dir(&self.args.cwd)
      .into_iter()
      .flatten()
      .filter_map(|entry| entry.ok())
      .filter(|entry| entry.file_type().map_or(false, |ft| ft.is_dir()))
      .filter_map(|entry| {
        entry
          .path()
          .strip_prefix(&self.args.cwd)
          .ok()
          .map(|p| p.to_path_buf())
      })
  }

  fn find_all_matching_dirs(&self) -> Vec<PathBuf> {
    let needle = self.args.path.to_string_lossy().to_lowercase();

    self.find_all_dirs_in_cwd()
      .filter(|dir| {
        dir.file_name()
          .and_then(|name| name.to_str())
          .map(|name| name.to_lowercase().starts_with(&needle))
          .unwrap_or(false)
      })
      .collect()
  }

  fn find_all_matching_fuzzy_dirs(&self) -> Vec<PathBuf> {
  let matcher = SkimMatcherV2::default();
  let needle = self.args.path.to_string_lossy();

  let mut scored: Vec<(i64, PathBuf)> = self
  .find_all_dirs_in_cwd()
  .filter_map(|dir| {
  let name = dir.file_name()?.to_str()?;
  matcher.fuzzy_match(name, &needle).map(|score| (score, dir))
  })
  .collect();

  scored.sort_by(|a, b| b.0.cmp(&a.0));
  scored.into_iter().map(|(_, dir)| dir).collect()
  }

  pub fn find_way(&self) -> Vec<PathBuf> {
    if self.args.fuzzy {
      self.find_all_matching_fuzzy_dirs()
    } else {
      self.find_all_matching_dirs()
    }
  }
}