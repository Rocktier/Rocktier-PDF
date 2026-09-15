//! Tauri command surface.
//!
//! All commands are `async` so long-running Pdfium work never blocks the UI
//! thread. Errors are flattened to `String` and mapped to i18n keys on the
//! frontend.

use std::path::PathBuf;

use pdfium_render::prelude::*;
use tauri::{AppHandle, State};

use crate::pdf::{
    degrees_of, doc_info, init_pdfium, parse_ranges, render_page as render_page_impl,
    rotation_of, DocumentInfo, PathResult, RenderedPage, SplitMode,
};
use crate::state::{AppState, OpenDoc};

type CmdResult<T> = Result<T, String>;

/* ── Lifecycle ───────────────────────────────────────────────────── */

#[tauri::command]
pub async fn open_document(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> CmdResult<DocumentInfo> {
    let pdfium = init_pdfium(&app)?;

    // Load from bytes rather than a file handle: it keeps the source file
    // unlocked so "Save" can overwrite the original on Windows.
    let bytes = std::fs::read(&path).map_err(|e| format!("Cannot read file: {e}"))?;
    let document = pdfium
        .load_pdf_from_byte_vec(bytes, None)
        .map_err(|e| format!("Cannot open PDF: {e}"))?;

    if document.pages().len() == 0 {
        return Err("This PDF has no pages.".to_string());
    }

    let info = doc_info(&document, &path, false)?;

    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    *guard = Some(OpenDoc {
        document,
        path,
        dirty: false,
    });

    Ok(info)
}

#[tauri::command]
pub async fn get_document(state: State<'_, AppState>) -> CmdResult<Option<DocumentInfo>> {
    let guard = state.doc.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(d) => Ok(Some(doc_info(&d.document, &d.path, d.dirty)?)),
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn close_document(state: State<'_, AppState>) -> CmdResult<()> {
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    *guard = None;
    Ok(())
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
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

    let total = doc.document.pages().len();
    if indices.is_empty() {
        return Err("No pages selected".to_string());
    }
    if indices.len() as i32 >= total {
        return Err("Cannot delete every page".to_string());
    }

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
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

    let total = doc.document.pages().len();
    if indices.is_empty() {
        return Err("No pages selected".to_string());
    }

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
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

    let total = doc.document.pages().len();
    if from < 0 || from >= total || to < 0 || to >= total {
        return Err("Page index out of range".to_string());
    }
    if from == to {
        return doc_info(&doc.document, &doc.path, doc.dirty);
    }

    let mut order: Vec<i32> = (0..total).collect();
    let moved = order.remove(from as usize);
    order.insert(to as usize, moved);

    let new_doc = reorder(&app, &doc.document, &order)?;
    doc.document = new_doc;
    doc.dirty = true;
    doc_info(&doc.document, &doc.path, true)
}

/* ── Save / export ───────────────────────────────────────────────── */

#[tauri::command]
pub async fn save_document(
    state: State<'_, AppState>,
    path: Option<String>,
) -> CmdResult<PathResult> {
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

    let target = match path.filter(|p| !p.trim().is_empty()) {
        Some(p) => ensure_pdf_extension(p),
        None => {
            if doc.path.is_empty() {
                return Err("No destination path".to_string());
            }
            doc.path.clone()
        }
    };

    doc.document
        .save_to_file(&target)
        .map_err(|e| format!("Cannot save PDF: {e}"))?;

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
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&path)
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

/// Build a new document containing `source`'s pages in the given order.
fn reorder(app: &AppHandle, source: &PdfDocument<'static>, order: &[i32]) -> CmdResult<PdfDocument<'static>> {
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
