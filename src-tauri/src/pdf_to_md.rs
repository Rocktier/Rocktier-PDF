// ⚠️ WIP（2026-10-05）：本模块的推断内核与取字符层已完成且**54 个测试全绿**，
// 但**尚未接入 Tauri 命令**，因此对外没有任何入口，编译器会报一批 dead_code。
// 接入命令后删掉下面这行 allow，让编译器重新监督使用情况。
#![allow(dead_code)]

//! PDF → Markdown：pdfium 取字符 + 调度。
//!
//! 结构推断在 [`crate::mdconv`]（纯逻辑、单测覆盖）；这里只负责
//! 从 pdfium 取出带位置的字符、按页喂给它、把结果与统计交回前端。

use std::path::Path;

// 与 pdf.rs 一致：走 prelude（`pdf` 模块本身是私有的，只能经 prelude 导入）
use pdfium_render::prelude::*;

use crate::mdconv::{self, Glyph, Options, Stats, Warning};

/// 单页最多取多少字符 —— 防止 pathology 文档把 IPC 撑爆。
const MAX_CHARS_PER_PAGE: usize = 200_000;

/// 从一页取出带位置的字符。
pub fn page_glyphs(page: &PdfPage) -> Result<Vec<Glyph>, String> {
    let text = page.text().map_err(|e| e.to_string())?;
    let mut out: Vec<Glyph> = Vec::new();

    for ch in text.chars().iter() {
        if out.len() >= MAX_CHARS_PER_PAGE {
            break;
        }
        // 空白字符在 mdconv 里会按几何间距重建，这里跳过以免重复插入
        let Some(s) = ch.unicode_string() else { continue };
        if s.trim().is_empty() {
            continue;
        }
        // loose_bounds 比 tight 宽松：能覆盖到字形的完整墨迹范围，
        // 对判断「词间距」更可靠（tight 会因字体侧边距偏窄）。
        let Ok(rect) = ch.loose_bounds() else { continue };
        out.push(Glyph {
            text: s,
            size: ch.scaled_font_size().value,
            bold: matches!(ch.font_weight(), Some(PdfFontWeight::Weight700Bold))
                || matches!(ch.font_weight(), Some(PdfFontWeight::Weight800))
                || matches!(ch.font_weight(), Some(PdfFontWeight::Weight900)),
            italic: ch.font_is_italic(),
            x0: rect.left().value,
            x1: rect.right().value,
            y0: rect.bottom().value,
            y1: rect.top().value,
        });
    }
    Ok(out)
}

/// 整篇文档转 markdown。
///
/// ## 多栏排版说明
///
/// v1 **不做多栏检测**：左右分栏的页面会把左栏与右栏的行混在同一 y 上，
/// 聚类时可能把两栏的同一行拼成一行。已知的处理方式是按页把字符交给
/// `mdconv`，由其基线聚类决定；若结果里出现明显交错，v2 再引栏检测
/// （按 x 聚类后分栏内独立成流）。这里如实记在 [`Warning`] 里由前端提示。
pub fn document_to_markdown(
    doc: &PdfDocument,
    opts: Options,
) -> Result<(String, Stats, Vec<Warning>), String> {
    let page_count = doc.pages().len();
    if page_count <= 0 {
        return Ok((String::new(), Stats::default(), vec![Warning::EmptyDocument]));
    }

    // 正文尺寸必须**全篇统一估计** —— 逐页估计会让每页的 h1 都不同级
    let mut all_glyphs: Vec<Glyph> = Vec::new();
    let mut per_page: Vec<usize> = Vec::with_capacity(page_count as usize);
    let mut scanned_pages: Vec<usize> = Vec::new();

    for i in 0..page_count {
        let page = doc.pages().get(i).map_err(|e| e.to_string())?;
        let g = page_glyphs(&page)?;
        if g.len() < 8 {
            // 几乎取不到文字 —— 很可能是扫描件（需要 OCR，路线图 R4 的第二段接力）
            scanned_pages.push(i as usize + 1);
        }
        per_page.push(g.len());
        all_glyphs.extend(g);
    }

    if all_glyphs.is_empty() {
        return Ok((
            String::new(),
            Stats { pages: page_count as usize, ..Default::default() },
            vec![Warning::EmptyDocument],
        ));
    }

    // **逐页**转行再拼接。
    //
    // 曾试过「在页尾插一个哨兵字符，靠它断开上下页」—— 不成立：
    // `glyphs_to_lines` 开头会按 y 坐标重排，哨兵无论放哪都会被排到全篇末尾，
    // 起不到分隔作用。逐页处理是唯一可靠做法。
    let mut lines: Vec<mdconv::Line> = Vec::new();
    let mut idx = 0usize;
    for (p, &n) in per_page.iter().enumerate() {
        let page_lines = mdconv::glyphs_to_lines(&all_glyphs[idx..idx + n]);
        idx += n;
        let _ = p;
        lines.extend(page_lines);
    }

    let (blocks, mut stats, mut warnings) = mdconv::lines_to_blocks(&lines, opts);
    stats.pages = page_count as usize;
    stats.lines = lines.len();

    for p in scanned_pages {
        warnings.push(Warning::LooksScanned { page: p });
    }

    let md = mdconv::blocks_to_markdown(&blocks);
    Ok((md, stats, warnings))
}

/// 写出 `.md` 文件，返回写出的字节数。
pub fn write_markdown(path: &Path, markdown: &str) -> Result<u64, String> {
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir).map_err(|e| format!("Cannot create folder: {e}"))?;
        }
    }
    std::fs::write(path, markdown.as_bytes()).map_err(|e| format!("Cannot write Markdown: {e}"))?;
    Ok(markdown.as_bytes().len() as u64)
}

/// 供前端消费的转换结果（serde::Serialize 在 commands.rs 里加，这里保持纯数据）。
pub struct ConversionResult {
    pub markdown: String,
    pub stats: Stats,
    pub warnings: Vec<Warning>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(text: &str, size: f32, x: f32, y: f32) -> Glyph {
        Glyph {
            text: text.to_string(),
            size,
            bold: false,
            italic: false,
            x0: x,
            x1: x + size * 0.5,
            y0: y,
            y1: y + size * 0.7,
        }
    }

    #[test]
    fn 写出_创建目录并返回字节数() {
        let dir = std::env::temp_dir().join("rt_mdconv_test/nested");
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
        let f = dir.join("out.md");
        let n = write_markdown(&f, "# hi\n").unwrap();
        assert_eq!(n, 5, "5 字节");
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "# hi\n");
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }
}
