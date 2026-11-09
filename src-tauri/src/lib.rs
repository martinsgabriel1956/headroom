mod delete;
mod measure;
mod rules;
mod scan;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}
