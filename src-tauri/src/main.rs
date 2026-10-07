// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod compress;
mod formclear;
mod imagepass;
/// 家族内唯一的产品标识，用作试用记录的副存储命名空间。
///
/// 必须与 `tauri.conf.json` 的 `bundle.identifier` 逐字一致 ——
/// 副存储按它分文件，改了会导致老用户的试用记录读不到（等于白送 7 天）。
/// 改动时两处必须同步。
pub const APP_KEY: &str = "Rocktier.RocktierPDF";

// 授权：试用状态与回执验签。写命令的拦截在 commands.rs，界面在 LicenseDialog。
mod license;
mod trial;
// PDF → Markdown：结构推断内核（纯逻辑，单测覆盖），取字符在 pdf_to_markdown
mod mdconv;
mod pdf;
// PDF → Markdown：取字符与调度（结构推断在 mdconv）
mod pdf_to_md;
mod security;
mod state;

use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter};

use state::AppState;

/// Builds the family-standard menu (App / File / Edit / View / Window / Help).
///
/// Called by the frontend after mount and again whenever the UI language
/// changes. Custom items are forwarded to the frontend as a `menu-action`
/// event; predefined items are localised by the OS and keep their own
/// accelerators. Undo/redo are custom because the document — not the webview —
/// owns the edit history.
/// Menu labels for one language.
///
/// Why a struct instead of the previous `l(zh, en)` closure:
/// the closure only spoke two languages, and extending it to eight would have
/// meant turning every call site into an 8-argument function — unreadable and
/// easy to get out of order (swapping `ja` and `ko` compiles fine and shows
/// the wrong language). One struct per language keeps each call site to a
/// single field.
///
/// Wording comes from `docs/rocktier/i18n/glossary.json`, the same source the
/// frontend toolbars use — the native menu must not call "Save" something the
/// toolbar calls "Save as". Unknown languages fall back to English rather than
/// panicking, so a bad `localStorage` value degrades to a usable UI.
struct MenuStrings {
    open: &'static str,
    save: &'static str,
    save_as: &'static str,
    export_markdown: &'static str,
    undo: &'static str,
    redo: &'static str,
    file: &'static str,
    edit: &'static str,
    view: &'static str,
    window: &'static str,
    help: &'static str,
    find: &'static str,
    actual_size: &'static str,
    toggle_theme: &'static str,
    website: &'static str,
    feedback: &'static str,
    about: &'static str,
    license: &'static str,
}

impl MenuStrings {
    fn for_lang(lang: &str) -> Self {
        // Match on the primary subtag so "zh-CN" and "zh-Hans" both land on zh.
        let code = lang.split(['-', '_']).next().unwrap_or("");
        match code {
            "zh" => Self {
                open: "打开…", save: "保存", save_as: "另存为…",
                export_markdown: "导出为 Markdown…", undo: "撤销", redo: "重做",
                file: "文件", edit: "编辑", view: "显示", window: "窗口",
                help: "帮助", find: "查找", actual_size: "实际大小",
                toggle_theme: "切换日夜模式", website: "官方网站",
                feedback: "反馈", about: "关于 Rocktier PDF Editor",
                license: "许可与激活…",
            },
            "ja" => Self {
                open: "開く…", save: "保存", save_as: "名前を付けて保存…",
                export_markdown: "Markdown で書き出す…", undo: "元に戻す", redo: "やり直す",
                file: "ファイル", edit: "編集", view: "表示", window: "ウインドウ",
                help: "ヘルプ", find: "検索", actual_size: "原寸",
                toggle_theme: "テーマを切り替え", website: "公式サイト",
                feedback: "フィードバック", about: "Rocktier PDF Editor について",
                license: "ライセンス…",
            },
            "ko" => Self {
                open: "열기…", save: "저장", save_as: "다른 이름으로 저장…",
                export_markdown: "Markdown으로 내보내기…", undo: "실행 취소", redo: "다시 실행",
                file: "파일", edit: "편집", view: "보기", window: "창",
                help: "도움말", find: "검색", actual_size: "실제 크기",
                toggle_theme: "테마 전환", website: "공식 웹사이트",
                feedback: "피드백", about: "Rocktier PDF Editor 정보",
                license: "라이선스…",
            },
            "de" => Self {
                open: "Öffnen…", save: "Speichern", save_as: "Speichern unter…",
                export_markdown: "Als Markdown exportieren…", undo: "Rückgängig", redo: "Wiederholen",
                file: "Datei", edit: "Bearbeiten", view: "Ansicht", window: "Fenster",
                help: "Hilfe", find: "Suchen", actual_size: "Originalgröße",
                toggle_theme: "Design wechseln", website: "Website",
                feedback: "Feedback", about: "Über Rocktier PDF Editor",
                license: "Lizenz…",
            },
            "es" => Self {
                open: "Abrir…", save: "Guardar", save_as: "Guardar como…",
                export_markdown: "Exportar como Markdown…", undo: "Deshacer", redo: "Rehacer",
                file: "Archivo", edit: "Editar", view: "Ver", window: "Ventana",
                help: "Ayuda", find: "Buscar", actual_size: "Tamaño real",
                toggle_theme: "Cambiar tema", website: "Sitio web",
                feedback: "Comentarios", about: "Acerca de Rocktier PDF Editor",
                license: "Licencia…",
            },
            "pt" => Self {
                open: "Abrir…", save: "Salvar", save_as: "Salvar como…",
                export_markdown: "Exportar como Markdown…", undo: "Desfazer", redo: "Refazer",
                file: "Arquivo", edit: "Editar", view: "Exibir", window: "Janela",
                help: "Ajuda", find: "Buscar", actual_size: "Tamanho real",
                toggle_theme: "Alternar tema", website: "Site",
                feedback: "Comentários", about: "Sobre o Rocktier PDF Editor",
                license: "Licença…",
            },
            "ar" => Self {
                open: "فتح…", save: "احفظ", save_as: "حفظ باسم…",
                export_markdown: "تصدير كـ Markdown…", undo: "تراجع", redo: "إعادة",
                file: "ملف", edit: "تحرير", view: "عرض", window: "نافذة",
                help: "مساعدة", find: "ابحث", actual_size: "الحجم الحقيقي",
                toggle_theme: "تبديل المظهر", website: "الموقع",
                feedback: "ملاحظات", about: "حول Rocktier PDF Editor",
                license: "الترخيص…",
            },
            // English is both the default and the fallback: the family convention
            // is a fixed English default (family.json `defaultLanguage: "en"`),
            // and an unrecognised code must still yield a usable menu.
            _ => Self {
                open: "Open…", save: "Save", save_as: "Save As…",
                export_markdown: "Export as Markdown…", undo: "Undo", redo: "Redo",
                file: "File", edit: "Edit", view: "View", window: "Window",
                help: "Help", find: "Find", actual_size: "Actual Size",
                toggle_theme: "Toggle Theme", website: "Website",
                feedback: "Feedback", about: "About Rocktier PDF Editor",
                license: "License…",
            },
        }
    }
}

/// Builds the family-standard menu (App / File / Edit / View / Window / Help).
///
/// Called by the frontend after mount and again whenever the UI language
/// changes. Custom items are forwarded to the frontend as a `menu-action`
/// event; predefined items are localised by the OS and keep their own
/// accelerators. Undo/redo are custom because the document — not the webview —
/// owns the edit history.
fn build_app_menu(app: &AppHandle, lang: &str) -> tauri::Result<()> {
    let m = MenuStrings::for_lang(lang);

    let open_i = MenuItem::with_id(app, "open", m.open, true, Some("CmdOrCtrl+O"))?;
    let save_i = MenuItem::with_id(app, "save", m.save, true, Some("CmdOrCtrl+S"))?;
    let save_as_i = MenuItem::with_id(
        app,
        "save-as",
        m.save_as,
        true,
        Some("CmdOrCtrl+Shift+S"),
    )?;

    let app_menu = Submenu::with_items(
        app,
        "Rocktier PDF Editor",
        true,
        &[
            // 第三个参数不能是 None：Windows 后端只有匹配
            // `PredefinedMenuItemType::About(Some(metadata))` 才会调show_about_dialog，
            // None 落入 `_ => {}` —— 菜单项在，点击**完全无反应**。
            // macOS 走 NSAboutPanel（忽略 metadata），所以这个坑只在 Windows 暴露。
            &PredefinedMenuItem::about(
                app,
                Some(m.about),
                Some(AboutMetadata {
                    version: Some(env!("CARGO_PKG_VERSION").to_string()),
                    copyright: Some("Copyright 2026 Rocktier".to_string()),
                    ..Default::default()
                }),
            )?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;

    let file_menu = Submenu::with_items(
        app,
        m.file,
        true,
        &[
            &open_i,
            &PredefinedMenuItem::separator(app)?,
            &save_i,
            &save_as_i,
            &PredefinedMenuItem::separator(app)?,
            // 路线图 R4：导出为 Markdown。放File 菜单而非工具栏 ——
            // 它是「输出到另一种格式」，与导出 PNG 同类，且低频。
            &MenuItem::with_id(
                app,
                "export-markdown",
                m.export_markdown,
                true,
                Some("CmdOrCtrl+Shift+M"),
            )?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;

    let undo_i = MenuItem::with_id(app, "undo", m.undo, true, Some("CmdOrCtrl+Z"))?;
    let redo_i =
        MenuItem::with_id(app, "redo", m.redo, true, Some("CmdOrCtrl+Shift+Z"))?;
    let edit_menu = Submenu::with_items(
        app,
        m.edit,
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

    let find_i = MenuItem::with_id(app, "find", m.find, true, Some("CmdOrCtrl+F"))?;
    let actual_i =
        MenuItem::with_id(app, "actual-size", m.actual_size, true, None::<&str>)?;
    let theme_i = MenuItem::with_id(app, "toggle-theme", m.toggle_theme, true, None::<&str>)?;
    let view_menu = Submenu::with_items(
        app,
        m.view,
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
        m.window,
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::fullscreen(app, None)?,
        ],
    )?;

    let site_i = MenuItem::with_id(app, "website", m.website, true, None::<&str>)?;
    let mail_i = MenuItem::with_id(app, "feedback", m.feedback, true, None::<&str>)?;
    // 购买页面上写着"打开应用 → License → 输入激活码"，所以应用里必须真有一个能到那儿的入口。
    let license_i = MenuItem::with_id(
        app,
        "license",
        m.license,
        true,
        None::<&str>,
    )?;
    let help_menu = Submenu::with_items(
        app,
        m.help,
        true,
        &[
            &license_i,
            &PredefinedMenuItem::separator(app)?,
            &site_i,
            &mail_i,
        ],
    )?;

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

    // Windows 那条路径要经过 `cmd`，而 URL 是数据、不是命令行：cmd 会把 & | ^ 当作
    // 语法解析，于是一个含 & 的 mailto 就能再起一个进程。两道收口 —— 先拒绝会破坏
    // 引号包裹的字符，再把 URL 放进引号里（引号内 cmd 不解析元字符）。
    // macOS 与 Linux 直接 exec，不经过 shell，没有这个问题。
    if url.contains('"') || url.contains('\n') || url.contains('\r') {
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
            .args(["/C", "start", "", &format!("\"{url}\"")])
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
        .setup(|app| {
            // 授权状态的落盘目录。取不到就留空，`current_status()` 会按"不拦截"处理
            // —— 宁可少拦一次，也不能因为一个目录取不到把用户锁在外面。
            use tauri::Manager as _;
            if let Ok(dir) = app.path().app_data_dir() {
                commands::init_license_dir(dir);
            }
            commands::init_app_handle(app.handle().clone());
            Ok(())
        })
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
            commands::pdf_to_markdown,
            commands::pdf_save_markdown,
            commands::add_markup,
            commands::add_note,
            commands::add_signature,
            commands::redact_regions,
            commands::remove_password,
            commands::set_password,
            commands::list_form_fields,
            commands::set_form_values,
            commands::undo,
            commands::redo,
            commands::reveal_in_finder,
            commands::license_status,
            commands::machine_fingerprint,
            commands::store_receipt,
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
