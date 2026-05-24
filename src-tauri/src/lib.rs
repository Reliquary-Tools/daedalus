mod commands;
mod launch_guard;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if !launch_guard::launch_allowed("daedalus", "Daedalus") {
        eprintln!("Daedalus must be launched from Obelisk with an active RELIQUARY license.");
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::clear_download_archive,
            commands::get_theme_mode,
            commands::get_system_status,
            commands::install_tool,
            commands::open_obelisk,
            commands::probe_source,
            commands::start_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running Daedalus");
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
