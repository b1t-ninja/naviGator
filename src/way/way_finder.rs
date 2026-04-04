use std::path::PathBuf;
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
}

impl WayFinder {
  pub fn find_all_dirs_in_cwd(&self) -> Vec<PathBuf> {
    std::fs::read_dir(&self.args.cwd)
      .unwrap()
      .filter_map(|entry| entry.ok())
      .filter(|entry| entry.file_type().unwrap().is_dir())
      .map(|entry| {
        entry
          .path()
          .strip_prefix(&self.args.cwd)
          .unwrap()
          .to_path_buf()
      })
      .collect()
  }

  fn find_all_matching_dirs(&self) -> Vec<PathBuf> {
    let needle = self.args.path.to_string_lossy();

    self.find_all_dirs_in_cwd()
      .into_iter()
      .filter(|dir| {
        dir.file_name()
          .and_then(|name| name.to_str())
          .is_some_and(|name| name.to_lowercase().starts_with(needle.as_ref()))
      })
      .collect()
  }

  pub fn find_way(&self) -> String {
    let matches = self.find_all_matching_dirs();
    matches.first().unwrap_or_else(|| panic!("No matching directories found for {:?}", self.args.path)).to_string_lossy().to_string()
  }
}