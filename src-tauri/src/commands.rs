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
    rotation_of, DocumentInfo, MarkupKind, MarkupRect, PathResult, RenderedPage, SearchHit,
    SplitMode, StampKind,
};
use crate::state::{AppState, OpenDoc};

type CmdResult<T> = Result<T, String>;

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
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

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
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

    let c = (
        *color.first().unwrap_or(&255),
        *color.get(1).unwrap_or(&235),
        *color.get(2).unwrap_or(&59),
    );

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
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

    let c = (
        *color.first().unwrap_or(&255),
        *color.get(1).unwrap_or(&200),
        *color.get(2).unwrap_or(&0),
    );

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
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    let doc = guard.as_mut().ok_or_else(|| "No document is open".to_string())?;

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
    let pdfium = init_pdfium(&app)?;
    crate::pdf::images_to_pdf(pdfium, &paths, &output_path)
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
