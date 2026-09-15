//! Pdfium engine: startup, discovery, rendering and document inspection.
//!
//! Pdfium is loaded once per process — `pdfium-render` stores its bindings in a
//! global slot and refuses a second bind — so everything goes through
//! [`init_pdfium`], which memoises the result.

use std::io::Cursor;
use std::path::PathBuf;
use std::sync::OnceLock;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};
use tauri::Manager;

/// Upper bound on a rendered bitmap's width. A 6000px wide RGBA page is ~140MB,
/// which is already beyond what any sane viewer needs.
const MAX_RENDER_WIDTH: Pixels = 6000;
const MIN_RENDER_WIDTH: Pixels = 16;

static PDFIUM: OnceLock<Pdfium> = OnceLock::new();

/* ── Wire types ──────────────────────────────────────────────────── */

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub index: i32,
    /// Page width in PDF points (1/72"), rotation excluded.
    pub width: f32,
    /// Page height in PDF points, rotation excluded.
    pub height: f32,
    /// Intrinsic rotation: 0 | 90 | 180 | 270.
    pub rotation: i32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DocumentInfo {
    pub path: String,
    pub name: String,
    pub page_count: i32,
    pub file_size: u64,
    pub dirty: bool,
    pub pages: Vec<PageInfo>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedPage {
    pub index: i32,
    /// `data:image/png;base64,...`. Never crosses the network.
    pub data_url: String,
    pub width: i32,
    pub height: i32,
}

#[derive(Serialize)]
pub struct PathResult {
    pub path: String,
    pub size: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SplitMode {
    EveryPage,
    EveryN { n: i32 },
    Ranges { ranges: String },
}

/* ── Library discovery ───────────────────────────────────────────── */

/// Candidate directories that might hold the Pdfium dynamic library, in
/// priority order. Mirrors the three-tier lookup used by Rocktier PDF Squeeze
/// for Ghostscript: bundled resource → dev checkout → system.
fn library_dirs(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();

    if let Ok(dir) = app.path().resource_dir() {
        dirs.push(dir.join("pdfium-runtime"));
        dirs.push(dir);
    }

    // Guaranteed-correct path during `tauri dev`, resolved at compile time.
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    dirs.push(dev.join("pdfium-runtime"));
    dirs.push(dev);

    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("src-tauri").join("resources").join("pdfium-runtime"));
    }

    if let Ok(dir) = std::env::var("ROCKTIER_PDFIUM_DIR") {
        dirs.insert(0, PathBuf::from(dir));
    }

    dirs
}

/// Load Pdfium, or return the already-loaded instance.
pub fn init_pdfium(app: &tauri::AppHandle) -> Result<&'static Pdfium, String> {
    if let Some(existing) = PDFIUM.get() {
        return Ok(existing);
    }
    // `OnceLock::get_or_try_init` is still unstable, so do it by hand. If two
    // threads race, `get_or_init` keeps whichever landed first and we drop ours.
    let loaded = bind_pdfium(&library_dirs(app))?;
    Ok(PDFIUM.get_or_init(|| loaded))
}

/// Split out from [`init_pdfium`] so tests can bind without an `AppHandle`.
pub fn bind_pdfium(dirs: &[PathBuf]) -> Result<Pdfium, String> {
    for dir in dirs {
        let candidate = Pdfium::pdfium_platform_library_name_at_path(&dir);
        if !candidate.exists() {
            continue;
        }
        match Pdfium::bind_to_library(&candidate) {
            Ok(bindings) => {
                log::info!("Loaded Pdfium from {}", candidate.display());
                return Ok(Pdfium::new(bindings));
            }
            Err(e) => {
                log::warn!("Found {} but could not bind: {e}", candidate.display());
            }
        }
    }

    Pdfium::bind_to_system_library()
        .map(Pdfium::new)
        .map_err(|e| format!("{e}. Run `npm run fetch:pdfium` to install the bundled engine."))
}

/* ── Document inspection ─────────────────────────────────────────── */

pub fn page_infos(doc: &PdfDocument) -> Result<Vec<PageInfo>, String> {
    let pages = doc.pages();
    let count = pages.len();
    let mut out = Vec::with_capacity(count.max(0) as usize);
    for index in 0..count {
        let page = pages.get(index).map_err(|e| e.to_string())?;
        let rotation = page.rotation().map(degrees_of).unwrap_or(0);
        out.push(PageInfo {
            index,
            width: page.width().value,
            height: page.height().value,
            rotation,
        });
    }
    Ok(out)
}

pub fn doc_info(doc: &PdfDocument, path: &str, dirty: bool) -> Result<DocumentInfo, String> {
    let name = PathBuf::from(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Untitled.pdf".to_string());

    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    Ok(DocumentInfo {
        path: path.to_string(),
        name,
        page_count: doc.pages().len(),
        file_size,
        dirty,
        pages: page_infos(doc)?,
    })
}

/* ── Rendering ───────────────────────────────────────────────────── */

pub fn render_page(doc: &PdfDocument, index: i32, target_width: Pixels) -> Result<RenderedPage, String> {
    let width = target_width.clamp(MIN_RENDER_WIDTH, MAX_RENDER_WIDTH);

    let page = doc.pages().get(index).map_err(|e| e.to_string())?;

    let config = PdfRenderConfig::new().set_target_width(width);
    let bitmap = page.render_with_config(&config).map_err(|e| e.to_string())?;
    let image = bitmap.as_image().map_err(|e| e.to_string())?;

    let mut buf: Vec<u8> = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    Ok(RenderedPage {
        index,
        data_url: format!("data:image/png;base64,{}", BASE64.encode(&buf)),
        width: bitmap.width(),
        height: bitmap.height(),
    })
}

/* ── Rotation helpers ────────────────────────────────────────────── */

pub fn degrees_of(rotation: PdfPageRenderRotation) -> i32 {
    match rotation {
        PdfPageRenderRotation::None => 0,
        PdfPageRenderRotation::Degrees90 => 90,
        PdfPageRenderRotation::Degrees180 => 180,
        PdfPageRenderRotation::Degrees270 => 270,
    }
}

pub fn rotation_of(degrees: i32) -> PdfPageRenderRotation {
    match degrees.rem_euclid(360) {
        90 => PdfPageRenderRotation::Degrees90,
        180 => PdfPageRenderRotation::Degrees180,
        270 => PdfPageRenderRotation::Degrees270,
        _ => PdfPageRenderRotation::None,
    }
}

/* ── Page range parsing ──────────────────────────────────────────── */

/// Parse `"1-3, 5, 8-"` into zero-based inclusive `(start, end)` pairs.
/// A trailing `-` runs to the last page. Results are clamped and validated.
pub fn parse_ranges(spec: &str, page_count: i32) -> Result<Vec<(i32, i32)>, String> {
    if page_count <= 0 {
        return Err("document has no pages".to_string());
    }

    let mut out = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let (start, end) = match part.split_once('-') {
            Some((a, b)) => {
                let a = a.trim();
                let b = b.trim();
                let start: i32 = if a.is_empty() { 1 } else { parse_one(a)? };
                let end: i32 = if b.is_empty() {
                    page_count
                } else {
                    parse_one(b)?
                };
                (start, end)
            }
            None => {
                let n = parse_one(part)?;
                (n, n)
            }
        };

        if start < 1 || end < 1 || start > page_count || end > page_count {
            return Err(format!("page {start}-{end} out of range 1-{page_count}"));
        }
        if start > end {
            return Err(format!("page range {start}-{end} is reversed"));
        }

        out.push((start - 1, end - 1));
    }

    if out.is_empty() {
        return Err("no pages selected".to_string());
    }

    Ok(out)
}

fn parse_one(token: &str) -> Result<i32, String> {
    token
        .parse::<i32>()
        .map_err(|_| format!("'{token}' is not a page number"))
}

/* ── Tests ───────────────────────────────────────────────────────── */

#[cfg(test)]
mod tests {
    use super::*;

    /// Bind against the copy `build.rs` staged for bundling.
    fn engine() -> Pdfium {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/pdfium-runtime");
        bind_pdfium(&[dir]).expect("bundled Pdfium must load")
    }

    fn three_page_doc(pdfium: &Pdfium) -> PdfDocument<'_> {
        let mut doc = pdfium.create_new_pdf().expect("create");
        for _ in 0..3 {
            doc.pages_mut()
                .create_page_at_end(PdfPagePaperSize::a4())
                .expect("page");
        }
        doc
    }

    #[test]
    fn renders_a_page_to_png() {
        let pdfium = engine();
        let doc = three_page_doc(&pdfium);

        assert_eq!(doc.pages().len(), 3);

        let page = render_page(&doc, 0, 200).expect("render");
        assert_eq!(page.index, 0);
        assert!(page.data_url.starts_with("data:image/png;base64,"));
        assert!(page.width > 0 && page.height > 0);
        // A4 is portrait, so the rendered bitmap must be taller than wide.
        assert!(page.height > page.width);
    }

    #[test]
    fn save_then_reload_roundtrips() {
        let pdfium = engine();
        let doc = three_page_doc(&pdfium);

        let dir = std::env::temp_dir().join("rocktier-pdf-editor-tests");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("roundtrip.pdf");

        doc.save_to_file(&path).expect("save");
        assert!(path.exists(), "saved file should exist");

        let bytes = std::fs::read(&path).expect("read");
        let reloaded = pdfium
            .load_pdf_from_byte_vec(bytes, None)
            .expect("reload");
        assert_eq!(reloaded.pages().len(), 3);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rotation_is_persisted_on_the_page() {
        let pdfium = engine();
        let doc = three_page_doc(&pdfium);

        let mut page = doc.pages().get(1).expect("page 1");
        assert_eq!(page.rotation().map(degrees_of).unwrap_or(0), 0);
        page.set_rotation(rotation_of(90));

        assert_eq!(
            doc.pages().get(1).unwrap().rotation().map(degrees_of).unwrap_or(-1),
            90
        );

        // Rotating by another 90 must accumulate, not overwrite.
        let mut page = doc.pages().get(1).expect("page 1");
        let current = page.rotation().map(degrees_of).unwrap_or(0);
        page.set_rotation(rotation_of(current + 90));
        assert_eq!(
            doc.pages().get(1).unwrap().rotation().map(degrees_of).unwrap_or(-1),
            180
        );
    }

    #[test]
    fn deleting_and_copying_pages_works() {
        let pdfium = engine();
        let doc = three_page_doc(&pdfium);

        doc.pages().get(0).expect("page").delete().expect("delete");
        assert_eq!(doc.pages().len(), 2);

        // Rebuild in reverse order into a fresh document.
        let mut rebuilt = pdfium.create_new_pdf().expect("new");
        for index in [1, 0] {
            let at = rebuilt.pages().len();
            rebuilt
                .pages_mut()
                .copy_page_from_document(&doc, index, at)
                .expect("copy");
        }
        assert_eq!(rebuilt.pages().len(), 2);
    }

    #[test]
    fn merging_appends_pages() {
        let pdfium = engine();
        let mut dest = three_page_doc(&pdfium);
        let source = three_page_doc(&pdfium);

        dest.pages_mut().append(&source).expect("append");
        assert_eq!(dest.pages().len(), 6);
    }

    #[test]
    fn page_ranges_parse() {
        assert_eq!(parse_ranges("1", 5).unwrap(), vec![(0, 0)]);
        assert_eq!(parse_ranges("1-3", 5).unwrap(), vec![(0, 2)]);
        assert_eq!(parse_ranges("2, 4", 5).unwrap(), vec![(1, 1), (3, 3)]);
        assert_eq!(parse_ranges("4-", 5).unwrap(), vec![(3, 4)]);
        assert!(parse_ranges("9", 5).is_err(), "out of range must fail");
        assert!(parse_ranges("3-1", 5).is_err(), "reversed must fail");
        assert!(parse_ranges("", 5).is_err(), "empty must fail");
    }

    #[test]
    fn rotation_wraps_around() {
        assert_eq!(rotation_of(0), PdfPageRenderRotation::None);
        assert_eq!(rotation_of(360), PdfPageRenderRotation::None);
        assert_eq!(rotation_of(450), PdfPageRenderRotation::Degrees90);
        assert_eq!(rotation_of(-90), PdfPageRenderRotation::Degrees270);
    }
}
