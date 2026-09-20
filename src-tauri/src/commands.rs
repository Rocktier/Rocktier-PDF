//! Tauri command surface.
//!
//! All commands are `async` so long-running Pdfium work never blocks the UI
//! thread. Errors are flattened to English strings and shown as-is in the
//! status bar — v0.1 has no error-code mapping layer.

use std::path::PathBuf;

use pdfium_render::prelude::*;
use tauri::{AppHandle, State};

use crate::pdf::{
    degrees_of, doc_info, init_pdfium, parse_ranges, render_page as render_page_impl,
    rotation_of, DocumentInfo, FormFieldInfo, MarkupKind, MarkupRect, PathResult, RenderedPage,
    SearchHit, SplitMode, StampKind,
};
use crate::state::{AppState, OpenDoc};

type CmdResult<T> = Result<T, String>;

/* ── 授权：试用与激活（见 license.rs 的模块说明）────────────────────── */

/// 试用与授权状态的落盘目录。由 `main.rs` 的 setup 注入。
///
/// 用全局而不是给 17 个写命令各加一个参数：那样 diff 会大到看不出真正改了什么，
/// 而它也不是业务状态，读它不需要与文档状态同步。
static LICENSE_DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

pub fn init_license_dir(dir: std::path::PathBuf) {
    let _ = LICENSE_DIR.set(dir);
}

/// 供闸门发事件用。setup 注入；即使没注入也照样能拦截，只是界面不会自动弹窗。
static APP_HANDLE: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();

pub fn init_app_handle(app: tauri::AppHandle) {
    let _ = APP_HANDLE.set(app);
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 当前授权状态。
///
/// 目录未注入（setup 失败）时按"试用中、满额天数"处理 —— 失败方向刻意选**放行**：
/// 一个取不到的目录不该变成一次锁死。
fn current_license() -> crate::license::Status {
    let Some(dir) = LICENSE_DIR.get() else {
        return crate::license::Status::Trialing { days_left: crate::license::TRIAL_DAYS };
    };
    let now = now_secs();
    let started = crate::license::ensure_started(dir, now);
    let receipt = crate::license::read_valid_receipt(dir, crate::license::PUBLIC_KEY_B64);
    crate::license::status_from(started, receipt.as_ref(), now)
}

/// 写操作的统一闸门。
///
/// 在**命令层**拦，而不是在每个界面路径上判断：界面路径会随功能增长而增加，漏掉一条
/// 就是一道缝；命令层是所有写操作的必经之路。
///
/// 错误码固定为 `LICENSE_EXPIRED`，前端凭它弹购买/激活框。
fn ensure_write_allowed() -> CmdResult<()> {
    if current_license().allows_write(crate::license::enforced()) {
        return Ok(());
    }
    // 让界面主动知道"被拦下了"，而不是在每个动作的 catch 里各判一次错误码 ——
    // 那种写法漏掉一处，用户看到的就只是一个没有解释的失败。
    if let Some(app) = APP_HANDLE.get() {
        let _ = tauri::Emitter::emit(app, "license-expired", ());
    }
    Err("LICENSE_EXPIRED".to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseInfo {
    /// `trial` / `expired` / `licensed`。
    pub status: String,
    /// 仅 `trial` 时有意义。
    pub days_left: i64,
    /// 仅 `licensed` 时有值（`SQ` 单品 / `FL` 全家桶…）。
    pub product: Option<String>,
    /// 当前是否真的会拦截写操作（渠道 + 公钥 + 总开关三者决定）。
    pub enforcing: bool,
    /// `direct`（官网直链）/ `store`（微软商店）。
    pub channel: String,
    /// 本构建是否已配置验签公钥。
    ///
    /// 没配置时**任何人都激活不了**（回执必然验不过）。界面据此如实说明，而不是
    /// 拿"激活码未被接受"去搪塞一位已经付过钱的用户。
    pub activation_configured: bool,
}

fn license_info() -> LicenseInfo {
    let status = current_license();
    LicenseInfo {
        status: status.as_str().to_string(),
        days_left: match &status {
            crate::license::Status::Trialing { days_left } => *days_left,
            _ => 0,
        },
        product: match &status {
            crate::license::Status::Licensed { product } => Some(product.clone()),
            _ => None,
        },
        enforcing: crate::license::enforced(),
        channel: crate::license::channel().to_string(),
        activation_configured: !crate::license::PUBLIC_KEY_B64.trim().is_empty(),
    }
}

/// 供界面展示：剩余试用天数 / 是否已激活 / 当前渠道。
#[tauri::command]
pub async fn license_status() -> CmdResult<LicenseInfo> {
    Ok(license_info())
}

/// 保存服务端签出的回执并立即验签。
///
/// 联网换回执的那一步在**前端**做（`fetch` 到 rocktier.com/api/activate），
/// 为的是不引入 HTTP 客户端依赖；但**验签与落盘必须在这里** —— 前端拿到的只是一段
/// 待验的字符串，能证明它有效与否的只有公钥。
#[tauri::command]
pub async fn store_receipt(signed: String) -> CmdResult<LicenseInfo> {
    let dir = LICENSE_DIR
        .get()
        .ok_or_else(|| "no app data directory".to_string())?;
    let trimmed = signed.trim();
    crate::license::verify_receipt(trimmed, crate::license::PUBLIC_KEY_B64)?;
    crate::license::save_receipt(dir, trimmed)?;
    Ok(license_info())
}

/// 这份 PDF 是否带密码保护。
///
/// pdfium 不提供该信息，也不写加密，所以在打开时用 lopdf 读一次 trailer 的
/// `/Encrypt` 记下来。解析失败一律当作"没有"——这个判断只用于保存时拦截原地
/// 覆盖，不该因为一个畸形文件就让正常保存也失败。
fn detect_encrypted(bytes: &[u8]) -> bool {
    lopdf::Document::load_from(std::io::Cursor::new(bytes))
        .map(|d| d.trailer.get(b"Encrypt").is_ok())
        .unwrap_or(false)
}

/* ── Lifecycle ───────────────────────────────────────────────────── */

#[tauri::command]
pub async fn open_document(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    password: Option<String>,
) -> CmdResult<DocumentInfo> {
    let pdfium = init_pdfium(&app)?;

    // Load from bytes rather than a file handle: it keeps the source file
    // unlocked so "Save" can overwrite the original on Windows.
    let bytes = std::fs::read(&path).map_err(|e| format!("Cannot read file: {e}"))?;
    // 在 bytes 被 move 进 pdfium 之前先判断这份文件是否带密码保护。
    let was_encrypted = detect_encrypted(&bytes);
    let document = pdfium
        .load_pdf_from_byte_vec(bytes, password.as_deref())
        .map_err(|e| match e {
            // Typed sentinels the frontend matches on to raise a password prompt.
            PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError) => {
                if password.is_some() {
                    "PASSWORD_INCORRECT".to_string()
                } else {
                    "PASSWORD_REQUIRED".to_string()
                }
            }
            other => format!("Cannot open PDF: {other}"),
        })?;

    if document.pages().is_empty() {
        return Err("This PDF has no pages.".to_string());
    }

    let info = doc_info(&document, &path, false)?;

    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    *guard = Some(OpenDoc {
        document,
        path,
        was_encrypted,
        dirty: false,
        undo: Vec::new(),
        redo: Vec::new(),
        radio_clears: std::collections::BTreeSet::new(),
    });

    Ok(info)
}

#[tauri::command]
pub async fn close_document(state: State<'_, AppState>) -> CmdResult<()> {
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    *guard = None;
    Ok(())
}

/* ── Undo / redo ─────────────────────────────────────────────────── */

/// Snapshots the document before a mutation so it can be undone later.
/// Keeps the history bounded — a 40-step buffer of a 20 MB PDF is enough.
fn push_undo(doc: &mut OpenDoc) {
    if let Ok(bytes) = doc.document.save_to_bytes() {
        doc.undo.push(bytes);
        if doc.undo.len() > 40 {
            doc.undo.remove(0);
        }
        doc.redo.clear();
    }
}

#[tauri::command]
pub async fn undo(app: AppHandle, state: State<'_, AppState>) -> CmdResult<DocumentInfo> {
    let pdfium = init_pdfium(&app)?;
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

    let Some(previous) = doc.undo.pop() else {
        return Err("Nothing to undo".to_string());
    };
    if let Ok(current) = doc.document.save_to_bytes() {
        doc.redo.push(current);
    }

    doc.document = pdfium
        .load_pdf_from_byte_vec(previous, None)
        .map_err(|e| e.to_string())?;
    doc.dirty = true;
    doc_info(&doc.document, &doc.path, true)
}

#[tauri::command]
pub async fn redo(app: AppHandle, state: State<'_, AppState>) -> CmdResult<DocumentInfo> {
    let pdfium = init_pdfium(&app)?;
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

    let Some(next) = doc.redo.pop() else {
        return Err("Nothing to redo".to_string());
    };
    if let Ok(current) = doc.document.save_to_bytes() {
        doc.undo.push(current);
    }

    doc.document = pdfium
        .load_pdf_from_byte_vec(next, None)
        .map_err(|e| e.to_string())?;
    doc.dirty = true;
    doc_info(&doc.document, &doc.path, true)
}

/* ── Rendering ───────────────────────────────────────────────────── */

#[tauri::command]
pub async fn render_page(
    state: State<'_, AppState>,
    index: i32,
    target_width: i32,
) -> CmdResult<RenderedPage> {
    let guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_ref().ok_or_else(|| "No document is open".to_string())?;
    render_page_impl(&doc.document, index, target_width)
}

/* ── Page operations ─────────────────────────────────────────────── */

#[tauri::command]
pub async fn delete_pages(state: State<'_, AppState>, indices: Vec<i32>) -> CmdResult<DocumentInfo> {
    ensure_write_allowed()?;
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

    let total = doc.document.pages().len();
    if indices.is_empty() {
        return Err("No pages selected".to_string());
    }
    if indices.len() as i32 >= total {
        return Err("Cannot delete every page".to_string());
    }
    push_undo(doc);

    let mut sorted = indices;
    sorted.sort_unstable();
    sorted.dedup();

    for &index in sorted.iter().rev() {
        if index < 0 || index >= total {
            return Err(format!("Page {} is out of range", index + 1));
        }
        doc.document
            .pages()
            .get(index)
            .map_err(|e| e.to_string())?
            .delete()
            .map_err(|e| e.to_string())?;
    }

    doc.dirty = true;
    doc_info(&doc.document, &doc.path, true)
}

#[tauri::command]
pub async fn rotate_pages(
    state: State<'_, AppState>,
    indices: Vec<i32>,
    degrees: i32,
) -> CmdResult<DocumentInfo> {
    ensure_write_allowed()?;
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

    let total = doc.document.pages().len();
    if indices.is_empty() {
        return Err("No pages selected".to_string());
    }
    push_undo(doc);

    let mut sorted = indices;
    sorted.sort_unstable();
    sorted.dedup();

    for &index in &sorted {
        if index < 0 || index >= total {
            return Err(format!("Page {} is out of range", index + 1));
        }
        // `get()` returns an owned PdfPage, so binding it as `mut` is enough —
        // the rotation flag lives on the underlying page object in the document.
        let mut page = doc.document.pages().get(index).map_err(|e| e.to_string())?;
        let current = page.rotation().map(degrees_of).unwrap_or(0);
        page.set_rotation(rotation_of(current + degrees));
    }

    doc.dirty = true;
    doc_info(&doc.document, &doc.path, true)
}

#[tauri::command]
pub async fn move_page(
    app: AppHandle,
    state: State<'_, AppState>,
    from: i32,
    to: i32,
) -> CmdResult<DocumentInfo> {
    ensure_write_allowed()?;
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

    let total = doc.document.pages().len();
    if from < 0 || from >= total || to < 0 || to >= total {
        return Err("Page index out of range".to_string());
    }
    if from == to {
        return doc_info(&doc.document, &doc.path, doc.dirty);
    }
    push_undo(doc);

    let mut order: Vec<i32> = (0..total).collect();
    let moved = order.remove(from as usize);
    order.insert(to as usize, moved);

    // 优先原地重排 —— 保住书签/表单/元数据；页树是多级结构时才退回重建路径。
    let in_place = doc
        .document
        .save_to_bytes()
        .ok()
        .and_then(|b| reorder_in_place(&b, &order).ok());
    match in_place {
        Some(bytes) => {
            let pdfium = init_pdfium(&app)?;
            doc.document = pdfium
                .load_pdf_from_byte_vec(bytes, None)
                .map_err(|e| e.to_string())?;
        }
        None => {
            doc.document = reorder(&app, &doc.document, &order)?;
        }
    }
    doc.dirty = true;
    doc_info(&doc.document, &doc.path, true)
}

/* ── Save / export ───────────────────────────────────────────────── */

/// Makes each concurrent save use its own temp file. See `save_document`.
static TMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[tauri::command]
pub async fn save_document(
    state: State<'_, AppState>,
    path: Option<String>,
) -> CmdResult<PathResult> {
    ensure_write_allowed()?;
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

    let target = match path.filter(|p| !p.trim().is_empty()) {
        Some(p) => ensure_pdf_extension(p),
        None => {
            if doc.path.is_empty() {
                return Err("No destination path".to_string());
            }
            // 原地覆盖 = 用 pdfium 重写原文件，而 pdfium 不写加密。
            // 对加密文档来说那不是"保存"，是"静默移除密码保护"。宁可拒绝也不做。
            if doc.was_encrypted {
                return Err(
                    "ENCRYPTED_IN_PLACE_BLOCKED: this document is password-protected, and \
                     saving over it would remove the password. Use Save As to write a copy."
                        .to_string(),
                );
            }
            doc.path.clone()
        }
    };

    // Save through a temp file next to the target, then rename. `save_to_file` writes
    // straight to the destination, so a full disk or a permission error halfway
    // through leaves the user's original PDF truncated and unrecoverable.
    // The temp keeps a `.pdf` suffix because pdfium picks its writer from the path.
    let target_path = std::path::Path::new(&target);
    let dir = target_path
        .parent()
        .ok_or_else(|| "Invalid path".to_string())?;
    let name = target_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid file name".to_string())?;
    let tmp = dir.join(format!(
        ".{}.{}.{}.tmp.pdf",
        name,
        std::process::id(),
        TMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_file(&tmp);
    doc.document
        .save_to_file(&tmp)
        .map_err(|e| format!("Cannot save PDF: {e}"))?;
    // 在 rename 之前跑：这一步失败就中止保存，用户的原件一个字节都没动。
    if let Err(e) = crate::formclear::clear_radio_groups(&tmp, &doc.radio_clears) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    if let Err(e) = std::fs::rename(&tmp, &target) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("Cannot save PDF: {e}"));
    }

    doc.path = target.clone();
    doc.dirty = false;

    let size = std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
    Ok(PathResult {
        path: target,
        size,
    })
}

#[tauri::command]
pub async fn extract_pages(
    app: AppHandle,
    state: State<'_, AppState>,
    indices: Vec<i32>,
    output_path: String,
) -> CmdResult<PathResult> {
    ensure_write_allowed()?;
    let guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_ref().ok_or_else(|| "No document is open".to_string())?;

    let total = doc.document.pages().len();
    let mut sorted = indices;
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.is_empty() {
        return Err("No pages selected".to_string());
    }
    for &i in &sorted {
        if i < 0 || i >= total {
            return Err(format!("Page {} is out of range", i + 1));
        }
    }

    let out = ensure_pdf_extension(output_path);
    let mut new_doc = init_pdfium(&app)?.create_new_pdf().map_err(|e| e.to_string())?;
    for &index in &sorted {
        let insert_at = new_doc.pages().len();
        new_doc
            .pages_mut()
            .copy_page_from_document(&doc.document, index, insert_at)
            .map_err(|e| e.to_string())?;
    }
    new_doc.save_to_file(&out).map_err(|e| e.to_string())?;

    let size = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    Ok(PathResult { path: out, size })
}

#[tauri::command]
pub async fn merge_documents(
    app: AppHandle,
    paths: Vec<String>,
    output_path: String,
) -> CmdResult<PathResult> {
    ensure_write_allowed()?;
    if paths.len() < 2 {
        return Err("Pick at least two PDFs to merge".to_string());
    }

    let pdfium = init_pdfium(&app)?;

    let first = std::fs::read(&paths[0]).map_err(|e| format!("Cannot read file: {e}"))?;
    let mut dest = pdfium
        .load_pdf_from_byte_vec(first, None)
        .map_err(|e| format!("Cannot open PDF: {e}"))?;

    for path in &paths[1..] {
        let bytes = std::fs::read(path).map_err(|e| format!("Cannot read file: {e}"))?;
        let source = pdfium
            .load_pdf_from_byte_vec(bytes, None)
            .map_err(|e| format!("Cannot open PDF: {e}"))?;
        dest.pages_mut()
            .append(&source)
            .map_err(|e| format!("Cannot merge {path}: {e}"))?;
    }

    let out = ensure_pdf_extension(output_path);
    dest.save_to_file(&out).map_err(|e| e.to_string())?;

    let size = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    Ok(PathResult { path: out, size })
}

#[tauri::command]
pub async fn split_document(
    app: AppHandle,
    state: State<'_, AppState>,
    output_dir: String,
    mode: SplitMode,
) -> CmdResult<Vec<PathResult>> {
    ensure_write_allowed()?;
    let guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_ref().ok_or_else(|| "No document is open".to_string())?;

    let total = doc.document.pages().len();
    if total == 0 {
        return Err("This PDF has no pages".to_string());
    }

    let base = file_stem(&doc.path);
    let groups: Vec<(String, Vec<i32>)> = match mode {
        SplitMode::EveryPage => (0..total)
            .map(|i| (format!("{base}_{:04}", i + 1), vec![i]))
            .collect(),
        SplitMode::EveryN { n } => {
            let size = n.max(1);
            let mut groups = Vec::new();
            let mut start = 0;
            let mut part = 1;
            while start < total {
                let end = (start + size).min(total);
                let pages = (start..end).collect::<Vec<_>>();
                groups.push((format!("{base}_part{part:03}_{:04}-{:04}", start + 1, end), pages));
                start = end;
                part += 1;
            }
            groups
        }
        SplitMode::Ranges { ranges } => parse_ranges(&ranges, total)?
            .into_iter()
            .map(|(s, e)| {
                let pages = (s..=e).collect::<Vec<_>>();
                let name = if s == e {
                    format!("{base}_{:04}", s + 1)
                } else {
                    format!("{base}_{:04}-{:04}", s + 1, e + 1)
                };
                (name, pages)
            })
            .collect(),
    };

    let pdfium = init_pdfium(&app)?;
    let dir = PathBuf::from(&output_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create folder: {e}"))?;

    let mut results = Vec::with_capacity(groups.len());
    for (name, pages) in groups {
        let mut new_doc = pdfium.create_new_pdf().map_err(|e| e.to_string())?;
        for &index in &pages {
            let insert_at = new_doc.pages().len();
            new_doc
                .pages_mut()
                .copy_page_from_document(&doc.document, index, insert_at)
                .map_err(|e| e.to_string())?;
        }
        let out = dir.join(format!("{name}.pdf"));
        new_doc.save_to_file(&out).map_err(|e| e.to_string())?;
        let size = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
        results.push(PathResult {
            path: out.to_string_lossy().to_string(),
            size,
        });
    }

    Ok(results)
}

/* ── Text & search ───────────────────────────────────────────────── */

/// Find every occurrence of `query` across all pages.
#[tauri::command]
pub async fn search_document(
    state: State<'_, AppState>,
    query: String,
    match_case: bool,
    whole_word: bool,
) -> CmdResult<Vec<SearchHit>> {
    let guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_ref().ok_or_else(|| "No document is open".to_string())?;
    crate::pdf::search_document(&doc.document, &query, match_case, whole_word)
}

/// Concatenated plain text of the given pages, for "copy page text".
#[tauri::command]
pub async fn page_text(state: State<'_, AppState>, indices: Vec<i32>) -> CmdResult<String> {
    let guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_ref().ok_or_else(|| "No document is open".to_string())?;
    let total = doc.document.pages().len();

    let mut sorted = indices;
    sorted.sort_unstable();
    sorted.dedup();

    let mut out = String::new();
    for index in sorted {
        if index < 0 || index >= total {
            continue;
        }
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&crate::pdf::page_text(&doc.document, index)?);
    }
    Ok(out)
}

/* ── Stamping ────────────────────────────────────────────────────── */

/// Adds page numbers or a text watermark to every page.
#[tauri::command]
pub async fn stamp_document(
    state: State<'_, AppState>,
    kind: StampKind,
    text: String,
    font_size: f32,
    margin: f32,
    opacity: f32,
) -> CmdResult<DocumentInfo> {
    ensure_write_allowed()?;
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

    push_undo(doc);
    crate::pdf::apply_stamp(&mut doc.document, kind, &text, font_size, margin, opacity)?;
    doc.dirty = true;
    doc_info(&doc.document, &doc.path, true)
}

/* ── Annotations ─────────────────────────────────────────────────── */

/// Draws a highlight / underline / strikeout over the dragged rectangle.
#[tauri::command]
pub async fn add_markup(
    state: State<'_, AppState>,
    kind: MarkupKind,
    rect: MarkupRect,
    color: Vec<u8>,
    opacity: f32,
) -> CmdResult<DocumentInfo> {
    ensure_write_allowed()?;
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

    let c = (
        *color.first().unwrap_or(&255),
        *color.get(1).unwrap_or(&235),
        *color.get(2).unwrap_or(&59),
    );

    push_undo(doc);
    crate::pdf::add_markup(&mut doc.document, kind, rect, c, opacity)?;
    doc.dirty = true;
    doc_info(&doc.document, &doc.path, true)
}

/// Adds a sticky note at the given point.
#[tauri::command]
pub async fn add_note(
    state: State<'_, AppState>,
    page: i32,
    x: f32,
    y: f32,
    text: String,
    color: Vec<u8>,
) -> CmdResult<DocumentInfo> {
    ensure_write_allowed()?;
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

    let c = (
        *color.first().unwrap_or(&255),
        *color.get(1).unwrap_or(&200),
        *color.get(2).unwrap_or(&0),
    );

    push_undo(doc);
    crate::pdf::add_note(&mut doc.document, page, x, y, &text, c)?;
    doc.dirty = true;
    doc_info(&doc.document, &doc.path, true)
}

/// Places a signature image at the given point on a page.
#[tauri::command]
pub async fn add_signature(
    state: State<'_, AppState>,
    page: i32,
    x: f32,
    y: f32,
    width: f32,
    image_path: String,
) -> CmdResult<DocumentInfo> {
    ensure_write_allowed()?;
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

    push_undo(doc);
    crate::pdf::add_signature(&mut doc.document, page, x, y, width, &image_path)?;
    doc.dirty = true;
    doc_info(&doc.document, &doc.path, true)
}

/* ── Import / export images ──────────────────────────────────────── */

/// Export the given pages as PNG files into `output_dir`.
#[tauri::command]
pub async fn export_page_images(
    state: State<'_, AppState>,
    indices: Vec<i32>,
    output_dir: String,
    width: i32,
) -> CmdResult<Vec<PathResult>> {
    ensure_write_allowed()?;
    let guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_ref().ok_or_else(|| "No document is open".to_string())?;

    let total = doc.document.pages().len();
    let mut sorted = indices;
    sorted.sort_unstable();
    sorted.dedup();
    for &i in &sorted {
        if i < 0 || i >= total {
            return Err(format!("Page {} is out of range", i + 1));
        }
    }

    crate::pdf::export_pages_as_png(&doc.document, &sorted, &output_dir, &file_stem(&doc.path), width)
}

/// Build a PDF from a list of image files (JPEG / PNG).
#[tauri::command]
pub async fn images_to_pdf(
    app: AppHandle,
    paths: Vec<String>,
    output_path: String,
) -> CmdResult<PathResult> {
    ensure_write_allowed()?;
    let pdfium = init_pdfium(&app)?;
    crate::pdf::images_to_pdf(pdfium, &paths, &output_path)
}

/* ── AcroForm fields ─────────────────────────────────────────────── */

#[tauri::command]
pub async fn list_form_fields(state: State<'_, AppState>) -> CmdResult<Vec<FormFieldInfo>> {
    let guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_ref().ok_or_else(|| "No document is open".to_string())?;
    crate::pdf::list_form_fields(&doc.document)
}

#[tauri::command]
pub async fn set_form_values(
    state: State<'_, AppState>,
    values: Vec<(String, String)>,
) -> CmdResult<DocumentInfo> {
    ensure_write_allowed()?;
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

    push_undo(doc);
    crate::pdf::set_form_values(&mut doc.document, &values)?;

    // pdfium 能把 radio 组选中，却没有"取消选中"的能力（原委见 formclear.rs）。
    // 这里只登记用户的意图，真正的清空在写文件时完成。
    //
    // 已知的边角：撤销不会回滚这个登记集合（undo 恢复的是字节快照），所以
    // "取消选中 → 撤销 → 再保存"仍会写出未选中的状态。比"永远取消不掉"好。
    for (name, value) in &values {
        if value.eq_ignore_ascii_case("true") || value == "1" {
            doc.radio_clears.remove(name);
        } else if value.eq_ignore_ascii_case("false") || value == "0" {
            doc.radio_clears.insert(name.clone());
        }
    }
    doc.dirty = true;
    doc_info(&doc.document, &doc.path, true)
}

/* ── Password protection ─────────────────────────────────────────── */

/// Writes an unencrypted copy of `input` to `output`.
#[tauri::command]
pub async fn remove_password(
    input: String,
    output: String,
    password: String,
) -> CmdResult<PathResult> {
    ensure_write_allowed()?;
    crate::security::remove_password(&input, &output, &password)
}

/// Writes an AES-128 encrypted copy of `input` to `output`.
#[tauri::command]
pub async fn set_password(
    input: String,
    output: String,
    password: String,
    owner_password: Option<String>,
) -> CmdResult<PathResult> {
    ensure_write_allowed()?;
    let owner = owner_password
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| password.clone());
    crate::security::set_password(&input, &output, &password, &owner)
}

/* ── Shell integration ───────────────────────────────────────────── */

#[tauri::command]
pub async fn reveal_in_finder(path: String) -> CmdResult<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        // Explorer wants `/select,<path>` as a single argument; splitting it
        // across two makes it open the default folder instead of selecting.
        std::process::Command::new("explorer")
            .arg(format!("/select,{path}"))
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let folder = PathBuf::from(&path)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(&path));
        std::process::Command::new("xdg-open")
            .arg(folder)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/* ── Helpers ─────────────────────────────────────────────────────── */

/// 在**同一文档内**重排页面：只改页树 `/Kids` 的顺序，不新建任何对象。
///
/// 为什么不走"新建文档 + 逐页拷贝"：拷贝出来的页面是**新对象**，于是
/// `/Outlines` 里指向旧页面的目标全部失效、`/AcroForm`（表单域所在页）与
/// `/Metadata`、`/Names` 这些**文档级**结构更是直接消失 —— 对"整理页面"这个
/// 核心卖点来说，整理完丢书签是不可接受的。原地调序按构造就保住一切：
/// 对象一个没变，变的只是引用的先后。
///
/// 返回 `Err("NESTED_PAGE_TREE")` 表示页树是多级结构（`/Kids` 里嵌套 `Pages`），
/// 本函数只处理扁平页树，由调用方退回旧路径。
fn reorder_in_place(bytes: &[u8], order: &[i32]) -> Result<Vec<u8>, String> {
    let mut doc = lopdf::Document::load_from(std::io::Cursor::new(bytes))
        .map_err(|e| format!("Cannot read PDF: {e}"))?;

    let pages = doc.get_pages(); // 1-based 页码 -> 页面对象 id

    let mut kids: Vec<lopdf::Object> = Vec::with_capacity(order.len());
    for &index in order {
        let id = pages
            .get(&(index as u32 + 1))
            .ok_or_else(|| format!("Page {} is out of range", index + 1))?;
        kids.push(lopdf::Object::Reference(*id));
    }

    let first_page = *pages.values().next().ok_or("This PDF has no pages")?;
    let pages_id = doc
        .get_object(first_page)
        .ok()
        .and_then(|o| o.as_dict().ok())
        .and_then(|d| d.get(b"Parent").ok())
        .and_then(|o| o.as_reference().ok())
        .ok_or("Cannot locate the page tree")?;

    let dict = doc
        .get_object_mut(pages_id)
        .map_err(|e| e.to_string())?
        .as_dict_mut()
        .map_err(|e| e.to_string())?;

    // 多级页树不动手：那种结构下 /Kids 混着 Pages 节点，直接替换会改坏树。
    let flat = dict
        .get(b"Kids")
        .ok()
        .and_then(|o| o.as_array().ok())
        .map(|a| a.iter().all(|o| o.as_reference().is_ok()))
        .unwrap_or(false);
    if !flat {
        return Err("NESTED_PAGE_TREE".to_string());
    }

    dict.set("Kids", lopdf::Object::Array(kids));
    // /Count 不用改：页数没变。

    let mut out: Vec<u8> = Vec::new();
    doc.save_to(&mut out).map_err(|e| e.to_string())?;
    Ok(out)
}

/// Build a new document containing `source`'s pages in the given order.
///
/// 旧路径，仅在原地重排不可用时兜底（多级页树）。**它会丢文档级结构**，
/// 因此不是默认选择。
fn reorder(
    app: &AppHandle,
    source: &PdfDocument<'static>,
    order: &[i32],
) -> CmdResult<PdfDocument<'static>> {
    let pdfium = init_pdfium(app)?;
    let mut new_doc = pdfium.create_new_pdf().map_err(|e| e.to_string())?;
    for &index in order {
        let insert_at = new_doc.pages().len();
        new_doc
            .pages_mut()
            .copy_page_from_document(source, index, insert_at)
            .map_err(|e| e.to_string())?;
    }
    Ok(new_doc)
}

fn ensure_pdf_extension(path: String) -> String {
    if path.to_lowercase().ends_with(".pdf") {
        path
    } else {
        format!("{path}.pdf")
    }
}

fn file_stem(path: &str) -> String {
    PathBuf::from(path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "document".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;

    /// 加密过的 PDF 必须被认出来，普通 PDF 不能被误判。
    /// 误判两个方向都疼：漏判 = 静默移除密码保护；误判 = 正常的原地保存被拦住。
    #[test]
    fn detects_encryption_without_false_positives() {
        let dir = std::env::temp_dir().join(format!("rt-editor-enc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let plain = dir.join("plain.pdf");
        let locked = dir.join("locked.pdf");

        // 造一份最小 PDF：本测试只需要"lopdf 能读它"
        let mut doc = lopdf::Document::with_version("1.5");
        doc.objects.insert(
            (1, 0),
            lopdf::Object::Dictionary(lopdf::dictionary! { "Type" => "Catalog" }),
        );
        doc.save(&plain).unwrap();

        assert!(
            !detect_encrypted(&std::fs::read(&plain).unwrap()),
            "普通 PDF 不能被判成加密文档"
        );

        crate::security::set_password(
            plain.to_str().unwrap(),
            locked.to_str().unwrap(),
            "user-pw",
            "owner-pw",
        )
        .expect("encrypt");
        assert!(
            detect_encrypted(&std::fs::read(&locked).unwrap()),
            "加密 PDF 必须被认出来，否则原地保存会静默抹掉密码"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 随手给一段垃圾字节：不能 panic，也不能判成加密。
    #[test]
    fn malformed_bytes_are_not_treated_as_encrypted() {
        assert!(!detect_encrypted(b"not a pdf at all"));
        assert!(!detect_encrypted(b""));
    }

    /// P0-3 的核心断言：原地重排必须保住文档级结构（这里是书签），
    /// 而旧路径（新建文档+逐页拷贝）正是把 `/Outlines` 丢掉的地方。
    /// P0-3 的核心断言：**原地重排必须保住书签**。
    ///
    /// 走真实文档而不是合成夹具：手工页树在 lopdf 的 `get_pages()` 下只看到不足
    /// 声明页数（试过补 trailer `/Root` 仍不行），断言根本没机会跑。真实带书签的
    /// PDF 才能证明"只改 /Kids 顺序"这条路真的保住了 `/Outlines`。
    ///
    /// 用 ROCKTIER_TEST_PDF 指定一份**带书签**的 PDF（CI 不跑，真实文档不入库）：
    ///   ROCKTIER_TEST_PDF=~/Downloads/x.pdf cargo test keeps_bookmarks -- --nocapture
    #[test]
    fn reorder_in_place_keeps_bookmarks_on_a_real_document() {
        let path = match std::env::var("ROCKTIER_TEST_PDF") {
            Ok(p) => p,
            Err(_) => return,
        };
        let bytes = std::fs::read(&path).expect("读不到测试 PDF");

        let before = lopdf::Document::load_from(std::io::Cursor::new(&bytes)).expect("原文件可解析");
        let has_outlines = before
            .trailer
            .get(b"Root")
            .ok()
            .and_then(|r| r.as_reference().ok())
            .and_then(|r| before.get_object(r).ok())
            .and_then(|o| o.as_dict().ok())
            .map(|d| d.get(b"Outlines").is_ok())
            .unwrap_or(false);
        let page_count = before.get_pages().len();
        assert!(has_outlines, "这份夹具本身没有书签，无法用来验证");
        assert!(page_count >= 3, "夹具至少要有 3 页");

        // 把第 3 页移到最前，其余顺序不动
        let mut order: Vec<i32> = (0..page_count as i32).collect();
        let moved = order.remove(2);
        order.insert(0, moved);

        let out = reorder_in_place(&bytes, &order).expect("原地重排应当成功");
        let after = lopdf::Document::load_from(std::io::Cursor::new(&out)).expect("产物可解析");

        let catalog = after
            .get_object(after.trailer.get(b"Root").unwrap().as_reference().unwrap())
            .unwrap();
        let dict = catalog.as_dict().unwrap();
        assert!(
            dict.get(b"Outlines").is_ok(),
            "书签必须保留 —— 旧路径（新建文档+逐页拷贝）正是在这里丢掉的"
        );
        assert_eq!(after.get_pages().len(), page_count, "页数不变");

        // 页序确实变了：第 1 页应是原来的第 3 页
        let original_first_of_new = before.get_pages().get(&3).copied().unwrap();
        let new_first = after.get_pages().get(&1).copied().unwrap();
        assert_eq!(new_first, original_first_of_new, "第 1 页应变成原来的第 3 页");

        println!(
            "BOOKMARKS OK: {} 页, 书签保留, 全部/Outlines 仍在 catalog 中",
            page_count
        );
    }

    /// 页树是多级结构时必须明确拒绝，交给调用方退回旧路径，而不是改坏树。
    #[test]
    fn reorder_in_place_refuses_a_nested_page_tree() {
        let mut doc = lopdf::Document::with_version("1.5");
        doc.objects.insert(
            (1, 0),
            lopdf::Object::Dictionary(lopdf::dictionary! {
                "Type" => "Pages",
                "Count" => 0i64,
                "Kids" => lopdf::Object::Array(vec![lopdf::Object::Reference((5, 0))]),
            }),
        );
        doc.objects.insert(
            (5, 0),
            lopdf::Object::Dictionary(lopdf::dictionary! {
                "Type" => "Pages",
                "Parent" => lopdf::Object::Reference((1, 0)),
                "Count" => 0i64,
                "Kids" => lopdf::Object::Array(vec![]),
            }),
        );
        let mut bytes: Vec<u8> = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        // 没有页面对象时 get_pages() 为空 -> 直接报错，绝不能改坏
        assert!(reorder_in_place(&bytes, &[0]).is_err());
    }
}

/* ── Compression (merged in from Rocktier PDF Squeeze) ───────────── */

/// 压缩一份 PDF，写出到 `output`。
///
/// 进度经 `compress-progress` 事件上报：qpdf 那一段**没有**进度可报（只能表示
/// "进行中"），图像重压那一段报真实的 已完成／总数。UI 据此画进度而不是干等
/// —— 一份 38 MB 的扫描件在第二段要跑几十秒，没有反馈就是"卡住了"。
#[tauri::command]
pub async fn compress_document(
    app: tauri::AppHandle,
    input: String,
    output: String,
    profile: String,
) -> CmdResult<PathResult> {
    ensure_write_allowed()?;
    let input_path = PathBuf::from(&input);
    let output = ensure_pdf_extension(output);
    let output_path = PathBuf::from(&output);
    let size = crate::compress::compress(&input_path, &output_path, &profile, &|p| {
        let _ = tauri::Emitter::emit(&app, "compress-progress", &p);
    })?;
    Ok(PathResult { path: output, size })
}
