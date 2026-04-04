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

  fn find_all_sub_dirs(&self, base_path: &PathBuf) -> impl Iterator<Item = PathBuf> {
    std::fs::read_dir(base_path)
      .into_iter()
      .flatten()
      .filter_map(|entry| entry.ok())
      .filter(|entry| entry.file_type().map_or(false, |ft| ft.is_dir()))
      .map(|entry| entry.path())
  }

  fn find_all_matching_dirs(&self) -> Vec<PathBuf> {
    let mut candidates = vec![self.args.cwd.clone()];

    for component in self.args.path.components() {
      let needle = component.as_os_str().to_string_lossy().to_lowercase();
      let mut next_candidates = Vec::new();

      for base in &candidates {
        let matches = self.find_all_sub_dirs(base)
          .filter(|full_path| {
            full_path.file_name()
              .and_then(|name| name.to_str())
              .map(|name| name.to_lowercase().starts_with(&needle))
              .unwrap_or(false)
          });
        next_candidates.extend(matches);
      }
      candidates = next_candidates;
      if candidates.is_empty() {
        break;
      }
    }

    candidates.into_iter()
      .filter_map(|p| p.strip_prefix(&self.args.cwd).ok().map(|p| p.to_path_buf()))
      .collect()
  }

  fn find_all_matching_fuzzy_dirs(&self) -> Vec<PathBuf> {
    let matcher = SkimMatcherV2::default();
    let mut candidates = vec![(0i64, self.args.cwd.clone())];

    for component in self.args.path.components() {
      let needle = component.as_os_str().to_string_lossy();
      let mut next_candidates = Vec::new();

      for (total_score, base) in &candidates {
        let matches: Vec<(i64, PathBuf)> = self.find_all_sub_dirs(base)
          .filter_map(|full_path| {
            let name = full_path.file_name()?.to_str()?;
            matcher.fuzzy_match(name, &needle).map(|score| (total_score + score, full_path))
          })
          .collect();
        next_candidates.extend(matches);
      }
      candidates = next_candidates;
      if candidates.is_empty() {
        break;
      }
    }

    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    candidates.into_iter()
      .filter_map(|(_, p)| p.strip_prefix(&self.args.cwd).ok().map(|p| p.to_path_buf()))
      .collect()
  }

  pub fn find_way(&self) -> Vec<PathBuf> {
    if self.args.fuzzy {
      self.find_all_matching_fuzzy_dirs()
    } else {
      self.find_all_matching_dirs()
    }
  }
}