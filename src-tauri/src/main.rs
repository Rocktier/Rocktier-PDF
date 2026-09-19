// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod compress;
mod formclear;
mod imagepass;
mod pdf;
mod security;
mod state;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter};

use state::AppState;

/// Builds the family-standard menu (App / File / Edit / View / Window / Help).
///
/// Called by the frontend after mount and again whenever the UI language
/// changes. Custom items are forwarded to the frontend as a `menu-action`
/// event; predefined items are localised by the OS and keep their own
/// accelerators. Undo/redo are custom because the document — not the webview —
/// owns the edit history.
fn build_app_menu(app: &AppHandle, lang: &str) -> tauri::Result<()> {
    let zh = lang.starts_with("zh");
    let l = |zhv: &'static str, en: &'static str| if zh { zhv } else { en };

    let open_i = MenuItem::with_id(app, "open", l("打开…", "Open…"), true, Some("CmdOrCtrl+O"))?;
    let save_i = MenuItem::with_id(app, "save", l("保存", "Save"), true, Some("CmdOrCtrl+S"))?;
    let save_as_i = MenuItem::with_id(
        app,
        "save-as",
        l("另存为…", "Save As…"),
        true,
        Some("CmdOrCtrl+Shift+S"),
    )?;

    let app_menu = Submenu::with_items(
        app,
        "Rocktier PDF Editor",
        true,
        &[
            &PredefinedMenuItem::about(app, Some(l("关于 Rocktier PDF Editor", "About Rocktier PDF Editor")), None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;

    let file_menu = Submenu::with_items(
        app,
        l("文件", "File"),
        true,
        &[
            &open_i,
            &PredefinedMenuItem::separator(app)?,
            &save_i,
            &save_as_i,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;

    let undo_i = MenuItem::with_id(app, "undo", l("撤销", "Undo"), true, Some("CmdOrCtrl+Z"))?;
    let redo_i =
        MenuItem::with_id(app, "redo", l("重做", "Redo"), true, Some("CmdOrCtrl+Shift+Z"))?;
    let edit_menu = Submenu::with_items(
        app,
        l("编辑", "Edit"),
        true,
        &[
            &undo_i,
            &redo_i,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;

    let find_i = MenuItem::with_id(app, "find", l("查找", "Find"), true, Some("CmdOrCtrl+F"))?;
    let actual_i =
        MenuItem::with_id(app, "actual-size", l("实际大小", "Actual Size"), true, None::<&str>)?;
    let theme_i = MenuItem::with_id(app, "toggle-theme", l("切换日夜模式", "Toggle Theme"), true, None::<&str>)?;
    let view_menu = Submenu::with_items(
        app,
        l("显示", "View"),
        true,
        &[
            &find_i,
            &PredefinedMenuItem::separator(app)?,
            &actual_i,
            &theme_i,
        ],
    )?;

    let window_menu = Submenu::with_items(
        app,
        l("窗口", "Window"),
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::fullscreen(app, None)?,
        ],
    )?;

    let site_i = MenuItem::with_id(app, "website", l("官方网站", "Website"), true, None::<&str>)?;
    let mail_i = MenuItem::with_id(app, "feedback", l("反馈", "Feedback"), true, None::<&str>)?;
    let help_menu = Submenu::with_items(app, l("帮助", "Help"), true, &[&site_i, &mail_i])?;

    let menu = Menu::with_items(
        app,
        &[
            &app_menu,
            &file_menu,
            &edit_menu,
            &view_menu,
            &window_menu,
            &help_menu,
        ],
    )?;
    app.set_menu(menu)?;
    Ok(())
}

/// Rebuilt by the frontend on mount and on every language change.
#[tauri::command]
fn build_menu(app: AppHandle, lang: String) -> Result<(), String> {
    build_app_menu(&app, &lang).map_err(|e| e.to_string())
}

/// Opens an external link from the Help menu, restricted to a whitelist.
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    const ALLOWED: [&str; 3] = [
        "https://rocktier.com/",
        "https://www.rocktier.com/",
        "mailto:",
    ];
    if !ALLOWED.iter().any(|prefix| url.starts_with(prefix)) {
        return Err(format!("blocked url: {url}"));
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &url])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

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
            commands::undo,
            commands::redo,
            commands::reveal_in_finder,
            build_menu,
            open_url,
        
            commands::compress_document,])
        .on_menu_event(|app, event| {
            // Menu clicks become a frontend action chain, so unsaved-changes
            // guards and toasts stay in one place.
            let _ = app.emit("menu-action", event.id().0.as_str());
        })
        .run(tauri::generate_context!())
        .expect("error while running Rocktier PDF Editor");
}
