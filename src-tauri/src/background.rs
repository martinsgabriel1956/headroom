use std::{
  ffi::CString,
  io,
  os::unix::ffi::OsStrExt,
  path::Path,
  time::{Duration, Instant},
};

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
