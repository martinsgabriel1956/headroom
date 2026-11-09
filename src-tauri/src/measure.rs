use rayon::prelude::*;
use std::{collections::HashMap, fs, os::unix::fs::MetadataExt, path::Path};

pub type Inode = (u64, u64); // (st_dev, st_ino)

#[derive(Default, Clone, Debug)]
pub struct Measured {
  pub apparent: u64,
  /// Blocks of directories and of files with a single link: freed whenever this folder goes.
  pub single: u64,
  /// Newest mtime seen, in unix seconds.
  pub newest: i64,
  /// Files with several links: (links inside this folder, st_nlink, bytes).
  pub linked: HashMap<Inode, (u64, u64, u64)>,
}

impl Measured {
  fn add(&mut self, md: &fs::Metadata) {
    let bytes = md.blocks() * 512;
    self.newest = self.newest.max(md.mtime());
    if !md.is_dir() {
      self.apparent += md.len();
    }
    if md.is_dir() || md.nlink() == 1 {
      self.single += bytes;
    } else {
      self.linked.entry((md.dev(), md.ino())).or_insert((0, md.nlink(), bytes)).0 += 1;
    }
  }

  fn merge(mut self, o: Measured) -> Measured {
    self.apparent += o.apparent;
    self.single += o.single;
    self.newest = self.newest.max(o.newest);
    for (k, (c, n, b)) in o.linked {
      self.linked.entry(k).or_insert((0, n, b)).0 += c;
    }
    self
  }

  pub fn real(&self) -> u64 {
    real_of([self])
  }
}

/// Bytes freed by deleting every folder in `set` together: a multi-linked file counts only
/// when all its links are inside the set (PRD RF16, RF19).
pub fn real_of<'a>(set: impl IntoIterator<Item = &'a Measured>) -> u64 {
  let mut single = 0;
  let mut links: HashMap<Inode, (u64, u64, u64)> = HashMap::new();
  for m in set {
    single += m.single;
    for (k, (c, n, b)) in &m.linked {
      links.entry(*k).or_insert((0, *n, *b)).0 += c;
    }
  }
  single + links.values().filter(|(c, n, _)| c >= n).map(|(_, _, b)| b).sum::<u64>()
}

/// Measures `dir` without following symlinks or leaving device `dev`. Stops early if `cancel()`.
pub fn measure(dir: &Path, dev: u64, cancel: &(dyn Fn() -> bool + Sync)) -> Measured {
  let mut m = Measured::default();
  if let Ok(md) = fs::symlink_metadata(dir) {
    m.add(&md);
  }
  m.merge(walk(dir, dev, cancel))
}

fn walk(dir: &Path, dev: u64, cancel: &(dyn Fn() -> bool + Sync)) -> Measured {
  if cancel() {
    return Measured::default();
  }
  let Ok(entries) = fs::read_dir(dir) else { return Measured::default() };
  let entries: Vec<_> = entries.flatten().collect();
  entries
    .par_iter()
    .map(|e| {
      let mut m = Measured::default();
      let Ok(md) = e.metadata() else { return m }; // DirEntry::metadata does not follow symlinks
      m.add(&md);
      if md.is_dir() && md.dev() == dev {
        m = m.merge(walk(&e.path(), dev, cancel));
      }
      m
    })
    .reduce(Measured::default, Measured::merge)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::os::unix::fs::symlink;

  fn dev(p: &Path) -> u64 {
    fs::metadata(p).unwrap().dev()
  }
  const NO: &(dyn Fn() -> bool + Sync) = &|| false;

  #[test]
  fn apparent_and_real() {
    let t = crate::test_dir("measure");
    let a = t.join("a");
    fs::create_dir_all(a.join("sub")).unwrap();
    fs::write(a.join("x"), vec![1u8; 10_000]).unwrap();
    fs::write(a.join("sub/y"), vec![1u8; 5_000]).unwrap();
    let m = measure(&a, dev(&t), NO);
    assert_eq!(m.apparent, 15_000);
    assert!(m.real() >= 15_000, "real counts blocks: {}", m.real());
    assert!(m.newest > 0);
  }

  #[test]
  fn hardlinks_count_only_when_all_links_are_in_the_set() {
    let t = crate::test_dir("links");
    let (a, b, store) = (t.join("a"), t.join("b"), t.join("store"));
    for d in [&a, &b, &store] {
      fs::create_dir_all(d).unwrap();
    }
    // Linked to a store outside: never freed (pnpm case).
    fs::write(store.join("pkg"), vec![1u8; 40_000]).unwrap();
    fs::hard_link(store.join("pkg"), a.join("pkg")).unwrap();
    // Two links inside `a`: freed once.
    fs::write(a.join("twin1"), vec![1u8; 40_000]).unwrap();
    fs::hard_link(a.join("twin1"), a.join("twin2")).unwrap();
    // Shared between `a` and `b`: freed only when both are in the set.
    fs::write(a.join("shared"), vec![1u8; 80_000]).unwrap();
    fs::hard_link(a.join("shared"), b.join("shared")).unwrap();

    let d = dev(&t);
    let ma = measure(&a, d, NO);
    let mb = measure(&b, d, NO);
    let dirs = ma.single; // only the directory itself is single-linked in `a`
    let block = |n: u64| fs::metadata(a.join(if n == 40 { "twin1" } else { "shared" })).unwrap().blocks() * 512;
    assert_eq!(ma.real(), dirs + block(40), "twin counted once, pkg and shared not counted");
    assert_eq!(mb.real(), mb.single, "shared alone is not freed");
    assert_eq!(real_of([&ma, &mb]), dirs + mb.single + block(40) + block(80), "shared freed with both");
    assert_eq!(ma.apparent, 40_000 * 3 + 80_000);
  }

  #[test]
  fn symlinks_are_not_followed() {
    let t = crate::test_dir("symlink");
    fs::create_dir_all(t.join("a")).unwrap();
    fs::create_dir_all(t.join("big")).unwrap();
    fs::write(t.join("big/f"), vec![1u8; 100_000]).unwrap();
    symlink(t.join("big"), t.join("a/dir")).unwrap();
    symlink(t.join("big/f"), t.join("a/file")).unwrap();
    let m = measure(&t.join("a"), dev(&t), NO);
    assert!(m.apparent < 1_000, "only the link sizes: {}", m.apparent);
  }
}
