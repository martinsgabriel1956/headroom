use std::{
  fs,
  path::PathBuf,
  process::{Command, Stdio},
};
use tauri::AppHandle;

static ICON: &[u8] = include_bytes!("../icons/128x128.png");

/// notify-send wants an icon file: write it once under $XDG_RUNTIME_DIR.
fn icon_path() -> Option<PathBuf> {
  let dir = PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?).join("headroom");
  let path = dir.join("icon.png");
  if !path.exists() {
    fs::create_dir_all(&dir).ok()?;
    fs::write(&path, ICON).ok()?;
  }
  Some(path)
}

/// Desktop notification through `notify-send` (techspec decision). Clicking it opens the window (PRD RF17).
/// Without notify-send the notice only goes to the log.
pub fn notify(app: &AppHandle, summary: &str, body: &str) {
  let mut cmd = Command::new("notify-send");
  cmd.args(["--app-name=Headroom", "--action=default=Abrir"]);
  if let Some(icon) = icon_path() {
    cmd.arg(format!("--icon={}", icon.display()));
  }
  cmd.arg("--").arg(summary).arg(body).stdout(Stdio::piped());
  match cmd.spawn() {
    Err(e) => log::warn!("notify-send indisponível, aviso não exibido ({summary}): {e}"),
    Ok(child) => {
      log::info!("aviso: {summary} · {body}");
      let app = app.clone();
      // --action waits for the user, so this thread parks until the notification closes.
      std::thread::spawn(move || {
        let clicked = child.wait_with_output().is_ok_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "default");
        if clicked {
          log::info!("aviso clicado");
          crate::show_window(&app);
        }
      });
    }
  }
}
