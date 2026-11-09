use crate::{rules::match_rule, scan::Found};
use std::{fs, os::unix::fs::MetadataExt, path::Path};

/// Checks the disk as it is now, not as the scan saw it (PRD RF21). `root` is the canonical scan root.
pub fn revalidate(root: &Path, dev: u64, f: &Found) -> Result<(), String> {
  let md = fs::symlink_metadata(&f.path).map_err(|_| "a pasta não existe mais")?;
  if md.file_type().is_symlink() {
    return Err("virou link simbólico".into());
  }
  if !md.is_dir() {
    return Err("não é mais uma pasta".into());
  }
  if md.dev() != dev {
    return Err("está em outro disco".into());
  }
  let parent = f.path.parent().and_then(|p| fs::canonicalize(p).ok()).ok_or("caminho inválido")?;
  if !parent.starts_with(root) {
    return Err("saiu da pasta varrida".into());
  }
  match match_rule(&f.path) {
    Some(r) if r.kind == f.kind => Ok(()),
    _ => Err("a regra não vale mais".into()),
  }
}

/// Deletes for good. `remove_dir_all` does not follow symlinks inside the tree.
pub fn remove(f: &Found) -> Result<(), String> {
  fs::remove_dir_all(&f.path).map_err(|e| match f.path.exists() {
    true => format!("apagado em parte: {e}"),
    false => e.to_string(),
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::measure::Measured;
  use std::{os::unix::fs::symlink, path::PathBuf};

  fn found(path: PathBuf) -> Found {
    Found { id: 1, project: path.parent().unwrap().into(), path, kind: "node_modules", rule: String::new(), measured: Measured::default() }
  }
  fn project(t: &Path, name: &str) -> Found {
    let p = t.join(name);
    fs::create_dir_all(p.join("node_modules/pkg")).unwrap();
    fs::write(p.join("package.json"), "{}").unwrap();
    fs::write(p.join("node_modules/pkg/index.js"), "x").unwrap();
    found(p.join("node_modules"))
  }

  #[test]
  fn refuses_what_changed_and_deletes_what_holds() {
    let t = fs::canonicalize(crate::test_dir("delete")).unwrap();
    let dev = fs::metadata(&t).unwrap().dev();

    let ok = project(&t, "ok");
    assert_eq!(revalidate(&t, dev, &ok), Ok(()));
    // A symlink inside the artifact must not take its target with it.
    fs::create_dir_all(t.join("keep")).unwrap();
    fs::write(t.join("keep/precious"), "x").unwrap();
    symlink(t.join("keep"), ok.path.join("link")).unwrap();
    remove(&ok).unwrap();
    assert!(!ok.path.exists());
    assert!(t.join("keep/precious").exists());

    let no_manifest = project(&t, "no-manifest");
    fs::remove_file(t.join("no-manifest/package.json")).unwrap();
    assert_eq!(revalidate(&t, dev, &no_manifest), Err("a regra não vale mais".into()));

    let gone = found(t.join("gone/node_modules"));
    assert_eq!(revalidate(&t, dev, &gone), Err("a pasta não existe mais".into()));

    let swapped = project(&t, "swapped");
    fs::remove_dir_all(&swapped.path).unwrap();
    symlink(t.join("keep"), &swapped.path).unwrap();
    assert_eq!(revalidate(&t, dev, &swapped), Err("virou link simbólico".into()));

    let outside = project(&t, "outside");
    assert_eq!(revalidate(&t.join("other-root"), dev, &outside), Err("saiu da pasta varrida".into()));
  }
}
