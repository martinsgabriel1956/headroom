mod background;
mod delete;
mod measure;
mod notify;
mod rules;
mod scan;

use measure::real_of;
use scan::{Counters, Found};
use serde::Serialize;
use std::{
  collections::{HashMap, HashSet},
  fs,
  os::unix::fs::MetadataExt,
  path::{Path, PathBuf},
  sync::{
    atomic::{AtomicBool, AtomicU64, Ordering::{Relaxed, SeqCst}},
    Mutex,
  },
  thread,
  time::{Duration, Instant, SystemTime},
};
use tauri::{ipc::Channel, AppHandle, Manager, State};

const ACTIVE_DAYS: i64 = 7;

/// The last scan lives here; the UI only ever sees ids (PRD RF20, techspec "Estado no backend").
struct Scan {
  generation: u64,
  root: PathBuf,
  dev: u64,
  items: HashMap<u32, Found>,
  /// Summary of the finished scan; None while it runs or if it was cancelled.
  done: Option<DoneDto>,
}

#[derive(Default)]
struct AppState {
  generation: AtomicU64,
  last: Mutex<Option<Scan>>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct ArtifactDto {
  id: u32,
  project: String,
  kind: &'static str,
  rel: String,
  rule: String,
  real: u64,
  apparent: u64,
  days: i64,
}

#[derive(Serialize, Clone, Debug)]
struct DoneDto {
  millis: u64,
  skipped: u64,
  count: usize,
  active: Vec<ActiveDto>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SnapshotDto {
  generation: u64,
  root: String,
  home: String,
  artifacts: Vec<ArtifactDto>,
  done: Option<DoneDto>,
}

#[derive(Serialize, Clone, Debug)]
struct ActiveDto {
  project: String,
  days: i64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
enum ScanEvent {
  Started { generation: u64, root: String, home: String },
  Artifact { artifact: ArtifactDto },
  Progress { dirs: u64 },
  Done(DoneDto),
}

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
enum DeleteEvent {
  Deleting { id: u32 },
  Deleted { id: u32 },
  Refused { id: u32, reason: String },
}

#[derive(Serialize, Debug)]
struct Refusal {
  id: u32,
  reason: String,
}

#[derive(Serialize, Debug)]
struct DeleteReport {
  freed: u64,
  /// Ids no longer on disk as scanned (deleted, partly deleted, or already gone): the UI drops them.
  removed: Vec<u32>,
  refused: Vec<Refusal>,
}

fn unix_now() -> i64 {
  SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

fn days_since(now: i64, t: i64) -> i64 {
  (now - t).max(0) / 86_400
}

fn dto(f: &Found, now: i64) -> ArtifactDto {
  ArtifactDto {
    id: f.id,
    project: f.project.display().to_string(),
    kind: f.kind,
    rel: f.path.strip_prefix(&f.project).unwrap_or(&f.path).display().to_string(),
    rule: f.rule.clone(),
    real: f.measured.real(),
    apparent: f.measured.apparent,
    days: days_since(now, f.measured.newest),
  }
}

fn current(g: &Option<Scan>, generation: u64) -> Result<&Scan, String> {
  g.as_ref().filter(|s| s.generation == generation).ok_or_else(|| "A varredura mudou. Escaneie de novo.".into())
}

/// Everything the `scan` command does, without Tauri, emitting through `emit`.
fn run_scan(state: &AppState, root: PathBuf, home: String, emit: &(dyn Fn(ScanEvent) + Sync)) -> Result<(), String> {
  let root = fs::canonicalize(&root).map_err(|e| format!("Não deu para abrir {}: {e}", root.display()))?;
  let dev = fs::metadata(&root).map_err(|e| e.to_string())?.dev();
  let generation = state.generation.fetch_add(1, SeqCst) + 1;
  *state.last.lock().unwrap() = Some(Scan { generation, root: root.clone(), dev, items: HashMap::new(), done: None });
  emit(ScanEvent::Started { generation, root: root.display().to_string(), home });

  let t0 = Instant::now();
  let now = unix_now();
  let counters = Counters::default();
  let finished = AtomicBool::new(false);
  let cancel = || state.generation.load(SeqCst) != generation;
  let recent = thread::scope(|s| {
    s.spawn(|| {
      while !finished.load(SeqCst) {
        thread::sleep(Duration::from_millis(100));
        emit(ScanEvent::Progress { dirs: counters.dirs.load(Relaxed) });
      }
    });
    let recent = scan::scan(&root, now - ACTIVE_DAYS * 86_400, &counters, &cancel, &|f| {
      let artifact = dto(&f, now);
      let mut g = state.last.lock().unwrap();
      if let Some(sc) = g.as_mut().filter(|sc| sc.generation == generation) {
        sc.items.insert(f.id, f);
        drop(g);
        emit(ScanEvent::Artifact { artifact });
      }
    });
    finished.store(true, SeqCst);
    recent
  });
  if cancel() {
    return Ok(());
  }

  let mut g = state.last.lock().unwrap();
  let sc = g.as_mut().filter(|s| s.generation == generation).ok_or("A varredura mudou. Escaneie de novo.")?;
  let projects: HashSet<&Path> = sc.items.values().map(|f| f.project.as_path()).collect();
  let active = scan::active_projects(&recent, projects)
    .into_iter()
    .map(|(p, t)| ActiveDto { project: p.display().to_string(), days: days_since(now, t) })
    .collect();
  let (millis, skipped, count) = (t0.elapsed().as_millis() as u64, counters.skipped.load(Relaxed), sc.items.len());
  log::info!("varredura de {}: {count} artefatos em {millis} ms, {} pastas, {skipped} puladas", root.display(), counters.dirs.load(Relaxed));
  let done = DoneDto { millis, skipped, count, active };
  sc.done = Some(done.clone());
  drop(g);
  emit(ScanEvent::Done(done));
  Ok(())
}

/// Revalidates and deletes each id of the current scan. Holding the lock throughout also
/// keeps a second deletion from running at the same time (PRD RF24).
fn run_delete(state: &AppState, generation: u64, ids: Vec<u32>, emit: &dyn Fn(DeleteEvent)) -> Result<DeleteReport, String> {
  let mut g = state.last.lock().unwrap();
  let sc = g.as_mut().filter(|s| s.generation == generation).ok_or("A varredura mudou. Escaneie de novo.")?;
  let (mut deleted, mut removed, mut refused) = (vec![], vec![], vec![]);
  for id in ids {
    let Some(f) = sc.items.get(&id) else {
      refused.push(Refusal { id, reason: "não está na varredura atual".into() });
      continue;
    };
    emit(DeleteEvent::Deleting { id });
    match delete::revalidate(&sc.root, sc.dev, f).and_then(|_| delete::remove(f)) {
      Ok(()) => {
        log::info!("apagado {} ({} bytes reais)", f.path.display(), f.measured.real());
        deleted.push(id);
        emit(DeleteEvent::Deleted { id });
      }
      Err(reason) => {
        log::warn!("recusado {}: {reason}", f.path.display());
        if !f.path.exists() || reason.starts_with("apagado em parte") {
          removed.push(id);
        }
        emit(DeleteEvent::Refused { id, reason: reason.clone() });
        refused.push(Refusal { id, reason });
      }
    }
  }
  let freed = real_of(deleted.iter().map(|i| &sc.items[i].measured));
  removed.extend(&deleted);
  for i in &removed {
    sc.items.remove(i);
  }
  Ok(DeleteReport { freed, removed, refused })
}

/// Brings the main window back from the tray, minimized or behind other windows.
pub(crate) fn show_window(app: &AppHandle) {
  if let Some(w) = app.get_webview_window("main") {
    let _ = w.show();
    let _ = w.unminimize();
    let _ = w.set_focus();
  }
}

#[tauri::command]
async fn scan(app: AppHandle, root: Option<String>, on_event: Channel<ScanEvent>) -> Result<(), String> {
  let home = std::env::var("HOME").map_err(|_| "Sem $HOME definido.")?;
  let root = root.map_or_else(|| PathBuf::from(&home), PathBuf::from);
  tauri::async_runtime::spawn_blocking(move || {
    run_scan(&app.state::<AppState>(), root, home, &|e| {
      let _ = on_event.send(e);
    })
  })
  .await
  .map_err(|e| e.to_string())?
}

fn home() -> String {
  std::env::var("HOME").unwrap_or_default()
}

/// The whole current scan at once, for a window that was hidden while it ran (PRD RF14).
#[tauri::command]
fn snapshot(state: State<AppState>) -> Option<SnapshotDto> {
  snapshot_of(&state)
}

fn snapshot_of(state: &AppState) -> Option<SnapshotDto> {
  let g = state.last.lock().unwrap();
  let sc = g.as_ref()?;
  let now = unix_now();
  Some(SnapshotDto {
    generation: sc.generation,
    root: sc.root.display().to_string(),
    home: home(),
    artifacts: sc.items.values().map(|f| dto(f, now)).collect(),
    done: sc.done.clone(),
  })
}

#[tauri::command]
fn measure(state: State<AppState>, generation: u64, ids: Vec<u32>) -> Result<u64, String> {
  let g = state.last.lock().unwrap();
  let sc = current(&g, generation)?;
  Ok(real_of(ids.iter().filter_map(|i| sc.items.get(i)).map(|f| &f.measured)))
}

#[tauri::command]
async fn delete(app: AppHandle, generation: u64, ids: Vec<u32>, on_event: Channel<DeleteEvent>) -> Result<DeleteReport, String> {
  tauri::async_runtime::spawn_blocking(move || {
    run_delete(&app.state::<AppState>(), generation, ids, &|e| {
      let _ = on_event.send(e);
    })
  })
  .await
  .map_err(|e| e.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())
    .manage(AppState::default())
    .invoke_handler(tauri::generate_handler![scan, measure, delete, snapshot])
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}

/// Fresh, empty directory for a test, under the system temp dir.
#[cfg(test)]
pub(crate) fn test_dir(name: &str) -> std::path::PathBuf {
  let d = std::env::temp_dir().join(format!("headroom-test-{name}-{}", std::process::id()));
  let _ = std::fs::remove_dir_all(&d);
  std::fs::create_dir_all(&d).unwrap();
  d
}

#[cfg(test)]
mod tests {
  use super::*;

  fn tree(t: &Path) {
    for (dir, manifest) in [("a", "package.json"), ("b", "package.json")] {
      fs::create_dir_all(t.join(dir).join("node_modules/pkg")).unwrap();
      fs::write(t.join(dir).join(manifest), "{}").unwrap();
      fs::write(t.join(dir).join("node_modules/pkg/index.js"), vec![1u8; 20_000]).unwrap();
    }
  }

  fn collect(state: &AppState, t: &Path) -> Vec<ScanEvent> {
    let events = Mutex::new(vec![]);
    run_scan(state, t.to_path_buf(), "/home".into(), &|e| events.lock().unwrap().push(e)).unwrap();
    events.into_inner().unwrap().into_iter().filter(|e| !matches!(e, ScanEvent::Progress { .. })).collect()
  }

  /// PRD goal "o número confere": deletes every artifact under `$HEADROOM_VERIFY` and compares the
  /// reported freed bytes with the change in free space from `df`. Destructive by design; run by hand:
  /// `HEADROOM_VERIFY=~/headroom-verify cargo test -- --ignored --nocapture freed_matches_df`.
  #[test]
  #[ignore]
  fn freed_matches_df() {
    let root = PathBuf::from(std::env::var_os("HEADROOM_VERIFY").expect("set HEADROOM_VERIFY"));
    let avail = || {
      let out = std::process::Command::new("df").args(["-B1", "--output=avail"]).arg(&root).output().unwrap();
      String::from_utf8(out.stdout).unwrap().lines().last().unwrap().trim().parse::<i64>().unwrap()
    };
    let state = AppState::default();
    collect(&state, &root);
    let ids: Vec<u32> = state.last.lock().unwrap().as_ref().unwrap().items.keys().copied().collect();
    let before = avail();
    let report = run_delete(&state, 1, ids, &|_| {}).unwrap();
    std::thread::sleep(Duration::from_secs(2)); // let the filesystem settle its counters
    let delta = avail() - before;
    let error = (delta - report.freed as i64).abs() as f64 / report.freed.max(1) as f64;
    println!("informado {} bytes, df mudou {} bytes, diferença {:.2}%", report.freed, delta, error * 100.0);
    assert!(error <= 0.05, "PRD: margem de até 5%");
  }

  #[test]
  fn scan_emits_in_order_and_ids_match_state() {
    let t = test_dir("lib-scan");
    tree(&t);
    let state = AppState::default();
    let events = collect(&state, &t);
    assert!(matches!(events.first(), Some(ScanEvent::Started { generation: 1, .. })));
    assert!(matches!(events.last(), Some(ScanEvent::Done(DoneDto { count: 2, .. }))));
    let ids: HashSet<u32> = events.iter().filter_map(|e| match e { ScanEvent::Artifact { artifact } => Some(artifact.id), _ => None }).collect();
    let g = state.last.lock().unwrap();
    assert_eq!(ids, g.as_ref().unwrap().items.keys().copied().collect());
  }

  #[test]
  fn snapshot_returns_the_finished_scan() {
    let t = test_dir("lib-snapshot");
    tree(&t);
    let state = AppState::default();
    assert!(snapshot_of(&state).is_none(), "nothing before the first scan");
    let events = collect(&state, &t);
    let streamed: HashSet<u32> = events.iter().filter_map(|e| match e { ScanEvent::Artifact { artifact } => Some(artifact.id), _ => None }).collect();
    let snap = snapshot_of(&state).unwrap();
    assert_eq!(snap.artifacts.iter().map(|a| a.id).collect::<HashSet<_>>(), streamed);
    assert_eq!(snap.done.unwrap().count, 2);
  }

  #[test]
  fn new_scan_invalidates_old_generation_and_delete_reports() {
    let t = test_dir("lib-delete");
    tree(&t);
    let state = AppState::default();
    collect(&state, &t);
    collect(&state, &t); // generation 2
    let stale = run_delete(&state, 1, vec![1], &|_| {});
    assert!(stale.is_err(), "old generation refused");

    let ids: Vec<u32> = state.last.lock().unwrap().as_ref().unwrap().items.keys().copied().collect();
    fs::remove_file(t.join("b/package.json")).unwrap(); // b's rule no longer holds
    let report = run_delete(&state, 2, ids.clone(), &|_| {}).unwrap();
    assert_eq!(report.removed.len(), 1);
    assert_eq!(report.refused.len(), 1);
    assert_eq!(report.refused[0].reason, "a regra não vale mais");
    assert!(report.freed >= 20_000);
    assert!(!t.join("a/node_modules").exists());
    assert!(t.join("b/node_modules").exists());
  }
}
