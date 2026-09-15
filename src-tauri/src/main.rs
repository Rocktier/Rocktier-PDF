// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod pdf;
mod security;
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
            commands::search_document,
            commands::page_text,
            commands::stamp_document,
            commands::export_page_images,
            commands::images_to_pdf,
            commands::add_markup,
            commands::add_note,
            commands::add_signature,
            commands::remove_password,
            commands::set_password,
            commands::list_form_fields,
            commands::set_form_values,
            commands::reveal_in_finder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Rocktier PDF Editor");
}
