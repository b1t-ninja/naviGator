use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

/// Bonus that keeps an exact name match (plain `cd` behaviour) on top.
const EXACT_MATCH_BONUS: i64 = 1_000_000;
/// Bonus per earlier visit, capped so it never beats an exact match.
const VISIT_BONUS: i64 = 1_000;
const MAX_COUNTED_VISITS: i64 = 999;
/// Below any visible match, but one earlier visit (VISIT_BONUS) outweighs it.
const HIDDEN_PENALTY: i64 = 500;
/// Keeps the dir itself above its children for a trailing `/`, visits included.
const SELF_BONUS: i64 = 10_000_000;
/// Per level below the search dir, so for `w go/` a shallow `Go` beats a deep `go-x`.
const DEPTH_PENALTY: i64 = 100;
/// How deep a trailing `/` searches, and how many dirs at most, so `w -i ~/` stays quick.
const MAX_DEPTH: usize = 3;
const MAX_LISTED: usize = 1_000;
/// Listing order for a trailing `/`: level by level, shortest path first, hidden
/// subtrees after everything visible. Both stay under SELF_BONUS.
const LISTED_LEVEL_PENALTY: i64 = 10_000;
const LISTED_HIDDEN_PENALTY: i64 = 1_000_000;
/// Big and never where you want to go; not listed or descended into.
const SKIPPED: [&str; 3] = [".git", "node_modules", "target"];

/// Every word of `needle` starts the matching word of `name`, so
/// "app sup" and "a s" both match "Application Support".
fn word_prefix_match(name: &str, needle: &str) -> bool {
  let mut words = name.split_whitespace();
  needle.split_whitespace().all(|n| words.next().is_some_and(|w| w.starts_with(n)))
}

pub fn sub_dirs(base: &Path) -> impl Iterator<Item = PathBuf> {
  std::fs::read_dir(base)
    .into_iter()
    .flatten()
    .filter_map(|entry| entry.ok())
    .map(|entry| entry.path())
    // `is_dir` follows symlinks, so linked directories count too.
    .filter(|path| path.is_dir())
}

/// Up to `limit` sub dirs down to MAX_DEPTH levels, level by level so the cap
/// drops the deepest. Stops reading the moment the cap is hit, so `n -i ~/`
/// doesn't list all of `~/Library` just to throw it away.
fn nested_dirs(base: &Path, limit: usize) -> Vec<PathBuf> {
  let mut found = Vec::new();
  if limit == 0 {
    return found;
  }
  let mut level = vec![base.to_path_buf()];
  for _ in 0..MAX_DEPTH {
    let start = found.len();
    for dir in &level {
      for p in sub_dirs(dir).filter(|p| !p.file_name().is_some_and(|n| SKIPPED.iter().any(|s| n == *s))) {
        found.push(p);
        if found.len() == limit {
          return found;
        }
      }
    }
    level = found[start..].to_vec();
    if level.is_empty() {
      break;
    }
  }
  found
}

/// Walks `path` component by component starting at `cwd`. `/`, `.` and `..`
/// behave like in `cd`; every other component is matched against the names of
/// the sub directories (prefix or fuzzy). Directories picked before (`visits`)
/// rank higher. A trailing `.` (`w .`, `w foo/.`) matches every hidden
/// directory there instead of meaning "stay here". A path ending in `..` (`w ..`,
/// `w ../`) returns that dir, then every dir above it, so `-i` can pick how far up
/// to go. Any other trailing `/` (`w go/`) matches the last part against every dir
/// up to MAX_DEPTH levels deep instead of only the direct ones, and returns the
/// matches first, then everything inside them, top level first and shortest first.
/// Returns absolute paths, best first.
pub fn find_way(
  path: &Path,
  fuzzy: bool,
  cwd: &Path,
  visits: &HashMap<PathBuf, i64>,
) -> Vec<PathBuf> {
  let matcher = SkimMatcherV2::default();
  let mut candidates = vec![(0i64, cwd.to_path_buf())];
  // `components()` drops a trailing `.` and `/`, so look at the raw string.
  let raw = path.to_string_lossy();
  let last = path.components().count().saturating_sub(1);

  for (i, component) in path.components().enumerate() {
    match component {
      Component::CurDir => {}
      // Pushing an absolute component replaces the path, e.g. cwd + "/" = "/".
      Component::RootDir | Component::Prefix(_) => {
        candidates.iter_mut().for_each(|(_, p)| p.push(component));
      }
      Component::ParentDir => {
        candidates.iter_mut().for_each(|(_, p)| {
          p.pop();
        });
        candidates.sort_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)));
        candidates.dedup_by(|a, b| a.1 == b.1);
      }
      Component::Normal(needle) => {
        let needle = needle.to_string_lossy();
        let needle_lower = needle.to_lowercase();
        let deep = i == last && raw.ends_with('/');
        // Shared by all bases, so many earlier matches can't each list MAX_LISTED.
        let mut budget = MAX_LISTED;
        candidates = candidates
          .iter()
          .flat_map(|(total, base)| {
            // Direct sub dirs always, so a typed `.git/` still matches.
            let mut found: Vec<PathBuf> = sub_dirs(base).collect();
            if deep {
              let nested = nested_dirs(base, budget);
              budget -= nested.len();
              // Level one is in `found` already.
              found.extend(nested.into_iter().filter(|p| p.parent() != Some(base.as_path())));
            }
            found.into_iter().map(move |p| {
              let depth = p.strip_prefix(base).map_or(1, |rel| rel.components().count()) as i64;
              (*total - (depth - 1) * DEPTH_PENALTY, p)
            })
          })
          .filter_map(|(total, p)| {
            let name = p.file_name()?.to_string_lossy().into_owned();
            // Deep search would list their whole insides; only a typed-out name gets them.
            if deep && SKIPPED.contains(&name.as_str()) && name != needle {
              return None;
            }
            let score = if fuzzy {
              matcher.fuzzy_match(&name, &needle)?
            } else if word_prefix_match(&name.to_lowercase(), &needle_lower) {
              // Shorter names are closer to what was typed.
              -(name.len() as i64)
            } else if word_prefix_match(name.to_lowercase().trim_start_matches('.'), &needle_lower) {
              // Hidden dir matched without typing the dot: rank below every visible match.
              -(name.len() as i64) - HIDDEN_PENALTY
            } else {
              return None;
            };
            let bonus = if name == needle { EXACT_MATCH_BONUS } else { 0 };
            Some((total + score + bonus, p))
          })
          .collect();
      }
    }
    if candidates.is_empty() {
      break;
    }
  }

  // Going up: the target first (so a plain `w ..` is `cd ..`), then its ancestors.
  // History is not applied, it must never outrank the dir `..` points at.
  let hidden_here = raw == "." || raw.ends_with("/.");
  if !hidden_here && path.components().last() == Some(Component::ParentDir) {
    let mut ups: Vec<PathBuf> = Vec::new();
    for (_, p) in &candidates {
      for up in p.ancestors().filter(|up| !up.as_os_str().is_empty()) {
        if !ups.iter().any(|u| u == up) {
          ups.push(up.to_path_buf());
        }
      }
    }
    return ups;
  }

  if raw.ends_with('/') {
    // Best matches first, so they get the shared budget.
    candidates.sort_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)));
    candidates.dedup_by(|a, b| a.1 == b.1);
    candidates.sort_by_key(|(score, _)| Reverse(*score));
    let mut budget = MAX_LISTED;
    let children: Vec<_> = candidates
      .iter()
      .flat_map(|(_, base)| {
        let nested = nested_dirs(base, budget);
        budget -= nested.len();
        nested.into_iter().map(move |p| {
          // Inside a hidden dir counts as hidden, so the subtree stays together.
          let hidden = p.strip_prefix(base).is_ok_and(|rel| {
            rel.components().any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
          });
          let rel = p.strip_prefix(base).unwrap_or(&p);
          let depth = rel.components().count() as i64;
          let hidden = if hidden { LISTED_HIDDEN_PENALTY } else { 0 };
          // Not the match's own score: an exact match's bonus would lift its whole subtree.
          (-hidden - depth * LISTED_LEVEL_PENALTY - rel.as_os_str().len() as i64, p)
        })
      })
      .collect();
    candidates.iter_mut().for_each(|(score, _)| *score += SELF_BONUS);
    candidates.extend(children);
    // A match can also sit inside another match; keep its best score only.
    candidates.sort_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)));
    candidates.dedup_by(|a, b| a.1 == b.1);
  } else if hidden_here {
    let hidden: Vec<_> = candidates
      .iter()
      .flat_map(|(total, base)| sub_dirs(base).map(move |p| (*total, p)))
      .filter_map(|(total, p)| {
        let name = p.file_name()?.to_string_lossy().into_owned();
        name.starts_with('.').then(|| (total - name.len() as i64, p))
      })
      .collect();
    // No hidden dirs: stay put, like `cd .`.
    if !hidden.is_empty() {
      candidates = hidden;
    }
  }

  for (score, p) in &mut candidates {
    *score += visits.get(p).map_or(0, |n| (*n).min(MAX_COUNTED_VISITS) * VISIT_BONUS);
  }
  // read_dir order is unspecified, so tie-break on the path for stable results.
  candidates.sort_by(|a, b| Reverse(a.0).cmp(&Reverse(b.0)).then(a.1.cmp(&b.1)));
  candidates.into_iter().map(|(_, p)| p).collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;

  #[test]
  fn walks_like_cd() {
    // Unique per run, so no leftover dir can prefix-match the absolute-path asserts.
    let nanos = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos();
    let root = std::env::temp_dir().join(format!("way-test-{}-{nanos}", std::process::id()));
    let _cleanup = RemoveOnDrop(root.clone());
    for dir in [
      "Application Support", "apps", "src", "srcold", ".git/src", "a/b", "gitlab-stuff", "src/.cache/x",
      "Go", "code/go-tools/cmd", "code/deep/er/gone",
    ] {
      fs::create_dir_all(root.join(dir)).unwrap();
    }
    let cwd = root.join("a/b");
    let no_visits = HashMap::new();
    let find = |p: &str, fuzzy| find_way(Path::new(p), fuzzy, &cwd, &no_visits);

    let up = find("..", false);
    assert_eq!(up[..2], [root.join("a"), root.clone()]);
    assert_eq!(up.last(), Some(&PathBuf::from("/")));
    assert_eq!(find("../", false), up);
    assert_eq!(find("../..", false)[0], root);
    assert_eq!(find("../../s", false), vec![root.join("src"), root.join("srcold")]);
    assert_eq!(find("./../../src", false)[0], root.join("src"));
    assert_eq!(find(root.to_str().unwrap(), false), vec![root.clone()]);
    let abs_prefix = format!("{}/app", root.display());
    assert_eq!(find(&abs_prefix, false), vec![root.join("apps"), root.join("Application Support")]);
    assert_eq!(find("../../application s", false), vec![root.join("Application Support")]);
    assert_eq!(find("../../src", true)[0], root.join("src"));
    assert_eq!(find("../../app sup", false), vec![root.join("Application Support")]);
    assert_eq!(find("../../a s", false), vec![root.join("Application Support")]);
    assert!(find("nope", false).is_empty());
    assert_eq!(find("../../.", false), vec![root.join(".git")]);
    assert_eq!(find(".", false), vec![cwd.clone()]);
    // `go/` searches 3 levels deep: Go first, go-tools and its insides; `gone` is level 4.
    let go = find("../../go/", false);
    assert_eq!(go, ["Go", "code/go-tools", "code/go-tools/cmd"].map(|d| root.join(d)));
    assert_eq!(find("../../go", false), vec![root.join("Go")]);
    let browse = find("../../.git/", false);
    assert_eq!(browse, vec![root.join(".git"), root.join(".git/src")]);
    // Level by level, shortest first, hidden last; .git is skipped.
    let browse_root = find(&format!("{}/", root.display()), false);
    let expected = [
      "", "a", "Go", "src", "apps", "code", "srcold", "gitlab-stuff", "Application Support",
      "a/b", "code/deep", "code/go-tools", "code/deep/er", "code/go-tools/cmd", "src/.cache", "src/.cache/x",
    ];
    assert_eq!(browse_root, expected.map(|d| root.join(d)));
    assert_eq!(find("../../.g", false), vec![root.join(".git")]);
    assert_eq!(find("../../gi", false), vec![root.join("gitlab-stuff"), root.join(".git")]);
    assert_eq!(find("../../gi/sr", true), vec![root.join(".git/src")]);

    // A past pick outranks the shorter name, but not an exact one.
    let visits = HashMap::from([(root.join("Application Support"), 1), (root.join("srcold"), 5)]);
    assert_eq!(find_way(Path::new("../../app"), false, &cwd, &visits)[0], root.join("Application Support"));
    assert_eq!(find_way(Path::new("../../src"), false, &cwd, &visits)[0], root.join("src"));
  }

  #[test]
  fn deep_search_order_skips_and_cap() {
    let root = std::env::temp_dir().join(format!("way-deep-{}", std::process::id()));
    let _cleanup = RemoveOnDrop(root.clone());
    for dir in ["Go/a/b", "code/go-tools/cmd", "code/go-tools/.h", "target/debug", "tango"] {
      fs::create_dir_all(root.join(dir)).unwrap();
    }
    let no_visits = HashMap::new();
    let find = |p: &str| find_way(Path::new(p), false, &root, &no_visits);

    // The exact `Go` doesn't lift its subtree: level by level across both matches, hidden last.
    let go = ["Go", "code/go-tools", "Go/a", "code/go-tools/cmd", "Go/a/b", "code/go-tools/.h"];
    assert_eq!(find("Go/"), go.map(|d| root.join(d)));
    // `target` only when typed out.
    assert_eq!(find("ta/"), vec![root.join("tango")]);
    assert_eq!(find("target/"), vec![root.join("target"), root.join("target/debug")]);

    // One budget for all matches, not MAX_LISTED each.
    for i in 0..600 {
      fs::create_dir_all(root.join(format!("x1/{i}"))).unwrap();
      fs::create_dir_all(root.join(format!("x2/{i}"))).unwrap();
    }
    assert_eq!(find("x/").len(), 2 + MAX_LISTED);
  }

  /// Removes the test dir even when an assert panics.
  struct RemoveOnDrop(PathBuf);

  impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
      let _ = fs::remove_dir_all(&self.0);
    }
  }
}
