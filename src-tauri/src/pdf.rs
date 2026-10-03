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
    /// 恒为 `false`：本产品放置的签名是**视觉图片图章**，不是密码学数字签名
    /// （P0-15）。字段随文档状态上报给前端，供界面如实声明，绝不能在签名功能
    /// 真正做数字签名之前翻转成 `true`。
    pub has_cryptographic_signature: bool,
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
        // Tauri preserves the declared relative path (`resources/pdfium-runtime`)
        // inside the bundle, so the library lands one level deeper.
        dirs.push(dir.join("resources").join("pdfium-runtime"));
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

/// The single Pdfium instance for the whole test binary.
///
/// pdfium can only be initialised once per process — a second bind fails with
/// `PdfiumLibraryBindingsAlreadyInitialized`. Tests run in parallel threads of
/// one process, so every test that used to call `bind_pdfium` itself was racing
/// for the one slot: whichever won, the rest failed. Memoising here (under a
/// lock, so the winner is decided before the binding starts) makes the suite
/// deterministic regardless of thread count or ordering.
#[cfg(test)]
pub fn test_pdfium() -> &'static Pdfium {
    static INIT: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = INIT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(existing) = PDFIUM.get() {
        return existing;
    }
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/pdfium-runtime");
    let loaded = bind_pdfium(&[dir]).expect("bundled Pdfium must load");
    PDFIUM.get_or_init(|| loaded)
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
        has_cryptographic_signature: false,
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

/* ── Text extraction & search ────────────────────────────────────── */

/// A single search hit, in PDF points (origin bottom-left, rotation excluded).
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub page: i32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// The full Unicode text of a page, in reading order.
pub fn page_text(doc: &PdfDocument, index: i32) -> Result<String, String> {
    let page = doc.pages().get(index).map_err(|e| e.to_string())?;
    let text = page.text().map_err(|e| e.to_string())?;
    Ok(text.all())
}

/// Case-insensitive (by default) search across every page, returning the
/// bounding box of each hit. Capped so a pathological document cannot flood
/// the IPC channel.
pub fn search_document(
    doc: &PdfDocument,
    needle: &str,
    match_case: bool,
    whole_word: bool,
) -> Result<Vec<SearchHit>, String> {
    const MAX_HITS: usize = 5000;

    let needle = needle.trim();
    if needle.is_empty() {
        return Ok(Vec::new());
    }

    let options = PdfSearchOptions::new()
        .match_case(match_case)
        .match_whole_word(whole_word);

    let mut hits: Vec<SearchHit> = Vec::new();
    for index in 0..doc.pages().len() {
        let page = doc.pages().get(index).map_err(|e| e.to_string())?;
        let text = page.text().map_err(|e| e.to_string())?;
        let search = text.search(needle, &options).map_err(|e| e.to_string())?;
        while let Some(segments) = search.find_next() {
            for segment in segments.iter() {
                let b = segment.bounds();
                hits.push(SearchHit {
                    page: index,
                    x: b.left().value,
                    y: b.bottom().value,
                    width: b.width().value,
                    height: b.height().value,
                });
            }
            if hits.len() >= MAX_HITS {
                return Ok(hits);
            }
        }
    }
    Ok(hits)
}

/* ── Stamping (page numbers & watermark) ─────────────────────────── */

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum StampKind {
    /// "n / total" at the foot of every page.
    PageNumbers,
    /// A diagonal text watermark in the centre of every page.
    Watermark,
}

/// Adds either page numbers or a text watermark to every page.
///
/// Both are ordinary text objects rather than annotations: they render in every
/// viewer, survive flattening, and print identically everywhere.
pub fn apply_stamp(
    doc: &mut PdfDocument,
    kind: StampKind,
    text: &str,
    font_size: f32,
    margin: f32,
    opacity: f32,
) -> Result<(), String> {
    let total = doc.pages().len();
    if total == 0 {
        return Err("This PDF has no pages".to_string());
    }

    let font = doc.fonts_mut().helvetica();
    let size = font_size.clamp(4.0, 200.0);
    let alpha = (opacity.clamp(0.05, 1.0) * 255.0).round() as u8;

    for index in 0..total {
        let (page_width, page_height) = {
            let page = doc.pages().get(index).map_err(|e| e.to_string())?;
            (page.width().value, page.height().value)
        };

        let label = match kind {
            StampKind::PageNumbers => format!("{} / {}", index + 1, total),
            StampKind::Watermark => text.to_string(),
        };
        if label.trim().is_empty() {
            continue;
        }

        let mut object = PdfPageTextObject::new(doc, &label, font, PdfPoints::new(size))
            .map_err(|e| e.to_string())?;
        object
            .set_fill_color(PdfColor::new(0, 0, 0, alpha))
            .map_err(|e| e.to_string())?;

        // Helvetica averages ~0.52 em per glyph; close enough for centring.
        let approx_width = label.chars().count() as f32 * size * 0.52;
        let mut matrix = PdfMatrix::IDENTITY;

        match kind {
            StampKind::PageNumbers => {
                matrix.set_e((page_width - approx_width) / 2.0);
                matrix.set_f(margin.clamp(4.0, page_height / 3.0));
            }
            StampKind::Watermark => {
                let (sin, cos) = 45.0_f32.to_radians().sin_cos();
                matrix.set_a(cos);
                matrix.set_b(sin);
                matrix.set_c(-sin);
                matrix.set_d(cos);
                matrix.set_e(page_width / 2.0 - (approx_width * cos) / 2.0);
                matrix.set_f(page_height / 2.0);
            }
        }

        object.apply_matrix(matrix).map_err(|e| e.to_string())?;

        doc.pages_mut()
            .get(index)
            .map_err(|e| e.to_string())?
            .objects_mut()
            .add_text_object(object)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/* ── Annotations / markup ────────────────────────────────────────── */

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum MarkupKind {
    Highlight,
    Underline,
    Strikeout,
}

/// A rectangle the user dragged, in PDF points (origin bottom-left).
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct MarkupRect {
    pub page: i32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Draws a highlight / underline / strikeout over the given rectangle.
///
/// These are ordinary filled vector objects rather than `/Annots`, so they
/// render and print identically everywhere without depending on annotation
/// appearance streams (which Pdfium does not generate for us).
pub fn add_markup(
    doc: &mut PdfDocument,
    kind: MarkupKind,
    rect: MarkupRect,
    color: (u8, u8, u8),
    opacity: f32,
) -> Result<(), String> {
    let total = doc.pages().len();
    if rect.page < 0 || rect.page >= total {
        return Err("Page out of range".to_string());
    }
    if rect.width <= 0.5 || rect.height <= 0.5 {
        return Err("Selection is too small".to_string());
    }

    let (r, g, b) = color;
    let alpha = (opacity.clamp(0.05, 1.0) * 255.0).round() as u8;
    let fill = PdfColor::new(r, g, b, alpha);

    let (x1, y1, x2, y2) = match kind {
        MarkupKind::Highlight => (rect.x, rect.y, rect.x + rect.width, rect.y + rect.height),
        MarkupKind::Underline => {
            let t = (rect.height * 0.08).clamp(1.0, 3.0);
            (rect.x, rect.y, rect.x + rect.width, rect.y + t)
        }
        MarkupKind::Strikeout => {
            let t = (rect.height * 0.08).clamp(1.0, 3.0);
            let mid = rect.y + rect.height / 2.0;
            (rect.x, mid, rect.x + rect.width, mid + t)
        }
    };

    let mut path = PdfPagePathObject::new(
        doc,
        PdfPoints::new(x1),
        PdfPoints::new(y1),
        None,
        None,
        Some(fill),
    )
    .map_err(|e| e.to_string())?;
    path.rect_to(PdfPoints::new(x2), PdfPoints::new(y2))
        .map_err(|e| e.to_string())?;

    doc.pages_mut()
        .get(rect.page)
        .map_err(|e| e.to_string())?
        .objects_mut()
        .add_path_object(path)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/* ── Redaction ───────────────────────────────────────────────────── */

/// 一个待脱敏矩形，PDF 点坐标（原点左下，未旋转空间）。
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct RedactRegion {
    pub page_index: i32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// 一次脱敏的统计结果。
///
/// 告警用结构化字段返回而不是英文串：错误走状态栏原样显示没问题，
/// 但「有对象被整块多删」「疑似有字没删干净」是给用户看的正常输出，
/// 必须经前端的 i18n 渲染（en/zh 各自成句）。
#[derive(Debug, Default, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactSummary {
    /// 删除的文本对象数。
    pub removed: usize,
    /// 跨出区域边界、被整对象删除的文本对象数（宁多勿漏，如实上报）。
    pub crossed: usize,
    /// 应用后重扫仍在区域内发现文本对象的区域数（校验告警，不阻塞）。
    pub residual_regions: usize,
}

/// 真脱敏：**删除**与区域相交的文本对象（含其字符内容），再在区域画不透明黑色矩形。
///
/// 与高亮/黑矩形盖住的区别在底层：文本对象从页面内容流里移除，保存后任何
/// 阅读器在该区域都选不中、复制不出原文字。这是**写后即不可逆**的操作
/// （撤销栈只回滚本次应用内的字节快照），确认文案必须写明。
///
/// 「宁多勿漏」：文本对象只要与区域相交就整对象删除——一个跨行/跨区域的
/// 文本对象被切一半会留下另一半原文，比多删几行危险得多。跨出边界的对象
/// 计入 `crossed` 如实上报。
///
/// v1 已知边界（有意不做）：只处理页面顶层内容流的文本对象——
/// 图片内文字（扫描件）、Form XObject 里嵌套的文本、批注（/Annots）都不在
/// 删除范围。前两类靠 `residual_regions` 校验告警兜底提示。
pub fn redact_regions(
    doc: &mut PdfDocument,
    regions: &[RedactRegion],
) -> Result<RedactSummary, String> {
    if regions.is_empty() {
        return Err("No redaction regions were drawn".to_string());
    }

    let total = doc.pages().len();
    for region in regions {
        if region.page_index < 0 || region.page_index >= total {
            return Err(format!("Page {} is out of range", region.page_index + 1));
        }
        if region.width <= 0.5 || region.height <= 0.5 {
            return Err("Redaction region is too small".to_string());
        }
    }

    let mut summary = RedactSummary::default();

    for region in regions {
        let area = PdfRect::new_from_values(
            region.y,
            region.x,
            region.y + region.height,
            region.x + region.width,
        );

        let mut page = doc
            .pages_mut()
            .get(region.page_index)
            .map_err(|e| e.to_string())?;
        let objects = page.objects_mut();

        // 倒序删除：正序删一个后面的索引整体前移，会漏掉或删错对象。
        for index in (0..objects.len()).rev() {
            // hit: Some(是否完全在区域内)。先取信息再丢弃对象包装，
            // 让不可变借用结束在 remove 之前。
            let hit = objects.get(index).ok().and_then(|object| {
                if object.object_type() != PdfPageObjectType::Text {
                    return None;
                }
                let rect = object.bounds().ok()?.to_rect();
                if !rect.does_overlap(&area) {
                    return None;
                }
                Some(rect.is_inside(&area))
            });
            let Some(entirely_inside) = hit else {
                continue;
            };
            if !entirely_inside {
                summary.crossed += 1;
            }
            if objects.remove_object_at_index(index).is_ok() {
                summary.removed += 1;
            }
        }

        // 不透明黑色矩形盖住区域。后加的对象画在最上层。
        objects
            .create_path_object_rect(area, None, None, Some(PdfColor::new(0, 0, 0, 255)))
            .map_err(|e| e.to_string())?;

        // 校验：重扫该区域，若仍有文本对象（例如 Form XObject 里嵌套的
        // 文本不在顶层内容流，上一轮删不到），计一条告警但继续。
        let residual = (0..objects.len()).any(|index| {
            objects
                .get(index)
                .ok()
                .filter(|object| object.object_type() == PdfPageObjectType::Text)
                .and_then(|object| object.bounds().ok())
                .is_some_and(|bounds| bounds.to_rect().does_overlap(&area))
        });
        if residual {
            summary.residual_regions += 1;
        }
    }

    Ok(summary)
}

/// Adds a sticky-note (Text) annotation anchored at the given point.
pub fn add_note(
    doc: &mut PdfDocument,
    page: i32,
    x: f32,
    y: f32,
    text: &str,
    color: (u8, u8, u8),
) -> Result<(), String> {
    let total = doc.pages().len();
    if page < 0 || page >= total {
        return Err("Page out of range".to_string());
    }
    if text.trim().is_empty() {
        return Err("Note text is empty".to_string());
    }

    let mut page_obj = doc.pages_mut().get(page).map_err(|e| e.to_string())?;
    let mut note = page_obj
        .annotations_mut()
        .create_text_annotation(text)
        .map_err(|e| e.to_string())?;

    note.set_position(PdfPoints::new(x), PdfPoints::new(y))
        .map_err(|e| e.to_string())?;
    note.set_width(PdfPoints::new(22.0)).map_err(|e| e.to_string())?;
    note.set_height(PdfPoints::new(22.0)).map_err(|e| e.to_string())?;
    note.set_stroke_color(PdfColor::new(color.0, color.1, color.2, 255))
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Places a signature image (typically a transparent PNG) so its bottom-left
/// corner sits at the given point.
pub fn add_signature(
    doc: &mut PdfDocument,
    page: i32,
    x: f32,
    y: f32,
    width: f32,
    image_path: &str,
) -> Result<(), String> {
    let total = doc.pages().len();
    if page < 0 || page >= total {
        return Err("Page out of range".to_string());
    }

    let image = image::open(image_path).map_err(|e| format!("Cannot read image: {e}"))?;
    let ratio = if image.width() == 0 {
        0.5
    } else {
        image.height() as f32 / image.width() as f32
    };

    let w = width.clamp(20.0, 1000.0);
    let h = w * ratio;

    let mut object = PdfPageImageObject::new_with_size(doc, &image, PdfPoints::new(w), PdfPoints::new(h))
        .map_err(|e| e.to_string())?;

    let mut matrix = PdfMatrix::IDENTITY;
    matrix.set_e(x);
    matrix.set_f(y);
    object.apply_matrix(matrix).map_err(|e| e.to_string())?;

    doc.pages_mut()
        .get(page)
        .map_err(|e| e.to_string())?
        .objects_mut()
        .add_image_object(object)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/* ── AcroForm fields ─────────────────────────────────────────────── */

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FormFieldInfo {
    pub page: i32,
    pub name: String,
    /// "text" | "checkbox" | "radio" | "combo" | "list" | "button" | "signature" | "unknown"
    pub kind: String,
    pub value: String,
}

/// Every fillable widget in the document's AcroForm, in page order.
pub fn list_form_fields(doc: &PdfDocument) -> Result<Vec<FormFieldInfo>, String> {
    let mut out = Vec::new();

    for page_index in 0..doc.pages().len() {
        let page = doc.pages().get(page_index).map_err(|e| e.to_string())?;
        for annotation in page.annotations().iter() {
            let Some(field) = annotation.as_form_field() else {
                continue;
            };
            let name = field.name().unwrap_or_default();
            if name.is_empty() {
                continue;
            }

            let (kind, value) = match field {
                PdfFormField::Text(f) => ("text", f.value().unwrap_or_default()),
                PdfFormField::Checkbox(f) => (
                    "checkbox",
                    if f.is_checked().unwrap_or(false) { "true" } else { "false" }.to_string(),
                ),
                PdfFormField::RadioButton(f) => (
                    "radio",
                    if f.is_checked().unwrap_or(false) { "true" } else { "false" }.to_string(),
                ),
                PdfFormField::ComboBox(f) => ("combo", f.value().unwrap_or_default()),
                PdfFormField::ListBox(f) => ("list", f.value().unwrap_or_default()),
                PdfFormField::PushButton(_) => ("button", String::new()),
                PdfFormField::Signature(_) => ("signature", String::new()),
                _ => ("unknown", String::new()),
            };

            out.push(FormFieldInfo {
                page: page_index,
                name,
                kind: kind.to_string(),
                value,
            });
        }
    }

    Ok(out)
}

/// Applies `(field name, value)` pairs to the document's form widgets.
///
/// Text fields receive the raw string; checkboxes and radio buttons treat
/// "true"/"1" as checked.
pub fn set_form_values(doc: &mut PdfDocument, values: &[(String, String)]) -> Result<(), String> {
    if values.is_empty() {
        return Ok(());
    }

    for page_index in 0..doc.pages().len() {
        let mut page = doc.pages_mut().get(page_index).map_err(|e| e.to_string())?;
        let annotations = page.annotations_mut();
        let count = annotations.len();

        for index in 0..count {
            let mut annotation = annotations.get(index).map_err(|e| e.to_string())?;
            let Some(field) = annotation.as_form_field_mut() else {
                continue;
            };
            let name = field.name().unwrap_or_default();
            let Some((_, value)) = values.iter().find(|(n, _)| *n == name) else {
                continue;
            };

            match field {
                PdfFormField::Text(f) => {
                    f.set_value(value).map_err(|e| e.to_string())?;
                }
                PdfFormField::Checkbox(f) => {
                    let on = value.eq_ignore_ascii_case("true") || value == "1";
                    f.set_checked(on).map_err(|e| e.to_string())?;
                }
                    // pdfium can only *select* a radio button; there is no call
                    // that clears one. Deselecting is therefore done at the file
                    // level on the way out — see `formclear.rs` for why the
                    // documented raw-handle route is closed (pdfium-render keeps
                    // `PdfFormFieldPrivate` `pub(crate)` on purpose) and what runs
                    // instead. Saving records the request; this branch only ever
                    // turns something on.
                // A guard rather than an inner `if`: clippy reads the latter as
                // collapsible, and the condition genuinely belongs to the pattern.
                PdfFormField::RadioButton(f)
                    if value.eq_ignore_ascii_case("true") || value == "1" =>
                {
                    f.set_checked().map_err(|e| e.to_string())?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

/* ── Import / export images ──────────────────────────────────────── */

/// Renders the given pages to individual PNG files inside `output_dir`.
pub fn export_pages_as_png(
    doc: &PdfDocument,
    indices: &[i32],
    output_dir: &str,
    base_name: &str,
    width: Pixels,
) -> Result<Vec<PathResult>, String> {
    if indices.is_empty() {
        return Err("No pages selected".to_string());
    }
    std::fs::create_dir_all(output_dir).map_err(|e| format!("Cannot create folder: {e}"))?;

    let mut results = Vec::with_capacity(indices.len());
    for &index in indices {
        let page = render_page(doc, index, width)?;
        let encoded = page.data_url.split(',').nth(1).unwrap_or("");
        let bytes = BASE64.decode(encoded).map_err(|e| e.to_string())?;
        let file = PathBuf::from(output_dir).join(format!("{base_name}_{:04}.png", index + 1));
        std::fs::write(&file, &bytes).map_err(|e| format!("Cannot write image: {e}"))?;
        results.push(PathResult {
            path: file.to_string_lossy().to_string(),
            size: bytes.len() as u64,
        });
    }
    Ok(results)
}

/// Builds a new PDF with one page per image (JPEG or PNG), each page sized to
/// the image at 96 dpi so it lands at its natural printed size.
pub fn images_to_pdf(
    pdfium: &Pdfium,
    paths: &[String],
    output_path: &str,
) -> Result<PathResult, String> {
    if paths.is_empty() {
        return Err("Pick at least one image".to_string());
    }

    let mut doc = pdfium.create_new_pdf().map_err(|e| e.to_string())?;

    for path in paths {
        let image = image::open(path).map_err(|e| format!("Cannot read {path}: {e}"))?;
        let width_pt = PdfPoints::new(image.width() as f32 * 72.0 / 96.0);
        let height_pt = PdfPoints::new(image.height() as f32 * 72.0 / 96.0);

        doc.pages_mut()
            .create_page_at_end(PdfPagePaperSize::from_points(width_pt, height_pt))
            .map_err(|e| e.to_string())?;

        let object = PdfPageImageObject::new_with_size(&doc, &image, width_pt, height_pt)
            .map_err(|e| e.to_string())?;

        let last = doc.pages().len() - 1;
        doc.pages_mut()
            .get(last)
            .map_err(|e| e.to_string())?
            .objects_mut()
            .add_image_object(object)
            .map_err(|e| e.to_string())?;
    }

    let out = if output_path.to_lowercase().ends_with(".pdf") {
        output_path.to_string()
    } else {
        format!("{output_path}.pdf")
    };
    doc.save_to_file(&out).map_err(|e| e.to_string())?;
    let size = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    Ok(PathResult { path: out, size })
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
    fn engine() -> &'static Pdfium {
        test_pdfium()
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
        let doc = three_page_doc(pdfium);

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
        let doc = three_page_doc(pdfium);

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
        let doc = three_page_doc(pdfium);

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
        let doc = three_page_doc(pdfium);

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
        let mut dest = three_page_doc(pdfium);
        let source = three_page_doc(pdfium);

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

    /* ── Redaction ───────────────────────────────────────────────── */

    /// 用 lopdf 在内存构造一份含已知文本的最小 PDF：
    /// 24pt 的 "SECRET-4242 confidential"（基线 y=700，x 从 72 起）和
    /// 14pt 的 "public line"（基线 y=600）。验收要求的核心夹具——
    /// 构造、脱敏、验证全链路都不依赖磁盘上的任何现成文件。
    fn build_text_pdf() -> Vec<u8> {
        use lopdf::{dictionary, Document, Object, Stream};

        let mut doc = Document::with_version("1.5");

        let content =
            b"BT /F1 24 Tf 72 700 Td (SECRET-4242 confidential) Tj ET\nBT /F1 14 Tf 72 600 Td (public line) Tj ET";
        let content_id = doc.add_object(Stream::new(
            dictionary! { "Length" => content.len() as i64 },
            content.to_vec(),
        ));

        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });

        let pages_id = doc.add_object(dictionary! {
            "Type" => "Pages",
            "Count" => 1i64,
            "Kids" => Vec::<Object>::new(),
        });

        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => Object::Reference(pages_id),
            "MediaBox" => vec![0i64.into(), 0i64.into(), 612i64.into(), 792i64.into()],
            "Contents" => Object::Reference(content_id),
            "Resources" => dictionary! {
                "Font" => dictionary! { "F1" => Object::Reference(font_id) },
            },
        });

        let pages = doc.objects.get_mut(&pages_id).expect("pages object");
        pages
            .as_dict_mut()
            .expect("pages dict")
            .set("Kids", vec![Object::Reference(page_id)]);

        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => Object::Reference(pages_id),
        });
        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut bytes: Vec<u8> = Vec::new();
        doc.save_to(&mut bytes).expect("serialize");
        bytes
    }

    fn text_pdf_doc() -> PdfDocument<'static> {
        test_pdfium()
            .load_pdf_from_byte_vec(build_text_pdf(), None)
            .expect("lopdf-built PDF must load into pdfium")
    }

    /// 页面上的文本对象是否仍包含 `needle`（绕开 search 的分词，直接逐对象读）。
    fn page_still_contains(doc: &PdfDocument, index: i32, needle: &str) -> bool {
        let page = doc.pages().get(index).expect("page");
        let count = page.objects().len();
        (0..count).any(|i| {
            page.objects()
                .get(i)
                .ok()
                .and_then(|object| object.as_text_object().map(|t| t.text()))
                .is_some_and(|text| text.contains(needle))
        })
    }

    /// 区域内是否存在一个黑色不透明填充的路径对象（即脱敏盖板）。
    fn has_black_cover(doc: &PdfDocument, index: i32, region: RedactRegion) -> bool {
        let page = doc.pages().get(index).expect("page");
        let area = PdfRect::new_from_values(
            region.y,
            region.x,
            region.y + region.height,
            region.x + region.width,
        );
        let count = page.objects().len();
        (0..count).any(|i| {
            page.objects()
                .get(i)
                .ok()
                .filter(|object| object.object_type() == PdfPageObjectType::Path)
                .and_then(|object| {
                    let fill = object.fill_color().ok()?;
                    let inside = object.bounds().ok()?.to_rect().is_inside(&area);
                    Some(
                        inside
                            && fill.red() == 0
                            && fill.green() == 0
                            && fill.blue() == 0
                            && fill.alpha() == 255,
                    )
                })
                .unwrap_or(false)
        })
    }

    /// 核心验收：区域内文字被真删（page_text / search / 逐对象三路确认），
    /// 黑色盖板存在，且统计数正确。
    #[test]
    fn redact_deletes_text_and_draws_opaque_cover() {
        let mut doc = text_pdf_doc();
        assert!(page_text(&doc, 0).unwrap().contains("public line"));

        // "public line" 的对象框完全落在这个区域里。
        let region = RedactRegion {
            page_index: 0,
            x: 60.0,
            y: 585.0,
            width: 220.0,
            height: 40.0,
        };
        let summary = redact_regions(&mut doc, &[region]).expect("redact");

        assert_eq!(summary.removed, 1, "应恰好删掉 1 个文本对象");
        assert_eq!(summary.crossed, 0, "对象完全在区域内，不应报跨界");
        assert_eq!(summary.residual_regions, 0, "区域内不应残留文本对象");

        // 三路验证文字真的没了：页面全文、搜索引擎、逐对象。
        assert!(!page_text(&doc, 0).unwrap().contains("public line"));
        assert!(search_document(&doc, "public", false, false)
            .unwrap()
            .is_empty());
        assert!(!page_still_contains(&doc, 0, "public"));

        // 区域外的文字不受影响。
        assert!(page_text(&doc, 0).unwrap().contains("SECRET-4242"));

        // 黑色不透明盖板存在。
        assert!(has_black_cover(&doc, 0, region), "区域内应有黑色盖板");
    }

    /// 宁多勿漏：文本对象与区域相交但跨出边界 → 整对象删除并如实上报。
    #[test]
    fn redact_removes_whole_object_when_it_crosses_the_region() {
        let mut doc = text_pdf_doc();

        // 只罩住 "SECRET-4242 confidential" 的左半段；对象框远超区域。
        let region = RedactRegion {
            page_index: 0,
            x: 60.0,
            y: 690.0,
            width: 100.0,
            height: 40.0,
        };
        let summary = redact_regions(&mut doc, &[region]).expect("redact");

        assert_eq!(summary.removed, 1);
        assert_eq!(summary.crossed, 1, "跨界对象必须被计数上报");
        assert_eq!(summary.residual_regions, 0);

        // 整对象删除：区域里没有字了，区域外那半段也不在了——宁可多删。
        assert!(!page_text(&doc, 0).unwrap().contains("SECRET-4242"));
        assert!(!page_text(&doc, 0).unwrap().contains("confidential"));
        assert!(search_document(&doc, "confidential", false, false)
            .unwrap()
            .is_empty());
        assert!(page_text(&doc, 0).unwrap().contains("public line"));
    }

    /// 多区域、多页：一次调用处理多个区域，只动各自页面。
    #[test]
    fn redact_handles_multiple_regions() {
        let mut doc = text_pdf_doc();
        doc.pages_mut()
            .create_page_at_end(PdfPagePaperSize::a4())
            .expect("second page");

        let regions = vec![
            RedactRegion {
                page_index: 0,
                x: 60.0,
                y: 690.0,
                width: 100.0,
                height: 40.0,
            },
            RedactRegion {
                page_index: 0,
                x: 60.0,
                y: 585.0,
                width: 220.0,
                height: 40.0,
            },
            RedactRegion {
                page_index: 1,
                x: 60.0,
                y: 585.0,
                width: 220.0,
                height: 40.0,
            },
        ];
        let summary = redact_regions(&mut doc, &regions).expect("redact");
        assert_eq!(summary.removed, 2, "第 2 页没有文本对象可删");

        let text = page_text(&doc, 0).unwrap();
        assert!(!text.contains("SECRET"));
        assert!(!text.contains("public"));
    }

    /// 垃圾输入必须报错：页码越界、区域过小、空区域列表。
    #[test]
    fn redact_rejects_bad_regions() {
        let mut doc = text_pdf_doc();

        let out_of_range = RedactRegion {
            page_index: 5,
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        assert!(redact_regions(&mut doc, &[out_of_range]).is_err());

        let too_small = RedactRegion {
            page_index: 0,
            x: 10.0,
            y: 10.0,
            width: 0.2,
            height: 30.0,
        };
        assert!(redact_regions(&mut doc, &[too_small]).is_err());

        assert!(redact_regions(&mut doc, &[]).is_err());

        // 报错的调用不能动文档。
        assert!(page_text(&doc, 0).unwrap().contains("SECRET-4242"));
    }

    /// 脱敏产物写出后重读：文字层在文件里就是没了，不是只在内存里。
    #[test]
    fn redacted_file_has_no_text_after_roundtrip() {
        let mut doc = text_pdf_doc();
        let region = RedactRegion {
            page_index: 0,
            x: 60.0,
            y: 690.0,
            width: 400.0,
            height: 40.0,
        };
        redact_regions(&mut doc, &[region]).expect("redact");

        let dir = std::env::temp_dir().join("rocktier-pdf-editor-tests");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("redacted.pdf");
        doc.save_to_file(&path).expect("save");

        let pdfium = test_pdfium();
        let reloaded = pdfium
            .load_pdf_from_byte_vec(std::fs::read(&path).expect("read"), None)
            .expect("reload");
        assert!(!page_text(&reloaded, 0).unwrap().contains("SECRET"));
        assert!(page_text(&reloaded, 0).unwrap().contains("public line"));
        assert!(search_document(&reloaded, "SECRET", false, false)
            .unwrap()
            .is_empty());

        let _ = std::fs::remove_file(&path);
    }
}
