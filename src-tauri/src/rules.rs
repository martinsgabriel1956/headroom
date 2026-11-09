use std::path::Path;

enum Need {
  Beside(&'static [&'static str]),
  Inside(&'static str),
  Always,
}

// PRD RF10. `dist` and `build` are deliberately absent (RF11).
const RULES: &[(&str, Need)] = &[
  ("node_modules", Need::Beside(&["package.json"])),
  ("target", Need::Beside(&["Cargo.toml"])),
  (".venv", Need::Inside("pyvenv.cfg")),
  ("venv", Need::Inside("pyvenv.cfg")),
  ("__pycache__", Need::Always),
  (".pytest_cache", Need::Always),
  (".next", Need::Beside(&["package.json"])),
  (".turbo", Need::Beside(&["package.json"])),
  (".gradle", Need::Beside(&["build.gradle", "build.gradle.kts", "settings.gradle"])),
];

pub struct Rule {
  pub kind: &'static str,
  pub evidence: String,
}

/// `dir` is the candidate folder. Returns the rule that approves it, checking the disk now.
pub fn match_rule(dir: &Path) -> Option<Rule> {
  let name = dir.file_name()?.to_str()?;
  let (kind, need) = RULES.iter().find(|(k, _)| *k == name)?;
  let evidence = match need {
    Need::Beside(files) => {
      let parent = dir.parent()?;
      let found = files.iter().find(|f| parent.join(f).is_file())?;
      format!("{found} ao lado")
    }
    Need::Inside(f) => dir.join(f).is_file().then(|| format!("{f} dentro"))?,
    Need::Always => "sempre cache".into(),
  };
  Some(Rule { kind, evidence })
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;

  fn evidence(root: &Path, dir: &str) -> Option<String> {
    let d = root.join(dir);
    fs::create_dir_all(&d).unwrap();
    match_rule(&d).map(|r| r.evidence)
  }

  #[test]
  fn each_rule_needs_its_evidence() {
    let t = crate::test_dir("rules");
    // Without evidence nothing matches.
    for d in ["a/node_modules", "a/target", "a/.venv", "a/venv", "a/.next", "a/.turbo", "a/.gradle"] {
      assert_eq!(evidence(&t, d), None, "{d}");
    }
    fs::write(t.join("a/package.json"), "{}").unwrap();
    fs::write(t.join("a/Cargo.toml"), "").unwrap();
    fs::write(t.join("a/.venv/pyvenv.cfg"), "").unwrap();
    fs::write(t.join("a/venv/pyvenv.cfg"), "").unwrap();
    fs::write(t.join("a/settings.gradle"), "").unwrap();
    assert_eq!(evidence(&t, "a/node_modules").as_deref(), Some("package.json ao lado"));
    assert_eq!(evidence(&t, "a/target").as_deref(), Some("Cargo.toml ao lado"));
    assert_eq!(evidence(&t, "a/.venv").as_deref(), Some("pyvenv.cfg dentro"));
    assert_eq!(evidence(&t, "a/venv").as_deref(), Some("pyvenv.cfg dentro"));
    assert_eq!(evidence(&t, "a/.next").as_deref(), Some("package.json ao lado"));
    assert_eq!(evidence(&t, "a/.turbo").as_deref(), Some("package.json ao lado"));
    assert_eq!(evidence(&t, "a/.gradle").as_deref(), Some("settings.gradle ao lado"));
    assert_eq!(evidence(&t, "b/__pycache__").as_deref(), Some("sempre cache"));
    assert_eq!(evidence(&t, "b/.pytest_cache").as_deref(), Some("sempre cache"));
    // dist and build never match, even beside a manifest.
    assert_eq!(evidence(&t, "a/dist"), None);
    assert_eq!(evidence(&t, "a/build"), None);
  }
}
