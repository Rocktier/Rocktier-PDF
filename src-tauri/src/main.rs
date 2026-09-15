// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod pdf;
mod state;

use state::AppState;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::open_document,
            commands::close_document,
            commands::render_page,
            commands::delete_pages,
            commands::rotate_pages,
            commands::move_page,
            commands::save_document,
            commands::extract_pages,
            commands::merge_documents,
            commands::split_document,
            commands::reveal_in_finder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Rocktier PDF Editor");
}
