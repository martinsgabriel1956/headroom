use crate::{fmt_bytes, notify::notify};
use std::{
  ffi::CString,
  io,
  os::unix::ffi::OsStrExt,
  path::Path,
  time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};

pub const TICK: Duration = Duration::from_secs(60);
pub const DISK_CHECK: Duration = Duration::from_secs(15 * 60);
pub const SCAN_EVERY: Duration = Duration::from_secs(6 * 3600);
pub const RECOVERABLE_MIN: u64 = 5_000_000_000;
const FULL_BELOW: u64 = 20_000_000_000;

/// PRD RF9: less than 10% free or less than 20 GB, whichever comes first.
pub fn disk_full(free: u64, total: u64) -> bool {
  free.saturating_mul(10) < total || free < FULL_BELOW
}

/// PRD RF16: true only on the false → true transition.
#[derive(Default)]
pub struct Edge(bool);

impl Edge {
  pub fn rose(&mut self, now: bool) -> bool {
    let rose = now && !self.0;
    self.0 = now;
    rose
  }
}

/// PRD RF11, RF12: every `every`, and only while the window is hidden.
pub fn scan_due(last: Option<Instant>, now: Instant, every: Duration, window_visible: bool) -> bool {
  !window_visible && last.is_none_or(|t| now.duration_since(t) >= every)
}

/// (free, total) bytes of the filesystem holding `path`, as an unprivileged user sees it.
pub fn free_space(path: &Path) -> io::Result<(u64, u64)> {
  let c = CString::new(path.as_os_str().as_bytes())?;
  let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
  if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 {
    return Err(io::Error::last_os_error());
  }
  let unit = s.f_frsize as u64;
  Ok((s.f_bavail as u64 * unit, s.f_blocks as u64 * unit))
}

/// The monitor thread (techspec "Uma thread de monitor com tick de 60 s"). Only reads: it never deletes (PRD RF15).
/// In debug builds, HEADROOM_FAST=1 shortens every interval and forces both conditions, for manual checks.
pub fn spawn(app: AppHandle) {
  let fast = cfg!(debug_assertions) && std::env::var("HEADROOM_FAST").is_ok_and(|v| v == "1");
  let (tick, disk_every, scan_every) = match fast {
    true => (Duration::from_secs(1), Duration::from_secs(10), Duration::from_secs(30)),
    false => (TICK, DISK_CHECK, SCAN_EVERY),
  };
  std::thread::spawn(move || {
    // Two threads at nice 10: the automatic scan never competes with the user for CPU (user's choice).
    let pool = rayon::ThreadPoolBuilder::new()
      .num_threads(2)
      .start_handler(|_| unsafe {
        libc::setpriority(libc::PRIO_PROCESS, 0, 10);
      })
      .build()
      .expect("pool de segundo plano");
    let (mut disk, mut recoverable) = (Edge::default(), Edge::default());
    // The scan the window runs at startup counts as the first one.
    let (mut last_disk, mut last_scan): (Option<Instant>, Option<Instant>) = (None, Some(Instant::now()));
    loop {
      let now = Instant::now();
      let root = crate::current_root(&app);
      if last_disk.is_none_or(|t| now.duration_since(t) >= disk_every) {
        last_disk = Some(now);
        match free_space(&root) {
          Ok((free, total)) => {
            let full = fast || disk_full(free, total);
            if disk.rose(full) {
              log::info!("disco cheio: {} livres de {}", fmt_bytes(free), fmt_bytes(total));
              let found = crate::recoverable(&app.state::<crate::AppState>())
                .map(|(bytes, _)| format!(" O Headroom acha {} para limpar.", fmt_bytes(bytes)))
                .unwrap_or_default();
              notify(&app, "Disco quase cheio", &format!("{} livres.{found}", fmt_bytes(free)));
            }
          }
          Err(e) => log::warn!("não deu para ler o espaço livre de {}: {e}", root.display()),
        }
      }
      if scan_due(last_scan, now, scan_every, crate::window_visible(&app)) {
        last_scan = Some(now);
        if let Some((bytes, projects)) = pool.install(|| crate::background_scan(&app, root)) {
          let _ = app.emit("scan-replaced", ());
          if recoverable.rose(fast || bytes >= RECOVERABLE_MIN) {
            notify(
              &app,
              &format!("{} para limpar", fmt_bytes(bytes)),
              &format!("em {projects} {}. Abra o Headroom para escolher o que apagar.", if projects == 1 { "projeto" } else { "projetos" }),
            );
          }
        }
      }
      std::thread::sleep(tick);
    }
  });
}

#[cfg(test)]
mod tests {
  use super::*;
  const GB: u64 = 1_000_000_000;

  #[test]
  fn disk_full_uses_whichever_limit_comes_first() {
    assert!(disk_full(47 * GB, 476 * GB));
    assert!(!disk_full(48 * GB, 476 * GB));
    assert!(disk_full(19 * GB, 100 * GB), "20 GB rule on a small disk");
    assert!(!disk_full(20 * GB, 100 * GB));
  }

  #[test]
  fn edge_fires_only_when_the_condition_starts() {
    let mut e = Edge::default();
    let fired: Vec<bool> = [false, true, true, false, true].iter().map(|&v| e.rose(v)).collect();
    assert_eq!(fired, [false, true, false, false, true]);
  }

  #[test]
  fn scan_due_waits_for_hidden_window_and_interval() {
    let t0 = Instant::now();
    let every = Duration::from_secs(100);
    assert!(scan_due(None, t0, every, false));
    assert!(!scan_due(None, t0, every, true), "never with the window visible");
    assert!(!scan_due(Some(t0), t0 + Duration::from_secs(99), every, false));
    assert!(scan_due(Some(t0), t0 + every, every, false));
  }

  #[test]
  fn free_space_reads_the_filesystem() {
    let (free, total) = free_space(&crate::test_dir("statvfs")).unwrap();
    assert!(free > 0 && free <= total);
  }
}
