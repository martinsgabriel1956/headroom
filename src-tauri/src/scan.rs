use crate::{
  measure::{measure, Measured},
  rules::{match_rule, Rule},
};
use std::{
  collections::HashMap,
  fs,
  os::unix::fs::MetadataExt,
  path::{Path, PathBuf},
  sync::{
    atomic::{AtomicU32, AtomicU64, Ordering::Relaxed},
    Mutex,
  },
};

pub struct Found {
  pub id: u32,
  pub path: PathBuf,
  pub project: PathBuf,
  pub kind: &'static str,
  pub rule: String,
  pub measured: Measured,
}

#[derive(Default)]
pub struct Counters {
  pub dirs: AtomicU64,
  pub skipped: AtomicU64,
}

struct Ctx<'a> {
  root: &'a Path,
  dev: u64,
  cutoff: i64,
  counters: &'a Counters,
  cancel: &'a (dyn Fn() -> bool + Sync),
  found: &'a (dyn Fn(Found) + Sync),
  next_id: AtomicU32,
  recent: Mutex<Vec<(PathBuf, i64)>>,
}

/// Walks `root` (already canonical) in parallel, calling `found` from many threads for each artifact,
/// already measured. Returns the directories outside artifacts holding a file modified after `cutoff`
/// (unix seconds), with the newest mtime of each.
pub fn scan(
  root: &Path,
  cutoff: i64,
  counters: &Counters,
  cancel: &(dyn Fn() -> bool + Sync),
  found: &(dyn Fn(Found) + Sync),
) -> Vec<(PathBuf, i64)> {
  let Ok(md) = fs::metadata(root) else {
    counters.skipped.fetch_add(1, Relaxed);
    return vec![];
  };
  let ctx = Ctx {
    root,
    dev: md.dev(),
    cutoff,
    counters,
    cancel,
    found,
    next_id: AtomicU32::new(1),
    recent: Mutex::default(),
  };
  rayon::scope(|s| visit(s, &ctx, root.to_path_buf()));
  ctx.recent.into_inner().unwrap()
}

fn visit<'s, 'a: 's>(s: &rayon::Scope<'s>, ctx: &'s Ctx<'a>, dir: PathBuf) {
  if (ctx.cancel)() {
    return;
  }
  let Ok(entries) = fs::read_dir(&dir) else {
    ctx.counters.skipped.fetch_add(1, Relaxed);
    return;
  };
  ctx.counters.dirs.fetch_add(1, Relaxed);
  let mut newest = 0;
  for e in entries.flatten() {
    let Ok(md) = e.metadata() else { continue }; // DirEntry::metadata does not follow symlinks
    if md.is_file() {
      newest = newest.max(md.mtime());
      continue;
    }
    // Symlinks, other filesystems and git's object store are never entered (PRD RF7, techspec).
    if !md.is_dir() || md.dev() != ctx.dev || e.file_name() == ".git" {
      continue;
    }
    let path = e.path();
    match match_rule(&path) {
      Some(rule) => s.spawn(move |_| emit(ctx, path, rule)),
      None => s.spawn(move |s| visit(s, ctx, path)),
    }
  }
  if newest >= ctx.cutoff {
    ctx.recent.lock().unwrap().push((dir, newest));
  }
}

fn emit(ctx: &Ctx, path: PathBuf, rule: Rule) {
  let measured = measure(&path, ctx.dev, ctx.cancel);
  if (ctx.cancel)() {
    return;
  }
  let project = project_of(&path, ctx.root);
  (ctx.found)(Found { id: ctx.next_id.fetch_add(1, Relaxed), path, project, kind: rule.kind, rule: rule.evidence, measured });
}

/// Nearest folder with a `.git` between the artifact and the root; without git, the artifact's parent.
pub fn project_of(artifact: &Path, root: &Path) -> PathBuf {
  let parent = artifact.parent().unwrap_or(root);
  parent
    .ancestors()
    .take_while(|a| a.starts_with(root))
    .find(|a| a.join(".git").exists())
    .unwrap_or(parent)
    .to_path_buf()
}

/// For each project, the newest mtime among the recent directories under it. Projects without one are absent.
pub fn active_projects<'a>(recent: &[(PathBuf, i64)], projects: impl IntoIterator<Item = &'a Path>) -> HashMap<PathBuf, i64> {
  let mut out = HashMap::new();
  for p in projects {
    if let Some(t) = recent.iter().filter(|(d, _)| d.starts_with(p)).map(|(_, t)| *t).max() {
      out.insert(p.to_path_buf(), t);
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::{os::unix::fs::{symlink, PermissionsExt}, process::Command, time::SystemTime};

  fn now() -> i64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64
  }
  fn mk(root: &Path, files: &[&str]) {
    for f in files {
      let p = root.join(f);
      fs::create_dir_all(p.parent().unwrap()).unwrap();
      fs::write(p, "x").unwrap();
    }
  }
  fn age(p: &Path) {
    Command::new("touch").args(["-d", "30 days ago"]).arg(p).status().unwrap();
  }
  fn run(root: &Path) -> (Vec<Found>, Vec<(PathBuf, i64)>, Counters) {
    let found = Mutex::new(vec![]);
    let counters = Counters::default();
    let recent = scan(root, now() - 7 * 86_400, &counters, &|| false, &|f| found.lock().unwrap().push(f));
    (found.into_inner().unwrap(), recent, counters)
  }

  #[test]
  fn finds_artifacts_with_the_right_cuts() {
    let t = fs::canonicalize(crate::test_dir("scan")).unwrap();
    mk(&t, &[
      "p1/.git/HEAD",
      "p1/package.json",
      "p1/node_modules/x/package.json",
      "p1/node_modules/x/node_modules/y/index.js", // nested artifact: not reported
      "p1/src-tauri/Cargo.toml",
      "p1/src-tauri/target/debug/bin",
      "p1/.git/package.json",
      "p1/.git/node_modules/z", // inside .git: never visited
      "p2/package.json",
      "p2/node_modules/fresh.js",
      ".cache/tool/.venv/pyvenv.cfg", // hidden folders are scanned
    ]);
    symlink(t.join("p2"), t.join("link-to-p2")).unwrap(); // not followed: p2 is found once
    age(&t.join("p2/package.json"));
    let locked = t.join("locked");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

    let (found, recent, counters) = run(&t);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

    let mut got: Vec<_> = found.iter().map(|f| (f.path.strip_prefix(&t).unwrap().to_owned(), f.project.strip_prefix(&t).unwrap().to_owned())).collect();
    got.sort();
    let want: Vec<(PathBuf, PathBuf)> = [
      (".cache/tool/.venv", ".cache/tool"),
      ("p1/node_modules", "p1"),
      ("p1/src-tauri/target", "p1"), // project found through p1/.git
      ("p2/node_modules", "p2"),     // no git: the parent
    ]
    .iter()
    .map(|(a, b)| (PathBuf::from(a), PathBuf::from(b)))
    .collect();
    assert_eq!(got, want);

    // p1 has a fresh file outside artifacts; p2's only fresh file is inside node_modules.
    let active = active_projects(&recent, [t.join("p1").as_path(), t.join("p2").as_path()]);
    assert!(active.contains_key(&t.join("p1")));
    assert!(!active.contains_key(&t.join("p2")));

    let root_user = Command::new("id").arg("-u").output().unwrap().stdout == b"0\n";
    if !root_user {
      assert!(counters.skipped.load(Relaxed) >= 1, "locked dir counted as skipped");
    }
    let ids: std::collections::HashSet<_> = found.iter().map(|f| f.id).collect();
    assert_eq!(ids.len(), found.len(), "ids are unique");
  }

  /// Manual performance check (PRD goal): `cargo test -- --ignored --nocapture timing`.
  #[test]
  #[ignore]
  fn timing() {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap());
    for root in [home.join("Documentos/development"), home] {
      let t0 = std::time::Instant::now();
      let first = Mutex::new(None);
      let n = AtomicU32::new(0);
      let counters = Counters::default();
      let listed = root.ends_with("development");
      scan(&root, now() - 7 * 86_400, &counters, &|| false, &|f| {
        if listed {
          println!("  {} ({})", f.path.display(), f.rule); // audit list for the zero-false-positive goal
        }
        first.lock().unwrap().get_or_insert(t0.elapsed());
        n.fetch_add(1, Relaxed);
      });
      println!(
        "{}: {} artefatos, primeiro em {:?}, total {:?}, {} pastas, {} puladas",
        root.display(),
        n.load(Relaxed),
        first.lock().unwrap(),
        t0.elapsed(),
        counters.dirs.load(Relaxed),
        counters.skipped.load(Relaxed)
      );
    }
  }
}
