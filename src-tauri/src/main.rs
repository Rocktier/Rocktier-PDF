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

use std::sync::Mutex;

use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::Manager as _;
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
    check_updates: &'static str,
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
                license: "许可与激活…", check_updates: "检查更新…",
            },
            "ja" => Self {
                open: "開く…", save: "保存", save_as: "名前を付けて保存…",
                export_markdown: "Markdown で書き出す…", undo: "元に戻す", redo: "やり直す",
                file: "ファイル", edit: "編集", view: "表示", window: "ウインドウ",
                help: "ヘルプ", find: "検索", actual_size: "原寸",
                toggle_theme: "テーマを切り替え", website: "公式サイト",
                feedback: "フィードバック", about: "Rocktier PDF Editor について",
                license: "ライセンス…", check_updates: "アップデートを確認…",
            },
            "ko" => Self {
                open: "열기…", save: "저장", save_as: "다른 이름으로 저장…",
                export_markdown: "Markdown으로 내보내기…", undo: "실행 취소", redo: "다시 실행",
                file: "파일", edit: "편집", view: "보기", window: "창",
                help: "도움말", find: "검색", actual_size: "실제 크기",
                toggle_theme: "테마 전환", website: "공식 웹사이트",
                feedback: "피드백", about: "Rocktier PDF Editor 정보",
                license: "라이선스…", check_updates: "업데이트 확인…",
            },
            "de" => Self {
                open: "Öffnen…", save: "Speichern", save_as: "Speichern unter…",
                export_markdown: "Als Markdown exportieren…", undo: "Rückgängig", redo: "Wiederholen",
                file: "Datei", edit: "Bearbeiten", view: "Ansicht", window: "Fenster",
                help: "Hilfe", find: "Suchen", actual_size: "Originalgröße",
                toggle_theme: "Design wechseln", website: "Website",
                feedback: "Feedback", about: "Über Rocktier PDF Editor",
                license: "Lizenz…", check_updates: "Nach Updates suchen…",
            },
            "es" => Self {
                open: "Abrir…", save: "Guardar", save_as: "Guardar como…",
                export_markdown: "Exportar como Markdown…", undo: "Deshacer", redo: "Rehacer",
                file: "Archivo", edit: "Editar", view: "Ver", window: "Ventana",
                help: "Ayuda", find: "Buscar", actual_size: "Tamaño real",
                toggle_theme: "Cambiar tema", website: "Sitio web",
                feedback: "Comentarios", about: "Acerca de Rocktier PDF Editor",
                license: "Licencia…", check_updates: "Buscar actualizaciones…",
            },
            "pt" => Self {
                open: "Abrir…", save: "Salvar", save_as: "Salvar como…",
                export_markdown: "Exportar como Markdown…", undo: "Desfazer", redo: "Refazer",
                file: "Arquivo", edit: "Editar", view: "Exibir", window: "Janela",
                help: "Ajuda", find: "Buscar", actual_size: "Tamanho real",
                toggle_theme: "Alternar tema", website: "Site",
                feedback: "Comentários", about: "Sobre o Rocktier PDF Editor",
                license: "Licença…", check_updates: "Verificar atualizações…",
            },
            "ar" => Self {
                open: "فتح…", save: "احفظ", save_as: "حفظ باسم…",
                export_markdown: "تصدير كـ Markdown…", undo: "تراجع", redo: "إعادة",
                file: "ملف", edit: "تحرير", view: "عرض", window: "نافذة",
                help: "مساعدة", find: "ابحث", actual_size: "الحجم الحقيقي",
                toggle_theme: "تبديل المظهر", website: "الموقع",
                feedback: "ملاحظات", about: "حول Rocktier PDF Editor",
                license: "الترخيص…", check_updates: "التحقق من التحديثات…",
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
                license: "License…", check_updates: "Check for Updates…",
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
            // Windows 上 muda 的 hide_others 落进 `_ => {}` 点了无反应、hide 是
            // SW_HIDE 窗口消失召不回（findings D-42）——仅 macOS 显示这两项。
            #[cfg(target_os = "macos")]
            &PredefinedMenuItem::hide(app, None)?,
            #[cfg(target_os = "macos")]
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
            // macOS 原生全屏有效；Windows 上 muda 的 fullscreen 落进 `_ => {}` 点了
            // 无反应，改用自定义 win-fullscreen 项，由 on_menu_event 调 set_fullscreen。
            #[cfg(target_os = "macos")]
            &PredefinedMenuItem::fullscreen(app, None)?,
            #[cfg(target_os = "windows")]
            &MenuItem::with_id(app, "win-fullscreen", "Toggle Full Screen", true, None::<&str>)?,
        ],
    )?;

    let site_i = MenuItem::with_id(app, "website", m.website, true, None::<&str>)?;
    let mail_i = MenuItem::with_id(app, "feedback", m.feedback, true, None::<&str>)?;
    // 购买页面上写着"打开应用 → License → 输入激活码"，所以应用里必须真有一个能到那儿的入口。
    // 检查更新：只 emit "check-updates"，联网由前端 services/updates.ts 做。
    let update_i = MenuItem::with_id(app, "check-updates", m.check_updates, true, None::<&str>)?;
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
            &update_i,
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

/// 启动时要打开的 PDF 路径，由操作系统传入：
/// - Windows / Linux：文件关联注册的打开命令是 `"app.exe" "%1"`，路径在 argv 里；
/// - macOS / iOS：走 `RunEvent::Opened` 事件（见 `main()`）。冷启动时该事件可能早于
///   前端挂载，所以先缓冲在这里，前端挂载后再用 `initial_file` 拉取。
struct InitialFile(Mutex<Option<String>>);

/// 启动期到达的文档队列（tao#1235：冷启动时 `application:openURLs:` 早于
/// setup/托管状态，`try_state` 拿不到任何东西，必须在进程级静态里排队，
/// setup 完成后再搬进 `InitialFile`。Chromium 的 `_startupComplete` 同款）。
static PENDING_DOCS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// 与 `tauri.conf.json` 的 `bundle.fileAssociations` 保持一致：只关联 pdf。
fn is_pdf_path(path: &std::path::Path) -> bool {
    if !path.is_file() {
        return false;
    }
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => ext.eq_ignore_ascii_case("pdf"),
        None => false,
    }
}

/// 取命令行里第一个真实存在的 PDF。更新器开关等其它参数会被 is_pdf_path 自然过滤掉。
fn file_from_args() -> Option<String> {
    std::env::args_os()
        .skip(1)
        .map(std::path::PathBuf::from)
        .find(|p| is_pdf_path(p))
        .map(|p| p.to_string_lossy().into_owned())
}

/// 前端挂载后询问「启动时是否带了文档」。只 clone 不 take：
/// effect 重跑（或未来引入 StrictMode）时 take 会让第二次调用拿到 None，
/// 启动文件就被丢掉了。
#[tauri::command]
fn initial_file(state: tauri::State<InitialFile>) -> Option<String> {
    state.0.lock().ok().and_then(|slot| slot.clone())
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .setup(|app| {
            // 授权状态的落盘目录。取不到就留空，`current_status()` 会按"不拦截"处理
            // —— 宁可少拦一次，也不能因为一个目录取不到把用户锁在外面。
            if let Ok(dir) = app.path().app_data_dir() {
                commands::init_license_dir(dir);
            }
            commands::init_app_handle(app.handle().clone());
            // 双击关联文件启动时（Windows/Linux）路径在 argv 里，先缓冲起来。
            app.manage(InitialFile(Mutex::new(file_from_args())));
            // tao#1235：把冷启动队列里的文档搬进托管状态（setup 晚于
            // application:openURLs:，此刻托管状态与窗口才真正可用）。
            let queued: Vec<String> = PENDING_DOCS
                .lock()
                .expect("PENDING_DOCS poisoned")
                .drain(..)
                .collect();
            if let Some(first) = queued.first() {
                if let Some(state) = app.try_state::<InitialFile>() {
                    if let Ok(mut slot) = state.0.lock() {
                        *slot = Some(first.clone());
                    }
                }
            }
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
            commands::report_machine_fingerprint,
            commands::store_receipt,
            commands::clear_receipt,
            build_menu,
            initial_file,
            open_url,
        
            commands::compress_document,])
        .on_menu_event(|app, event| {
            // Windows 真全屏（替代 muda 在 Win 上无效的 PredefinedMenuItem::fullscreen）
            if event.id().0.as_str() == "win-fullscreen" {
                if let Some(win) = app.webview_windows().values().next().cloned() {
                    let _ = win.set_fullscreen(!win.is_fullscreen().unwrap_or(false));
                }
                return;
            }
            // Menu clicks become a frontend action chain, so unsaved-changes
            // guards and toasts stay in one place.
            let _ = app.emit("menu-action", event.id().0.as_str());
        })
        // 冷启动竞态补发：Opened 事件可能落在「前端查询 initial_file 之后、
        // 监听器挂载之前」的空窗里。页面加载完成时若仍有待开文档，再补发一次 ——
        // 前端对同一文档有去重，不会弹两次。
        .on_page_load(|window, payload| {
            if !matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                return;
            }
            if let Some(state) = window.app_handle().try_state::<InitialFile>() {
                if let Ok(slot) = state.0.lock() {
                    if let Some(path) = slot.as_ref() {
                        let _ = window.emit("open-file", path.clone());
                    }
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building Rocktier PDF Editor");

    app.run(|_app_handle, _event| {
        // macOS / iOS 的文件关联不是命令行参数，而是一个事件（Windows 见 file_from_args）。
        // 两种时序都要兜住：冷启动时事件可能早于前端挂载 —— 所以写进 InitialFile，
        // 前端挂载后经 initial_file 拉取；也可能晚于挂载 —— 所以同时 emit 出去。
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        if let tauri::RunEvent::Opened { urls } = _event {
            for url in urls {
                let Ok(path) = url.to_file_path() else { continue };
                let Some(path) = path.to_str() else { continue };
                if !is_pdf_path(std::path::Path::new(path)) {
                    continue;
                }
                // tao#1235：冷启动时该事件在 setup/托管状态存在之前直达（urls 空、
                // try_state 为 None 都可能发生），所以先入进程级队列，setup 再搬运。
                // 热启动先排空队列：队列只在 setup 里 drain 一次，不排则每打开
                // 一个文件就多残留一条（缓慢内存泄漏）。
                if let Ok(mut q) = PENDING_DOCS.lock() {
                    q.clear();
                    q.push(path.to_string());
                }
                // 热启动（应用已运行）：托管状态与前端监听都在，立即送达。
                if let Some(state) = _app_handle.try_state::<InitialFile>() {
                    if let Ok(mut slot) = state.0.lock() {
                        *slot = Some(path.to_string());
                    }
                }
                let _ = _app_handle.emit("open-file", path);
                break;
            }
        }
    });
}
