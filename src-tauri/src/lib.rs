mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_system_status,
            commands::install_tool,
            commands::probe_source,
            commands::start_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running Daedalus");
}
