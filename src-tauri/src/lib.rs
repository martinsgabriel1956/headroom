mod delete;
mod measure;
mod rules;
mod scan;

/// Fresh, empty directory for a test, under the system temp dir.
#[cfg(test)]
pub(crate) fn test_dir(name: &str) -> std::path::PathBuf {
  let d = std::env::temp_dir().join(format!("headroom-test-{name}-{}", std::process::id()));
  let _ = std::fs::remove_dir_all(&d);
  std::fs::create_dir_all(&d).unwrap();
  d
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}
