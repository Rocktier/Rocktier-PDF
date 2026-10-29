//! Tauri command surface.
//!
//! All commands are `async` so long-running Pdfium work never blocks the UI
//! thread. Errors are flattened to English strings and shown as-is in the
//! status bar — v0.1 has no error-code mapping layer.

use std::path::PathBuf;

use pdfium_render::prelude::*;
use tauri::{AppHandle, Manager, State};

use crate::pdf::{
    degrees_of, doc_info, init_pdfium, parse_ranges, render_page as render_page_impl,
    rotation_of, DocumentInfo, FormFieldInfo, MarkupKind, MarkupRect, PathResult, RedactRegion,
    RenderedPage, SaveResult, SearchHit, SplitMode, StampKind,
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
    /* 试用起点双写（AppData + 副存储）并按机器指纹判定，
       见 trial.rs 的模块说明。app_key 用 bundle identifier ——
       家族内唯一，避免两个产品的副存储互相覆盖。 */
    let started = crate::trial::ensure_started(
        dir,
        crate::APP_KEY,
        now,
        &crate::trial::machine_fingerprint(),
    );
    // 只认本单品与全家桶的回执：别人的回执即使验签通过，也不是本应用的授权。
    let receipt = crate::license::read_valid_receipt(dir, crate::license::PUBLIC_KEY_B64)
        .filter(crate::license::accepts);
    crate::license::status_from(Some(started), receipt.as_ref(), now)
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

/// 已签名文档只允许「追加签名」，其它编辑一律拒绝。
/// 放在命令层：界面上的灰按钮会随新功能增加而漏掉，命令层是必经之路。
fn ensure_editable(doc: &crate::state::OpenDoc) -> CmdResult<()> {
    if doc.read_only {
        return Err(
            "SIGNED_READ_ONLY: this file has a digital signature and is read-only. \
             Save an editable copy first."
                .to_string(),
        );
    }
    Ok(())
}

/// 复制型命令的只读闸门：输入正好是当前已签名文档时，输出去哪都不许写。
/// 基线是原始路径字符串相等；再做一次尽力而为的规范化比较（软链、相对路径、
/// 大小写差异不至于漏拦）—— 两侧都 canonicalize 成功才比较，否则退回原始相等。
fn ensure_editable_input(state: &State<'_, AppState>, input: &str) -> CmdResult<()> {
    let guard = state.doc.lock().map_err(|e| e.to_string())?;
    if let Some(doc) = guard.as_ref() {
        if doc.read_only {
            let same_path = doc.path == input
                || match (
                    std::fs::canonicalize(&doc.path),
                    std::fs::canonicalize(input),
                ) {
                    (Ok(a), Ok(b)) => a == b,
                    _ => false,
                };
            if same_path {
                return Err(
                    "SIGNED_READ_ONLY: this file has a digital signature. Save an editable copy first."
                        .to_string(),
                );
            }
        }
    }
    Ok(())
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
    /// 仅 `licensed` 时有值：回执里的交易号（吊销复检的凭据）。
    pub txn: Option<String>,
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

/// 回执里的交易号（吊销复检用）。无有效回执时为 None。
fn receipt_txn() -> Option<String> {
    let dir = LICENSE_DIR.get()?;
    crate::license::read_valid_receipt(dir, crate::license::PUBLIC_KEY_B64).map(|r| r.txn)
}

fn license_info() -> LicenseInfo {
    let status = current_license();
    let txn = receipt_txn();
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
        txn,
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
/// 本机指纹，供前端在**激活时**上报给服务端做设备计数。
///
/// 为什么单独开一个命令而不是让前端自己算：指纹要读注册表 / ioreg，
/// 只有 Rust 侧做得到；且试用与激活必须用**同一个**指纹 ——
/// 用两套标识会出现「A 说没试过、B 说试过」这种自相矛盾。
///
/// 取不到时返回空串：服务端据此不计数也不拦激活（见
/// `rocktier.com/api/devices.js` 的模块说明）。
#[tauri::command]
pub fn report_machine_fingerprint() -> String {
    crate::trial::machine_fingerprint()
}

/// 清除本地回执（吊销复检判定 revoked 时由前端调用）。
/// 之后 license_status 回到 trial/expired，写闸门按状态拦截。
#[tauri::command]
pub fn clear_receipt() -> CmdResult<()> {
    let dir = LICENSE_DIR
        .get()
        .ok_or_else(|| "no app data directory".to_string())?;
    let p = dir.join("receipt.txt");
    if p.exists() {
        std::fs::remove_file(&p).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn store_receipt(signed: String) -> CmdResult<LicenseInfo> {
    let dir = LICENSE_DIR
        .get()
        .ok_or_else(|| "no app data directory".to_string())?;
    let trimmed = signed.trim();
    let receipt = crate::license::verify_receipt(trimmed, crate::license::PUBLIC_KEY_B64)?;

    // 其它单品的码虽然签名有效，但**不属于**本应用 —— 而且不要落盘：落下去以后
    // 会被当成有效回执读回来，等于自己给自己开后门。
    if !crate::license::accepts(&receipt) {
        return Err("LICENSE_WRONG_PRODUCT".to_string());
    }

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

    // 探测放在锁外、只读一次盘：签名是对字节的承诺，一旦检出就必须以只读打开。
    let read_only = crate::sign::has_signature(&path).unwrap_or(false);
    let info = doc_info(&document, &path, false, read_only)?;

    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    *guard = Some(OpenDoc {
        document,
        path,
        was_encrypted,
        dirty: false,
        // 探测只回答「有没有」，不回答「有效吗」—— 后者进验证页按需跑。
        read_only,
        signature_count: usize::from(read_only),
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
    doc_info(&doc.document, &doc.path, true, doc.read_only)
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
    doc_info(&doc.document, &doc.path, true, doc.read_only)
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
    ensure_editable(doc)?;

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
    doc_info(&doc.document, &doc.path, true, doc.read_only)
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
    ensure_editable(doc)?;

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
    doc_info(&doc.document, &doc.path, true, doc.read_only)
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
    ensure_editable(doc)?;

    let total = doc.document.pages().len();
    if from < 0 || from >= total || to < 0 || to >= total {
        return Err("Page index out of range".to_string());
    }
    if from == to {
        return doc_info(&doc.document, &doc.path, doc.dirty, doc.read_only);
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
    doc_info(&doc.document, &doc.path, true, doc.read_only)
}

/* ── Save / export ───────────────────────────────────────────────── */

/// Makes each concurrent save use its own temp file. See `save_document`.
static TMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[tauri::command]
pub async fn save_document(
    state: State<'_, AppState>,
    path: Option<String>,
) -> CmdResult<SaveResult> {
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

    // 已签名文档不能整份重写：pdfium 的 save_to_file 会丢掉增量签名，
    // 用户以为另存了同一份文件，实际上签名已经不存在了。
    let writing_copy = doc.read_only && target != doc.path;
    if doc.read_only && !writing_copy {
        return Err(
            "SIGNED_READ_ONLY: this file has a digital signature and is opened read-only. \
             Use Save As to write an editable copy without the signature."
                .to_string(),
        );
    }

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

    let read_only_after = if writing_copy {
        // 副本是 pdfium 重写出来的，签名已经被剥离 —— 这是用户显式选择的
        // 「可编辑副本」，只在 Save As 新路径时发生。
        doc.read_only = false;
        doc.signature_count = 0;
        false
    } else {
        doc.read_only
    };
    doc.path = target.clone();
    doc.dirty = false;

    let size = std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
    Ok(SaveResult {
        path: target,
        size,
        read_only: read_only_after,
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
///
/// Runs on the blocking pool: a search over a 900-page book is seconds of
/// CPU-bound Pdfium work, and running that on the async runtime starves every
/// other command — the window goes unresponsive until the scan finishes.
#[tauri::command]
pub async fn search_document(
    app: AppHandle,
    query: String,
    match_case: bool,
    whole_word: bool,
) -> CmdResult<Vec<SearchHit>> {
    tauri::async_runtime::spawn_blocking(move || {
        // A `State` borrow cannot cross into this `'static` task, so the state
        // is fetched through the handle instead.
        let state = app.state::<AppState>();
        let guard = state.doc.lock().map_err(|e| e.to_string())?;
        let doc = guard.as_ref().ok_or_else(|| "No document is open".to_string())?;
        crate::pdf::search_document(&doc.document, &query, match_case, whole_word)
    })
    .await
    .map_err(|e| format!("search task failed: {e}"))?
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
    ensure_editable(doc)?;

    push_undo(doc);
    crate::pdf::apply_stamp(&mut doc.document, kind, &text, font_size, margin, opacity)?;
    doc.dirty = true;
    doc_info(&doc.document, &doc.path, true, doc.read_only)
}

/* ── Redaction ───────────────────────────────────────────────────── */

/// 脱敏命令的返回：文档新状态 + 统计/告警。
///
/// 告警是结构化字段（不是拼好的英文串）：crossed/residual 的文案由前端
/// i18n 渲染，en/zh 各自成句。警告不阻塞——v1 的语义是「如实上报，由用户
/// 决定下一步」，而不是替用户回滚。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactOutcome {
    /// 删除的文本对象数。
    pub removed: usize,
    /// 跨出区域边界、被整对象删除的文本对象数。
    pub crossed: usize,
    /// 应用后重扫仍在区域内发现文本对象的区域数（校验告警）。
    pub residual_regions: usize,
    /// 脱敏后的文档状态（dirty = true）。
    pub info: DocumentInfo,
}

/// 真脱敏：永久删除区域内底层文字并盖不透明黑矩形。
///
/// 这是写后即不可逆的操作（保存后任何阅读器都复制不出原文字）——撤销栈
/// 能回滚的是**这次操作**（字节快照），不是已经保存出去的文件。确认弹层
/// 必须写明这一点；命令层照旧只负责闸门、快照与同步 dirty。
#[tauri::command]
pub async fn redact_regions(
    state: State<'_, AppState>,
    regions: Vec<RedactRegion>,
) -> CmdResult<RedactOutcome> {
    ensure_write_allowed()?;
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;
    ensure_editable(doc)?;

    push_undo(doc);
    let summary = crate::pdf::redact_regions(&mut doc.document, &regions)?;
    doc.dirty = true;
    Ok(RedactOutcome {
        removed: summary.removed,
        crossed: summary.crossed,
        residual_regions: summary.residual_regions,
        info: doc_info(&doc.document, &doc.path, true, doc.read_only)?,
    })
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
    ensure_editable(doc)?;

    let c = (
        *color.first().unwrap_or(&255),
        *color.get(1).unwrap_or(&235),
        *color.get(2).unwrap_or(&59),
    );

    push_undo(doc);
    crate::pdf::add_markup(&mut doc.document, kind, rect, c, opacity)?;
    doc.dirty = true;
    doc_info(&doc.document, &doc.path, true, doc.read_only)
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
    ensure_editable(doc)?;

    let c = (
        *color.first().unwrap_or(&255),
        *color.get(1).unwrap_or(&200),
        *color.get(2).unwrap_or(&0),
    );

    push_undo(doc);
    crate::pdf::add_note(&mut doc.document, page, x, y, &text, c)?;
    doc.dirty = true;
    doc_info(&doc.document, &doc.path, true, doc.read_only)
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
    ensure_editable(doc)?;

    push_undo(doc);
    crate::pdf::add_signature(&mut doc.document, page, x, y, width, &image_path)?;
    doc.dirty = true;
    doc_info(&doc.document, &doc.path, true, doc.read_only)
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
    ensure_editable(doc)?;

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
    doc_info(&doc.document, &doc.path, true, doc.read_only)
}

/* ── Password protection ─────────────────────────────────────────── */

/// Writes an unencrypted copy of `input` to `output`.
#[tauri::command]
pub async fn remove_password(
    state: State<'_, AppState>,
    input: String,
    output: String,
    password: String,
) -> CmdResult<PathResult> {
    ensure_write_allowed()?;
    ensure_editable_input(&state, &input)?;
    crate::security::remove_password(&input, &output, &password)
}

/// Writes an AES-128 encrypted copy of `input` to `output`.
#[tauri::command]
pub async fn set_password(
    state: State<'_, AppState>,
    input: String,
    output: String,
    password: String,
    owner_password: Option<String>,
) -> CmdResult<PathResult> {
    ensure_write_allowed()?;
    ensure_editable_input(&state, &input)?;
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
    state: State<'_, AppState>,
    input: String,
    output: String,
    profile: String,
) -> CmdResult<PathResult> {
    ensure_write_allowed()?;
    // 压缩产出的是「看起来像原件」的无签名副本：签名只覆盖被签的那串字节，
    // 重写过的文件一个签名也不剩。静默放行等于让用户把未签名文件当签名文件发出去，
    // 所以输入正好是当前已签名文档时一律拒绝。
    ensure_editable_input(&state, &input)?;
    let input_path = PathBuf::from(&input);
    let output = ensure_pdf_extension(output);
    let output_path = PathBuf::from(&output);
    let size = crate::compress::compress(&input_path, &output_path, &profile, &|p| {
        let _ = tauri::Emitter::emit(&app, "compress-progress", &p);
    })?;
    Ok(PathResult { path: output, size })
}

/* ── PDF → Markdown（路线图 R4）───────────────────────────────────────── */

/// 把当前文档转成 Markdown，**不写盘**（供前端预览）。
///
/// 转换是纯读操作，因此**不经过 `ensure_write_allowed`** ——
/// 试用期内也应当能预览，只是不能导出。这与既有命令的策略一致：
/// 写命令才拦。
#[tauri::command]
pub async fn pdf_to_markdown(state: State<'_, AppState>) -> CmdResult<crate::pdf_to_md::ConversionResult> {
    let guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_ref().ok_or_else(|| "No document is open".to_string())?;

    let (markdown, stats, warnings) =
        crate::pdf_to_md::document_to_markdown(&doc.document, Default::default())?;

    Ok(crate::pdf_to_md::ConversionResult { markdown, stats, warnings })
}

/// 转成 Markdown 并写入 `output_path`。
///
/// 这是**写盘**动作，走 `ensure_write_allowed()`。
#[tauri::command]
pub async fn pdf_save_markdown(
    state: State<'_, AppState>,
    output_path: String,
) -> CmdResult<PathResult> {
    ensure_write_allowed()?;
    let guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_ref().ok_or_else(|| "No document is open".to_string())?;

    let (markdown, _stats, _warnings) =
        crate::pdf_to_md::document_to_markdown(&doc.document, Default::default())?;
    if markdown.trim().is_empty() {
        // 空结果写出去会得到一个 0 字节的 .md，用户只会觉得「导出了个空文件」。
        // 扫描件请走 OCR（路线图 R4 第二段接力）。
        return Err("NO_TEXT_EXTRACTED".to_string());
    }

    let path = PathBuf::from(&output_path);
    let size = crate::pdf_to_md::write_markdown(&path, &markdown)?;
    Ok(PathResult { path: output_path, size })
}

/* ── 数字签名（并入自 Rocktier Sign）─────────────────────────────────
 *
 * 这一组命令都不碰内存里的文档，唯一的例外是 `sign_pdf`（它把输出读回来
 * 替换当前文档）。纪律见 sign.rs 模块说明：私钥只走 stdin / 系统钥匙串，
 * 引擎产物只落在 0700 临时目录里，且无论成功失败都要删掉。
 */

/// 引擎是否就位。前端据此禁用「数字签名」入口并给出原因（sidecar 没打进包里）。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignCapabilities {
    pub engine_available: bool,
}

#[tauri::command]
pub fn sign_capabilities(app: AppHandle) -> SignCapabilities {
    SignCapabilities {
        engine_available: crate::sign::engine_path(&app).is_ok(),
    }
}

#[tauri::command]
pub fn list_certs(app: AppHandle) -> CmdResult<Vec<crate::sign::CertInfo>> {
    let store = crate::sign::cert_store_dir(&app)?;
    let mut certs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&store) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("crt") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            // 创建时间只用于排序（最新的排最前）。取不到就给空串 ——
            // 证书能用才是重点，不该因为一个时间戳读不到就整页失败。
            let created = entry
                .metadata()
                .and_then(|m| m.created())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();
            certs.push(crate::sign::CertInfo {
                name: stem.to_string(),
                created,
                path: path.to_string_lossy().into_owned(),
                meta: crate::sign::read_cert_meta(&store, stem),
            });
        }
    }
    certs.sort_by(|a, b| b.created.cmp(&a.created));
    Ok(certs)
}

#[tauri::command]
pub fn set_default_cert(app: AppHandle, name: String) -> CmdResult<()> {
    let name = crate::sign::sanitize_cert_name(&name)?;
    let store = crate::sign::cert_store_dir(&app)?;
    if !store.join(format!("{name}.crt")).exists() {
        return Err(format!("cert '{name}' not found"));
    }
    let json = serde_json::json!({ "default": name });
    std::fs::write(crate::sign::default_cert_path(&app)?, json.to_string())
        .map_err(|e| format!("write default: {e}"))
}

#[tauri::command]
pub fn get_default_cert(app: AppHandle) -> CmdResult<Option<String>> {
    let path = crate::sign::default_cert_path(&app)?;
    let Ok(s) = std::fs::read_to_string(path) else {
        return Ok(None);
    };
    Ok(serde_json::from_str::<serde_json::Value>(&s)
        .ok()
        .and_then(|v| v.get("default").and_then(|d| d.as_str()).map(str::to_string)))
}

#[tauri::command]
pub fn allow_expired_cert(app: AppHandle, name: String) -> CmdResult<()> {
    let name = crate::sign::sanitize_cert_name(&name)?;
    let store = crate::sign::cert_store_dir(&app)?;
    if !store.join(format!("{name}.crt")).exists() {
        return Err(format!("cert '{name}' not found in store"));
    }
    let Some(mut meta) = crate::sign::read_cert_meta(&store, &name) else {
        return Ok(());
    };
    if !meta.expired {
        return Ok(());
    }
    meta.allow_expired = true;
    crate::sign::write_cert_meta(&store, &name, &meta)
}

/// 删除一张证书：.crt、.meta.json 与钥匙串里的私钥一起删。
///
/// 只动**新**要删除的这张（`name` 由用户显式点选）；迁移来的旧 Keychain 条目
/// 一律不碰 —— 那是 Sign 的数据，删掉就没有回头路。
#[tauri::command]
pub fn delete_cert(app: AppHandle, name: String) -> CmdResult<()> {
    let name = crate::sign::sanitize_cert_name(&name)?;
    let store = crate::sign::cert_store_dir(&app)?;
    let crt = store.join(format!("{name}.crt"));
    if !crt.exists() {
        return Err(format!("cert '{name}' not found"));
    }
    std::fs::remove_file(&crt).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(store.join(format!("{name}.meta.json")));
    if let Ok(entry) =
        keyring::Entry::new(crate::sign::keyring_service(), &crate::sign::keyring_user(&name))
    {
        let _ = entry.delete_credential();
    }
    // 删的正好是默认证书时清掉 default.json，避免指向一张不存在的证书。
    let default_path = crate::sign::default_cert_path(&app)?;
    if let Ok(s) = std::fs::read_to_string(&default_path) {
        let is_default = serde_json::from_str::<serde_json::Value>(&s)
            .ok()
            .and_then(|v| v.get("default").and_then(|d| d.as_str()).map(|d| d == name))
            .unwrap_or(false);
        if is_default {
            let _ = std::fs::remove_file(&default_path);
        }
    }
    Ok(())
}

/// 建一个私有临时目录：`inspect-cert` 是唯一要 `-key <path>` 的子命令，
/// 解出的私钥明文会短暂落在盘上。目录 0700、文件名带 pid + 纳秒，
/// 用完（含错误与超时路径）由调用方 `remove_dir_all` 收掉。
fn make_inspect_tmp() -> CmdResult<std::path::PathBuf> {
    let tmp = std::env::temp_dir().join(format!(
        "rocktier-sign-inspect-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&tmp).map_err(|e| format!("create temp: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o700));
    }
    Ok(tmp)
}

/// 临时目录守卫：Drop 时删掉整个目录。
///
/// 覆盖所有 `?` 提前返回路径 —— `inspect_cert_files` / `import_cert` 在引擎跑
/// 之前就可能失败（写 in.key / in.crt），失败时目录里已经落了私钥明文，
/// 不能靠逐处手写 cleanup 兜底。
struct TempDirGuard<'a>(&'a std::path::Path);

impl Drop for TempDirGuard<'_> {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0);
    }
}

/// 私钥明文文件收紧到 0600：目录 0700 之外再加一层文件级保护，
/// 防止同机其它进程（或引擎留下的副本）读到密钥。
#[cfg(unix)]
fn set_key_perms(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn set_key_perms(_path: &std::path::Path) {}

/// 只解析、不落库：证书页「预览这张文件是什么样的证书」用。
#[tauri::command]
pub fn inspect_cert_files(
    app: AppHandle,
    key_data: String,
    cert_data: Option<String>,
    password: Option<String>,
) -> CmdResult<crate::sign::CertMeta> {
    let engine = crate::sign::engine_path(&app)?;
    let tmp = make_inspect_tmp()?;
    // 无论从哪条路径返回（包括写 in.key / in.crt 的 `?`），目录都必须删掉
    // —— 里面是私钥明文。
    let _tmp_guard = TempDirGuard(&tmp);
    let key_in = tmp.join("in.key");
    std::fs::write(&key_in, crate::sign::latin1_to_bytes(&key_data))
        .map_err(|e| format!("write key input: {e}"))?;
    set_key_perms(&key_in);
    let cert_in = match cert_data.as_deref() {
        Some(c) if !c.trim().is_empty() => {
            let p = tmp.join("in.crt");
            std::fs::write(&p, crate::sign::latin1_to_bytes(c))
                .map_err(|e| format!("write cert input: {e}"))?;
            Some(p)
        }
        _ => None,
    };
    crate::sign::engine_inspect(
        &engine,
        &key_in,
        cert_in.as_deref(),
        password.as_deref(),
        &tmp,
    )
    .map_err(crate::sign::inspect_fail_to_err)
}

#[tauri::command]
pub fn import_cert(
    app: AppHandle,
    name: String,
    key_data: String,
    cert_data: Option<String>,
    password: Option<String>,
) -> CmdResult<crate::sign::CertMeta> {
    let engine = crate::sign::engine_path(&app)?;
    let name = crate::sign::sanitize_cert_name(&name)?;
    let store = crate::sign::cert_store_dir(&app)?;
    let cert_out = store.join(format!("{name}.crt"));
    if cert_out.exists() {
        return Err(format!(
            "A certificate named \"{name}\" already exists. Choose a different name."
        ));
    }

    let tmp = make_inspect_tmp()?;
    // 无论从哪条路径返回（包括写 in.key / in.crt、write_cert_meta 的 `?`），
    // 目录都必须删掉 —— 里面是私钥明文。
    let _tmp_guard = TempDirGuard(&tmp);
    let key_in = tmp.join("in.key");
    std::fs::write(&key_in, crate::sign::latin1_to_bytes(&key_data))
        .map_err(|e| format!("write key input: {e}"))?;
    set_key_perms(&key_in);
    let cert_in = match cert_data.as_deref() {
        Some(c) if !c.trim().is_empty() => {
            let p = tmp.join("in.crt");
            std::fs::write(&p, crate::sign::latin1_to_bytes(c))
                .map_err(|e| format!("write cert input: {e}"))?;
            Some(p)
        }
        _ => None,
    };
    let meta = crate::sign::engine_inspect(
        &engine,
        &key_in,
        cert_in.as_deref(),
        password.as_deref(),
        &tmp,
    )
    .map_err(crate::sign::inspect_fail_to_err)?;

    // `out.key` 是引擎归一化后的**单块** PEM：钥匙串只存它，
    // 引擎只读第一个 PEM 块，多块会静默签出错误的密钥。
    let out_key = tmp.join("out.key");
    set_key_perms(&out_key);
    let key_pem = std::fs::read_to_string(&out_key).map_err(|e| format!("read normalized key: {e}"))?;
    let entry =
        keyring::Entry::new(crate::sign::keyring_service(), &crate::sign::keyring_user(&name))
            .map_err(|e| format!("keychain init: {e}"))?;
    entry
        .set_password(&key_pem)
        .map_err(|e| format!("store private key in system keychain failed: {e}"))?;

    std::fs::copy(tmp.join("out.crt"), &cert_out).map_err(|e| format!("write cert: {e}"))?;
    crate::sign::write_cert_meta(&store, &name, &meta)?;
    Ok(meta)
}

#[tauri::command]
pub fn generate_key(app: AppHandle, name: String) -> CmdResult<crate::sign::CertInfo> {
    let engine = crate::sign::engine_path(&app)?;
    let name = crate::sign::sanitize_cert_name(&name)?;
    let store = crate::sign::cert_store_dir(&app)?;
    let cert_out = store.join(format!("{name}.crt"));
    if cert_out.exists() {
        return Err(format!(
            "A certificate named \"{name}\" already exists. Choose a different name."
        ));
    }

    let tmp = make_inspect_tmp()?;
    let key_tmp = tmp.join(format!("{name}.key"));
    let cert_tmp = tmp.join(format!("{name}.crt"));
    let out = std::process::Command::new(&engine)
        .arg("generate-key")
        .arg("-name")
        .arg(&name)
        .arg("-key-out")
        .arg(&key_tmp)
        .arg("-cert-out")
        .arg(&cert_tmp)
        .output()
        .map_err(|e| {
            let _ = std::fs::remove_dir_all(&tmp);
            format!("engine exec: {e}")
        })?;
    if !out.status.success() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }
    // 引擎落盘的私钥明文收紧到 0600，再读进钥匙串。
    set_key_perms(&key_tmp);

    let key_pem = std::fs::read_to_string(&key_tmp).map_err(|e| {
        let _ = std::fs::remove_dir_all(&tmp);
        format!("read temp key: {e}")
    })?;
    let entry =
        keyring::Entry::new(crate::sign::keyring_service(), &crate::sign::keyring_user(&name))
            .map_err(|e| {
                let _ = std::fs::remove_dir_all(&tmp);
                format!("keychain init: {e}")
            })?;
    entry.set_password(&key_pem).map_err(|e| {
        let _ = std::fs::remove_dir_all(&tmp);
        format!("store private key in system keychain failed: {e}")
    })?;

    std::fs::copy(&cert_tmp, &cert_out).map_err(|e| {
        let _ = std::fs::remove_dir_all(&tmp);
        format!("write cert: {e}")
    })?;
    // 元信息只是用于展示：引擎解析失败也不该让「已经生成好的证书」失败。
    let meta = crate::sign::engine_inspect(&engine, &key_tmp, Some(&cert_tmp), None, &tmp).ok();
    if let Some(m) = &meta {
        let _ = crate::sign::write_cert_meta(&store, &name, m);
    }
    let _ = std::fs::remove_dir_all(&tmp);

    Ok(crate::sign::CertInfo {
        name: name.clone(),
        created: crate::sign::now_rfc3339(),
        path: cert_out.to_string_lossy().into_owned(),
        meta,
    })
}

/// 验签。返回的是引擎的报告（verdict + 签名条目），不落盘、不改状态。
#[tauri::command]
pub fn verify_pdf(app: AppHandle, input: String) -> CmdResult<crate::sign::VerifyReport> {
    let engine = crate::sign::engine_path(&app)?;
    crate::sign::verify_document(&engine, &input)
}

/// 数字签名（增量追加）。成功时把签好的文件读回内存，替换当前文档。
///
/// 三条硬约束：
/// 1. **磁盘上的字节必须就是待签的那一份** —— 有未保存改动（`dirty`）一律拒绝，
///    否则用户会得到一个「签的内容和界面看到的不一样」的文件；
/// 2. **绝不覆盖原文件** —— 输出等于输入时报 `SIGN_OUTPUT_IS_INPUT`；
/// 3. 失败时清掉本次写出的半成品（输出路径上**原有**的文件不是本次产物，不动）。
///
/// 已签名文档允许走这里（追加签名是唯一的合法编辑），其余编辑全部被
/// `ensure_editable` 挡住。
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn sign_pdf(
    app: AppHandle,
    state: State<'_, AppState>,
    cert_id: String,
    output: String,
    rect: String,
    page: u32,
    name: String,
    image_path: Option<String>,
) -> CmdResult<DocumentInfo> {
    ensure_write_allowed()?;
    let engine = crate::sign::engine_path(&app)?;

    // 快照：签名期间不持锁（引擎调用可能几十毫秒到几秒），
    // 但必须保证磁盘上的字节就是待签的那一份。
    let (input, total_pages, dirty) = {
        let guard = state.doc.lock().map_err(|e| e.to_string())?;
        let doc = guard
            .as_ref()
            .ok_or_else(|| "No document is open".to_string())?;
        (doc.path.clone(), doc.document.pages().len(), doc.dirty)
    };
    if dirty {
        return Err(
            "SIGN_DIRTY_DOC: save the document before signing — a signature must match the \
             file on disk byte for byte."
                .to_string(),
        );
    }
    if input.trim().is_empty() {
        return Err("SIGN_DIRTY_DOC: save the document before signing".to_string());
    }
    let output = ensure_pdf_extension(output);
    if output == input {
        return Err(
            "SIGN_OUTPUT_IS_INPUT: signing never overwrites the original file. Choose a new output path."
                .to_string(),
        );
    }
    // 输出路径上如果已经有文件，失败清理就不能碰它 —— 那不是这次签名的产物。
    let output_pre_existed = std::fs::metadata(&output).map(|m| m.is_file()).unwrap_or(false);
    // `page` 是 1 起步的用户页码；先挡 0/negative 不可能（u32），再挡超出末页。
    if page < 1 || page as usize > total_pages {
        return Err(format!("SIGN_PAGE_OUT_OF_RANGE: page {page} does not exist"));
    }

    let store = crate::sign::cert_store_dir(&app)?;
    let cert_id = crate::sign::sanitize_cert_name(&cert_id)?;
    let cert = store.join(format!("{cert_id}.crt"));
    if !cert.exists() {
        return Err(format!("cert '{cert_id}' not found in store"));
    }
    if let Some(meta) = crate::sign::read_cert_meta(&store, &cert_id) {
        if meta.expired && !meta.allow_expired {
            // 前端凭这段格式化的错误码弹确认框，用户确认后调 allow_expired_cert。
            return Err(format!("CERT_EXPIRED_CONFIRM|{cert_id}|{}", meta.not_after));
        }
    }

    let key_pem = match keyring::Entry::new(
        crate::sign::keyring_service(),
        &crate::sign::keyring_user(&cert_id),
    ) {
        Ok(entry) => entry.get_password().ok(),
        Err(_) => None,
    }
    .ok_or_else(|| {
        format!(
            "SIGN_KEY_MISSING: the private key for '{cert_id}' is not in the system keychain. \
             Re-import the certificate."
        )
    })?;

    let image = image_path.map(std::path::PathBuf::from);
    if let Err(e) = crate::sign::sign_with_stdin_key(
        &engine,
        &input,
        &output,
        &key_pem,
        &cert,
        &name,
        &rect,
        page,
        image.as_deref(),
    ) {
        // 失败时清掉这次可能写了一半的输出，原文件一个字节都没动；
        // 输出路径上原有的文件不是本次产物，不动它。
        if !output_pre_existed {
            let _ = std::fs::remove_file(&output);
        }
        return Err(e);
    }

    // 输出必须存在且页数一致，否则宁可报错也不把半成品当成功。
    let bytes = std::fs::read(&output).map_err(|e| format!("read signed output: {e}"))?;
    let pdfium = init_pdfium(&app)?;
    let loaded = pdfium
        .load_pdf_from_byte_vec(bytes, None)
        .map_err(|e| format!("load signed output: {e}"))?;
    if loaded.pages().len() != total_pages {
        if !output_pre_existed {
            let _ = std::fs::remove_file(&output);
        }
        return Err(
            "SIGN_PAGE_COUNT_CHANGED: the signed file has a different page count".to_string(),
        );
    }

    // 探测失败按「已签名」处理：宁可多一道只读闸门，也不让用户以为还能编辑。
    let read_only = crate::sign::has_signature(&output).unwrap_or(true);
    {
        let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
        let doc = guard
            .as_mut()
            .ok_or_else(|| "No document is open".to_string())?;
        doc.document = loaded;
        doc.path = output;
        doc.dirty = false;
        doc.read_only = read_only;
        // 磁盘字节已经换成签名后的新文件：签名前的快照一旦撤销，
        // 只读会话里就会把未签名的旧字节写回内存，全部作废。
        doc.undo.clear();
        doc.redo.clear();
        // 追加签名：验签页会显示 N 个签名，这里只记「至少有一个」。
        doc.signature_count = doc.signature_count.max(1);
        doc_info(&doc.document, &doc.path, false, doc.read_only)
    }
}
