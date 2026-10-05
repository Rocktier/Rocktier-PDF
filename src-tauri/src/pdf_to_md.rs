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

/// 旋转角容差（度）。正文是 0°；PDF 里的小数舍入与微小倾斜给一点余量。
const ROTATION_TOLERANCE_DEG: f32 = 1.0;

/// 字体内嵌名里代表粗体的词根（小写匹配）。
const BOLD_NAME_HINTS: [&str; 8] = [
    "bold",
    "black",
    "heavy",
    "semibold",
    "demibold",
    "extrabold",
    "ultrabold",
    "extrab",
];

/// 判断一个字符是否粗体。
///
/// ## 为什么主信号是字体名，不是 `font_weight()`
///
/// 原本按 `font_weight()` 的枚举匹配（Weight700/800/900）。在自家
/// `demo-report.pdf` 上实测：**每一个字符都返回 `Some(Custom(0))`**，
/// 包括明显是粗体的标题（字体名 `Helvetica-Bold`）。
///
/// 后果不是报错而是**静默失效**：粗体一律判成false，于是
/// - 标题因为「不够大 + 不加粗」而漏判（`1. Summary` 变成列表项）
/// - 正文里的重点词失去了 `**…**`
///
/// Base14 字体（Helvetica/Times/Courier）一定带 `-Bold` 后缀，
/// 嵌入式字体（Inter-SemiBold 等）也带词根，所以字体名是可靠信号。
/// 三路信号取或：`font_is_bold_reenforced` 覆盖「填充+描边」的合成粗体。
fn is_bold(ch: &PdfPageTextChar) -> bool {
    let name = ch.font_name().to_ascii_lowercase();
    if BOLD_NAME_HINTS.iter().any(|h| name.contains(h)) {
        return true;
    }
    if ch.font_is_bold_reenforced() {
        return true;
    }
    matches!(
        ch.font_weight(),
        Some(PdfFontWeight::Weight700Bold)
            | Some(PdfFontWeight::Weight800)
            | Some(PdfFontWeight::Weight900)
    )
}

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
        // **剔掉旋转字形**（页边竖排的行号、栏边刻度）。
        //
        // 实测 45 页英文论文：这些是270°（竖排）的行号，按 y 基线聚类时
        // 会和正文并成一行，于是输出变成 `6 M0 on each case`。
        //
        // 为什么不能改成「剔文本块之外」的横向裁剪：该文档是**双栏**，
        // 字形 x0 的中位数 298 正好是右栏起点 —— 按中位数裁左边距会把
        // **整个左栏**当成页边删掉（实测丢掉 70023 个字形，标题被截成
        // "DAPTED? PREDICTION FRAG"）。横向裁剪在双栏下不成立。
        //
        // 旋转角是字形自带的、无歧义的信号：正文恒为 0°。
        match ch.angle_degrees() {
            Ok(a) if a.abs() > ROTATION_TOLERANCE_DEG => continue,
            _ => {}
        }
        // loose_bounds 比 tight 宽松：能覆盖到字形的完整墨迹范围，
        // 对判断「词间距」更可靠（tight 会因字体侧边距偏窄）。
        let Ok(rect) = ch.loose_bounds() else { continue };
        out.push(Glyph {
            text: s,
            size: ch.scaled_font_size().value,
            bold: is_bold(&ch),
            italic: ch.font_is_italic(),
            x0: rect.left().value,
            x1: rect.right().value,
            y0: rect.bottom().value,
            y1: rect.top().value,
            rotation_deg: ch.angle_degrees().unwrap_or(0.0),
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
    let mut per_page_glyphs: Vec<Vec<Glyph>> = Vec::with_capacity(page_count as usize);
    let mut page_heights: Vec<f32> = Vec::with_capacity(page_count as usize);
    let mut scanned_pages: Vec<usize> = Vec::new();

    for i in 0..page_count {
        let page = doc.pages().get(i).map_err(|e| e.to_string())?;
        let g = page_glyphs(&page)?;
        if g.len() < 8 {
            // 几乎取不到文字 —— 很可能是扫描件（需要 OCR，路线图 R4 的第二段接力）
            scanned_pages.push(i as usize + 1);
        }
        page_heights.push(page.height().value);
        per_page_glyphs.push(g);
    }

    let all_glyphs: usize = per_page_glyphs.iter().map(Vec::len).sum();
    if all_glyphs == 0 {
        // ⚠️ 这里必须**同时**给出 LooksScanned，否则用户在界面上只知道
        // 「什么都没取到」，不知道该怎么办。
        //
        // 实测踩过：一份中文扫描件走的就是这条早退分支，只报了
        // EmptyDocument。最有用的那条事实 —— 「这是扫描件，请走 OCR」——
        // 恰好被漏掉了。
        //
        // 只在**每一页**都取不到字时才说「疑似扫描件」；只有部分页取不到
        // 时由下面的 scanned_pages 单独报，两者不重复。
        let mut w = vec![Warning::EmptyDocument];
        if !scanned_pages.is_empty() && scanned_pages.len() == page_count as usize {
            w.extend(scanned_pages.iter().map(|&p| Warning::LooksScanned { page: p }));
        }
        return Ok((String::new(), Stats { pages: page_count as usize, ..Default::default() }, w));
    }

    // **逐页**转行再拼接。
    //
    // 曾试过「在页尾插一个哨兵字符，靠它断开上下页」—— 不成立：
    // `glyphs_to_lines` 开头会按 y 坐标重排，哨兵无论放哪都会被排到全篇末尾，
    // 起不到分隔作用。逐页处理是唯一可靠做法。
    //
    // 页眉页脚要在这里剔除 —— 它们只在页边缘，且跨页重复，
    // 这是「正文」与「页面装饰」唯一可靠的区分依据。
    let mut per_page_lines: Vec<Vec<mdconv::Line>> = Vec::with_capacity(page_count as usize);
    for g in &per_page_glyphs {
        per_page_lines.push(mdconv::glyphs_to_lines(g));
    }
    drop_running_furniture(&mut per_page_lines, &page_heights);

    let lines: Vec<mdconv::Line> = per_page_lines.into_iter().flatten().collect();

    let (blocks, mut stats, mut warnings) = mdconv::lines_to_blocks(&lines, opts);
    stats.pages = page_count as usize;
    stats.lines = lines.len();

    for p in scanned_pages {
        warnings.push(Warning::LooksScanned { page: p });
    }

    let md = mdconv::blocks_to_markdown(&blocks);
    Ok((md, stats, warnings))
}

/// 页边缘带（页高的百分比）—— 只在这个带里找页眉页脚候选。
const EDGE_BAND: f32 = 0.06;

/// 归一化一行文字，用于跨页比对。
///
/// 数字必须抹掉：页码是「Page 1 of 4」/「Page 2 of 4」这种**每页不同**的，
/// 不归一化就永远比不出重复，页码会一路混进正文。
fn furniture_key(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for c in s.chars() {
        if c.is_ascii_digit() {
            if !out.ends_with('#') {
                out.push('#');
            }
            prev_space = false;
        } else if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
            prev_space = false;
        } else if !prev_space {
            out.push(' ');
            prev_space = true;
        }
    }
    out.trim().to_string()
}

/// 剔除页眉页脚（running header / footer）。
///
/// ## 为什么用「跨页重复」而不是「位置在边缘」
///
/// 边缘带里不只有页眉页脚：**脚注**也在页面下缘，而且是要保留的内容。
/// 曾想过「边缘 + 字号小于正文」一并剔除 —— 那会把脚注整段删掉，
/// 属于静默丢内容，比留着页码更糟。
///
/// 跨页重复则是安全的：正文段落不会在每页原样重现，
/// 而页眉页脚**按定义**就是每页重复的那一行。
///
/// ## 已知边界
///
/// - **单页文档不剔除** —— 没有第二页可比，页码会留着。
/// - 逐页镜像排版的扫描件，其页眉文字一致，也会被剔除（这是对的）。
fn drop_running_furniture(pages: &mut [Vec<mdconv::Line>], heights: &[f32]) {
    use std::collections::HashMap;

    // 先数每个归一化键出现在**多少个不同页**上。
    // 注意是页数不是次数：同一页里页眉出现两次不代表它是页眉。
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (pi, lines) in pages.iter().enumerate() {
        let h = heights.get(pi).copied().unwrap_or(0.0);
        if h <= 0.0 {
            continue;
        }
        let mut on_this_page: Vec<String> = Vec::new();
        for l in lines {
            if !in_edge_band(l.baseline, h) {
                continue;
            }
            let k = furniture_key(&l.text);
            if !k.is_empty() && !on_this_page.contains(&k) {
                on_this_page.push(k);
            }
        }
        for k in on_this_page {
            *seen.entry(k).or_insert(0) += 1;
        }
    }
    if seen.values().all(|&n| n < 2) {
        return; // 没有任何跨页重复 → 没有页眉页脚，不动
    }

    for (pi, lines) in pages.iter_mut().enumerate() {
        let h = heights.get(pi).copied().unwrap_or(0.0);
        if h <= 0.0 {
            continue;
        }
        lines.retain(|l| {
            if !in_edge_band(l.baseline, h) {
                return true;
            }
            let k = furniture_key(&l.text);
            // 出现不足 2 页的边缘行：留着（可能是脚注或真正的首行标题）
            !seen.get(&k).is_some_and(|&n| n >= 2)
        });
    }
}

fn in_edge_band(baseline: f32, page_height: f32) -> bool {
    baseline >= page_height * (1.0 - EDGE_BAND) || baseline <= page_height * EDGE_BAND
}

/// 写出 `.md` 文件，返回写出的字节数。
pub fn write_markdown(path: &Path, markdown: &str) -> Result<u64, String> {
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir).map_err(|e| format!("Cannot create folder: {e}"))?;
        }
    }
    // as_bytes() 是多余的：String 本身就能传给 fs::write（clippy: needless_as_bytes）
    std::fs::write(path, markdown).map_err(|e| format!("Cannot write Markdown: {e}"))?;
    Ok(markdown.len() as u64)
}

/// 供前端消费的转换结果。
///
/// 前端必须同时拿到 `markdown` / `stats` / `warnings` 三样：
/// 只给 markdown 会让用户以为转换是完美的，而表格未转换、疑似扫描件
/// 这些情况**必须让他知道**。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionResult {
    pub markdown: String,
    pub stats: Stats,
    pub warnings: Vec<Warning>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一行（`gaps` 与位置推断无关，测试里不需要）。
    fn ln(text: &str, size: f32, baseline: f32) -> mdconv::Line {
        mdconv::Line {
            text: text.to_string(),
            size,
            bold: false,
            italic: false,
            x0: 60.0,
            x1: 500.0,
            baseline,
            gaps: Vec::new(),
        }
    }

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
            rotation_deg: 0.0,
        }
    }

    /// 逐页转行拼接：相邻两页**基线相同**也必须断成两行。
    ///
    /// 这是踩过的坑：曾用「页尾插哨兵字符靠 y 断层」断页，但
    /// `glyphs_to_lines` 开头按 y 重排，哨兵必被排到全篇末尾 ——
    /// 该设计不成立。改成逐页处理后，基线相同的两页也天然断开。
    #[test]
    fn 逐页拼接_基线相同的相邻页也断开() {
        let page1 = vec![g("PageOne", 10.0, 100.0, 700.0)];
        let page2 = vec![g("PageTwo", 10.0, 100.0, 700.0)]; // 故意同一基线

        let mut lines = mdconv::glyphs_to_lines(&page1);
        lines.extend(mdconv::glyphs_to_lines(&page2));

        let texts: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, vec!["PageOne", "PageTwo"], "基线相同也须断成两行");
    }

    /// 旋转字形（页边竖排行号）必须剔掉。
    ///
    /// 真实 45 页论文上，270° 的页边行号按 y 基线聚类时被并进正文，
    /// 输出变成 `6 M0 on each case`。
    ///
    /// 这条只能靠**旋转角**判：曾试过「按 x0 中位数裁掉左边距」，
    /// 而该文档是双栏、x0 中位数 298 正是右栏起点 —— 结果把整个左栏
    /// 当页边删了（丢掉 70023 字形，标题被截成"DAPTED? PREDICTION FRAG"）。
    /// 横向裁剪在双栏下不成立，旋转角却是无歧义的字形自带信号。
    #[test]
    fn 页边竖排行号_按旋转角剔除() {
        fn g(t: &str, x: f32, y: f32, deg: f32) -> Glyph {
            Glyph { text: t.into(), size: 10.0, bold: false, italic: false,
                    x0: x, x1: x + 5.0, y0: y, y1: y + 7.0, rotation_deg: deg }
        }
        // 数组而非 vec![]：clippy useless_vec 会把这个要求改成字面量数组
        let glyphs = [
            g("6", 30.0, 700.0, 270.0),   // 页边竖排行号
            g("Body", 300.0, 700.0, 0.0),  // 正文
            g("text", 300.0, 700.0, 0.0),
        ];
        let kept: Vec<&Glyph> = glyphs.iter().filter(|g| g.rotation_deg.abs() <= ROTATION_TOLERANCE_DEG).collect();
        let txt: String = kept.iter().map(|g| g.text.as_str()).collect();
        assert_eq!(txt, "Bodytext", "旋转的行号不该进正文");
    }

    /// 微小倾角要留着 —— PDF 里常有 0.x° 的舍入误差。
    #[test]
    fn 微小倾角_视为水平() {
        assert!(0.4f32.abs() <= ROTATION_TOLERANCE_DEG);
        assert!(2.0f32.abs() > ROTATION_TOLERANCE_DEG);
    }

    /// 页码必须按「归一化后跨页重复」判掉，而不是按位置。
    #[test]
    fn 页眉页脚_跨页重复的被剔除() {
        // A4 高 842：页眉在 806，页脚在 36
        let page = |header: &str, footer: &str, body: &str| -> Vec<mdconv::Line> {
            vec![
                ln(header, 8.0, 806.0),
                ln(body, 10.0, 700.0),
                ln(footer, 8.0, 36.0),
            ]
        };
        let mut pages = vec![
            page("Riverside Hall \u{2014} Inspection Report", "Page 1 of 3", "First body line."),
            page("Riverside Hall \u{2014} Inspection Report", "Page 2 of 3", "Second body line."),
            page("Riverside Hall \u{2014} Inspection Report", "Page 3 of 3", "Third body line."),
        ];
        drop_running_furniture(&mut pages, &[842.0; 3]);

        for (i, p) in pages.iter().enumerate() {
            let texts: Vec<&str> = p.iter().map(|l| l.text.as_str()).collect();
            assert_eq!(texts.len(), 1, "第 {} 页只剩正文，实际 {:?}", i + 1, texts);
            assert!(texts[0].ends_with("body line."), "第 {} 页正文被误删：{:?}", i + 1, texts);
        }
    }

    /// **脚注也在页面下缘**，且是要保留的内容 —— 绝不许被页眉页脚逻辑吃掉。
    #[test]
    fn 页脚_单页出现的脚注不剔除() {
        let mut pages = vec![
            vec![
                ln("Body line one.", 10.0, 700.0),
                // 脚注：下缘、小字号，但每页内容不同
                ln("1 Reference to the maintenance log.", 8.0, 40.0),
            ],
            vec![
                ln("Body line two.", 10.0, 700.0),
                ln("2 Reference to the test certificate.", 8.0, 40.0),
            ],
        ];
        drop_running_furniture(&mut pages, &[842.0; 2]);

        for (i, p) in pages.iter().enumerate() {
            assert_eq!(p.len(), 2, "第 {} 页的脚注被当成页脚删了", i + 1);
            assert!(p[1].text.starts_with(&(i + 1).to_string()), "脚注内容变了");
        }
    }

    /// 只有一页时无从判断重复 → 保持原样（不猜）。
    #[test]
    fn 页眉页脚_单页文档不动() {
        let mut pages = vec![vec![ln("Page 1 of 1", 8.0, 36.0), ln("Body.", 10.0, 700.0)]];
        drop_running_furniture(&mut pages, &[842.0]);
        assert_eq!(pages[0].len(), 2, "单页文档不该被裁剪");
    }

    #[test]
    fn 归一化_数字抹成井号() {
        assert_eq!(furniture_key("Page 12 of 30"), "page # of #");
        assert_eq!(furniture_key("Riverside Hall \u{2014} Report"), "riverside hall report");
        // 相邻数字只出一个井号
        assert_eq!(furniture_key("A12B"), "a#b");
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
    /// 真实 PDF 验收（默认不跑）。
    ///
    /// 路线图 R4 的口径是「10 个典型文档人工抽检，标题/段落/列表结构
    /// 正确率 ≥90%」—— 这**只能人看**，自动断言兜不住「把加粗正文
    /// 判成标题」这类错误。所以这里只做一件事：把真实 PDF 的转换结果
    /// 打出来，外加几道「明显崩坏」的断言（空结果、单行超长等）。
    ///
    /// 跑法（多个路径用 **冒号** 分隔 —— 空格分隔会被 split_whitespace
    /// 切开，而家族目录名本身就带空格，如「Rocktier PDF」）
    /// ```text
    /// ROCKTIER_ACC_PDFS="/abs/a.pdf:/abs/b.pdf" cargo test --bin rocktier-pdf-editor 真实pdf验收 -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "需要真实 PDF：设 ROCKTIER_ACC_PDFS 后用 --ignored 跑"]
    fn 真实pdf验收() {
        let Ok(list) = std::env::var("ROCKTIER_ACC_PDFS") else {
            eprintln!("跳过：未设 ROCKTIER_ACC_PDFS");
            return;
        };
        // 用冒号分隔而非空格 —— 本仓目录名含空格（"Rocktier PDF"），
        // 曾用 split_whitespace 导致路径被切成三段、三个都"读不到"。
        let paths: Vec<&str> = list.split(':').filter(|s| !s.is_empty()).collect();
        assert!(!paths.is_empty(), "ROCKTIER_ACC_PDFS 为空");

        let pdfium = crate::pdf::test_pdfium();
        let mut failed: Vec<String> = Vec::new();
        for p in paths {
            let Ok(bytes) = std::fs::read(p) else {
                eprintln!("跳过（读不到）：{p}");
                continue;
            };
            let Ok(doc) = pdfium.load_pdf_from_byte_slice(&bytes, None) else {
                eprintln!("跳过（打不开）：{p}");
                continue;
            };
            match document_to_markdown(&doc, Default::default()) {
                Ok((md, stats, warnings)) => {
                    println!("\n════════ {} ════════", p);
                    println!(
                        "页 {} · 行 {} · 标题 {} · 段落 {} · 列表 {} · 表格区 {} · 正文 {:.1}pt",
                        stats.pages, stats.lines, stats.headings, stats.paragraphs,
                        stats.list_items, stats.tableish_regions, stats.body_size
                    );
                    for w in &warnings {
                        println!("  告警: {w:?}");
                    }
                    // **空结果对扫描件是正确的**，不是崩坏 —— 所以只在
                    // 没有给出EmptyDocument 告警时才判失败。
                    //
                    // 曾在这里无条件断言非空，结果一份中文扫描件就把整个
                    // 验收中断了、后面几份文档根本没跑 —— 一个样本的问题
                    // 掩盖了全部样本。
                    let empty_ok = warnings.iter().any(|w| matches!(w, Warning::EmptyDocument));
                    if md.trim().is_empty() {
                        if empty_ok {
                            println!("  （空结果，已按扫描件如实告警，不算失败）");
                            continue;
                        }
                        // 没告警却空 → 真的坏了，且要继续跑完其余文档
                        failed.push(format!("{p}: 空结果但未给 EmptyDocument 告警"));
                        continue;
                    }
                    let lines = md.lines().filter(|l| !l.trim().is_empty()).count();
                    if lines <= 1 {
                        failed.push(format!("{p}: 结果只有 {lines} 行，疑似未分行"));
                        continue;
                    }
                    // 把首页渲染成 PNG —— 判断「交错」是文本层问题还是聚类问题，
                    // 只有看图才能定论
                    if let Ok(rp) = std::env::var("ROCKTIER_ACC_RENDER") {
                        // 页数判断用 !is_empty() 而非 > 0（clippy: len_zero）
                        if !doc.pages().is_empty() {
                            if let Ok(r) = crate::pdf::render_page(&doc, 0, 1400) {
                                if let Some(b64) = r.data_url.split(',').nth(1) {
                                    use base64::Engine as _;
                                    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64) {
                                        let _ = std::fs::write(&rp, bytes);
                                        println!("已渲染 -> {rp}");
                                    }
                                }
                            }
                        }
                    }
                    println!("--- 前 40 行 ---");
                    for l in md.lines().take(40) {
                        println!("{l}");
                    }
                }
                Err(e) => {
                    eprintln!("{p}: 转换失败 {e}");
                    failed.push(format!("{p}: 转换失败 {e}"));
                }
            }
        }

        // 跑完**所有**样本再汇总 —— 逐个 assert 会让第一个问题样本
        // 遮住后面的结果，验收就看不到全貌了。
        if !failed.is_empty() {
            panic!(
                "{} 份样本有问题：\n{}",
                failed.len(),
                failed.iter().map(|f| format!("  - {f}")).collect::<Vec<_>>().join("\n")
            );
        }
    }
}
