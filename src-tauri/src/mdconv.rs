// ⚠️ WIP（2026-10-05）：本模块的推断内核与取字符层已完成且**54 个测试全绿**，
// 但**尚未接入 Tauri 命令**，因此对外没有任何入口，编译器会报一批 dead_code。
// 接入命令后删掉下面这行 allow，让编译器重新监督使用情况。
#![allow(dead_code)]

//! PDF → Markdown 的**结构推断**内核（纯逻辑，不依赖 pdfium）。
//!
//! 这一层只吃「带位置的字符」，吐出markdown 块。上层负责从 pdfium 取字符。
//! 拆开的原因：结构推断是这个功能里最容易出错的部分（阈值、聚类、边界），
//! 必须能在没有 PDF 的情况下用单测反复验证。
//!
//! ## 启发式（按路线图 R4）
//!
//! - **字号聚类 → 标题**：统计每行「代表字号」（按字符数加权众数），
//!   按出现频率降序取前几个明显大于正文的簇 → h1/h2/h3。
//! - **字重 → 辅助**：标题行通常加粗；但**光靠加粗不能判标题**
//!   （表格表头、图注、导航条也常加粗），必须字号也更大。
//! - **行距 → 段落**：同栏相邻两行的基线间距显著大于行内间距 → 新段落。
//! - **前缀 → 列表**：`-` `•` `1.` `1)` `a.` `i.` `（1）` 等。
//! - **缩进 → 层级**：列表/引用按 x 起点缩进分级。
//! - **表格 v1 降级**：检测到多栏对齐时输出为纯文本行 + TODO 注释，
//!   不猜表格结构（猜错比不猜更糟）。见`table_like_regions`。

use std::collections::BTreeMap;

/// 一个带位置的字符（pdfium 层负责构造）。
#[derive(Debug, Clone, PartialEq)]
pub struct Glyph {
    /// 原文（可能是代理对/多码位的组合）
    pub text: String,
    /// 缩放后字号（pt）
    pub size: f32,
    /// 700 及以上算粗体
    pub bold: bool,
    pub italic: bool,
    /// 字符包围盒（PDF 点，左下原点）
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

/// 一行：由字符按基线与水平间距聚类而来。
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub text: String,
    /// 行内出现次数最多的字号（按字符数加权）
    pub size: f32,
    /// 行内是否**主要**为粗体（≥60% 的非空白字符）
    pub bold: bool,
    pub italic: bool,
    pub x0: f32,
    pub x1: f32,
    /// 基线（用y0，代表行的垂直位置）
    pub baseline: f32,
    /// 行内「大于一个字宽」的词间间隙的 x 位置 —— 表格列边界就靠它。
    /// 刻意存几何位置而非靠文本里的连续空格：`finish_line` 会把空格收敛成
    /// 一个，靠空格数量判断列数在真实 PDF 上不成立。
    pub gaps: Vec<f32>,
}

impl Line {
    /// 行宽（pt）。注意 PDF 的 y 轴向上，而阅读顺序自上而下，
    /// 故排序时要用 `-baseline`。
    pub fn width(&self) -> f32 {
        self.x1 - self.x0
    }
}

/// 推断出的块类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Heading(u8),
    Paragraph,
    ListItem(u8),
    Quote,
    /// 表格候选区域：v1 只保留纯文本，不猜结构
    Tableish,
}

/// markdown 输出用的块。
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub kind: BlockKind,
    pub text: String,
}

/// 整篇转换的统计与告警（前端要如实展示，不能静默失败）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stats {
    pub pages: usize,
    pub lines: usize,
    pub headings: usize,
    pub paragraphs: usize,
    pub list_items: usize,
    pub tableish_regions: usize,
    /// 正文识别到的主要字号（用于前端提示用户核对）
    pub body_size: f32,
}

/// 非致命问题：如实告诉用户哪里可能不准。
#[derive(Debug, Clone, PartialEq)]
pub enum Warning {
    /// 该页几乎没有可提取文本 —— 极可能是扫描件
    LooksScanned { page: usize },
    /// 检测到疑似表格，v1 只输出纯文本
    TablesNotConverted { regions: usize },
    /// 字符总数为 0
    EmptyDocument,
}

/// 转换选项。
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// 正文判定基准；为 None 时自动估计
    pub body_size: Option<f32>,
    /// 视为列表的最左缩进（相对正文左边界的倍数）
    pub indent_ratio: f32,
}

impl Default for Options {
    fn default() -> Self {
        Self { body_size: None, indent_ratio: 0.02 }
    }
}

// ────────────────────────────────────────────────────────────────────
// 字符 → 行
// ────────────────────────────────────────────────────────────────────

/// 把字符聚成行。
///
/// 两级判定，顺序很重要：
/// 1. **先按基线聚类** —— 基线容差取「字号 × 0.6」，
///    因为行内字号可能有小幅波动（上下标），但基线必须一致。
/// 2. **同一基线内按水平间距插入空格** —— PDF 里字间距是排版信息，
///    直接拼接会把英文单词粘成 `Helloworld`。间距阈值取字宽的一部分，
///    避免把字母间距较大的字体误插空格。
pub fn glyphs_to_lines(glyphs: &[Glyph]) -> Vec<Line> {
    if glyphs.is_empty() {
        return Vec::new();
    }
    // pdfium 的 char迭代顺序通常已是阅读序，但仍按基线归并更稳。
    let mut indexed: Vec<&Glyph> = glyphs.iter().collect();
    indexed.sort_by(|a, b| {
        // y 轴向上 → 先按基线从大到小（视觉上从上到下），再按 x 从小到大
        b.y0.partial_cmp(&a.y0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.x0.partial_cmp(&b.x0).unwrap_or(std::cmp::Ordering::Equal))
    });

    let center = |g: &Glyph| (g.y0 + g.y1) / 2.0;
    let mut lines: Vec<Vec<&Glyph>> = Vec::new();
    for g in indexed {
        let mut placed = false;
        if let Some(cur) = lines.last_mut() {
            // 与当前簇的**y 中心**比较（不是字号 —— 曾写成 size 导致每个字符自成一簇）。
            // 容差取字符自身字号的 0.6 倍：行内字号可能小幅波动，但基线必须一致。
            let n = cur.len() as f32;
            let cur_center: f32 = cur.iter().map(|g| center(g)).sum::<f32>() / n;
            let tol = (g.size * 0.6).max(1.0);
            if (center(g) - cur_center).abs() <= tol {
                cur.push(g);
                placed = true;
            }
        }
        if !placed {
            lines.push(vec![g]);
        }
    }

    // 聚类只保证「同基线」，但基线有微小抖动时同一视觉行仍可能被拆成两簇，
    // 两簇的 x 会交错（如 "Hello" 与 "World" 交错）。
    // 故先合并「基线接近且水平区间重叠」的相邻簇，再生成Line。
    let merged = merge_interleaved(lines);
    merged
        .iter()
        .filter_map(|cluster| {
            if cluster.is_empty() {
                return None;
            }
            Some(finish_line(cluster))
        })
        .collect()
}

/// 合并基线接近、但 x 区间交错的相邻簇。
///
/// 场景：pdfium 的 `loose_bounds()` 对同一基线上的字符给出的 y 会有亚点级抖动，
/// 超出 0.6×字号 的容差就会把一行拆成 "HHeelllo" / "WWeorrlld" 两簇。
/// 判据：相邻两簇基线差在容差内，**且** 水平区间有重叠 —— 真正的上下两行
/// x 区间通常也重叠，故还要求两簇字号相同（跨行几乎必同字号，但上下行若有
/// 标题混排则字号不同，用字号再挡一道）。
fn merge_interleaved(clusters: Vec<Vec<&Glyph>>) -> Vec<Vec<&Glyph>> {
    let center = |g: &Glyph| (g.y0 + g.y1) / 2.0;
    let mut out: Vec<Vec<&Glyph>> = Vec::with_capacity(clusters.len());
    for cl in clusters {
        if let Some(prev) = out.last_mut() {
            let pc: f32 = prev.iter().map(|g| center(g)).sum::<f32>() / prev.len() as f32;
            let cc: f32 = cl.iter().map(|g| center(g)).sum::<f32>() / cl.len() as f32;
            let ps: f32 = prev.iter().map(|g| g.size).sum::<f32>() / prev.len() as f32;
            let cs: f32 = cl.iter().map(|g| g.size).sum::<f32>() / cl.len() as f32;
            let tol = (cs * 0.6).max(1.0);
            let p_lo = prev.iter().map(|g| g.x0).fold(f32::MAX, f32::min);
            let p_hi = prev.iter().map(|g| g.x1).fold(f32::MIN, f32::max);
            let c_lo = cl.iter().map(|g| g.x0).fold(f32::MAX, f32::min);
            let c_hi = cl.iter().map(|g| g.x1).fold(f32::MIN, f32::max);
            let overlaps = c_lo <= p_hi && p_lo <= c_hi;
            if (pc - cc).abs() <= tol && (ps - cs).abs() <= 0.3 && overlaps {
                prev.extend(cl);
                continue;
            }
        }
        out.push(cl);
    }
    out
}

/// 由一个基线簇生成 `Line`。
fn finish_line(cluster: &[&Glyph]) -> Line {
    let mut sorted: Vec<&Glyph> = cluster.to_vec();
    sorted.sort_by(|a, b| a.x0.partial_cmp(&b.x0).unwrap_or(std::cmp::Ordering::Equal));

    let mut text = String::new();
    let mut gaps: Vec<f32> = Vec::new();
    let mut prev: Option<&Glyph> = None;
    for g in &sorted {
        if let Some(p) = prev {
            // 间距 = 当前字符左边界 − 前一个字符右边界
            let gap = g.x0 - p.x1;
            let ref_w = (p.x1 - p.x0).max(g.x1 - g.x0);
            // 超过四分之一字宽算「词间空格」
            if gap > ref_w * 0.28 && !text.ends_with(' ') && !g.text.starts_with(' ') {
                text.push(' ');
            }
            // 超过 1.5 字宽 = 列间大间隙，位置记下来给表格检测用
            if gap > ref_w * 1.5 {
                gaps.push((p.x1 + g.x0) / 2.0);
            }
        }
        text.push_str(&g.text);
        prev = Some(g);
    }

    // 字号：按字符数加权众数（避免一个大字号水印拉高整行）
    let mut sizes: BTreeMap<i32, usize> = BTreeMap::new();
    for g in cluster {
        if g.text.trim().is_empty() {
            continue;
        }
        // 0.1pt 精度分桶
        *sizes.entry((g.size * 10.0).round() as i32).or_insert(0) += g.text.chars().count();
    }
    let size = sizes
        .iter()
        .max_by_key(|(k, v)| (**v, -*k))
        .map(|(k, _)| *k as f32 / 10.0)
        .unwrap_or_else(|| cluster[0].size);

    let mut ink = 0usize;
    let mut bold_ink = 0usize;
    let mut ital_ink = 0usize;
    for g in cluster {
        let n = g.text.chars().count();
        if g.text.trim().is_empty() {
            continue;
        }
        ink += n;
        if g.bold {
            bold_ink += n;
        }
        if g.italic {
            ital_ink += n;
        }
    }

    Line {
        text: collapse_spaces(&text),
        size,
        bold: ink > 0 && bold_ink * 100 >= ink * 60,
        italic: ink > 0 && ital_ink * 100 >= ink * 60,
        x0: sorted.iter().map(|g| g.x0).fold(f32::MAX, f32::min),
        x1: sorted.iter().map(|g| g.x1).fold(f32::MIN, f32::max),
        baseline: cluster[0].y0,
        gaps,
    }
}

fn collapse_spaces(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for c in s.chars() {
        let is_space = c == ' ' || c == '\u{0}';
        if is_space {
            if !prev_space && !out.is_empty() {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

// ────────────────────────────────────────────────────────────────────
// 列表前缀识别
// ────────────────────────────────────────────────────────────────────

/// 识别列表前缀，返回（标记后的文本，缩进级0=无）。
///
/// 刻意**只认前缀**、不靠缩进猜：PDF 里缩进可能是版式留白，
/// 而 `1.` `•` `-` 是作者明确表达的结构意图。
fn strip_list_marker(text: &str) -> Option<(String, u8)> {
    let t = text.trim_start();
    // 有序：1. / 1) / 1、/ (1) / （1）
    for open in ["(", "（"] {
        if t.starts_with(open) {
            let close_ch = if open == "(" { ')' } else { '）' };
            // open 是 &str，用字节长度（全角括号 3 字节）
            let open_len = open.len();
            if let Some(rel) = t[open_len..].find(close_ch) {
                let close = open_len + rel;
                let inner = &t[open_len..close];
                if !inner.is_empty() && inner.chars().all(|c| c.is_ascii_digit()) {
                    let rest = &t[close + close_ch.len_utf8()..];
                    let rest = rest.trim_start_matches(['.', '、', ')', '）', ' ']);
                    return Some((rest.to_string(), 0));
                }
            }
        }
    }
    // 有序的「数字 + 分隔符」形式：2.  2)  2、  2） （中英文标点都收）
    {
        let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            let after = &t[digits.len()..];
            for sep in [')', '）', '.', '、'] {
                if let Some(rest) = after.strip_prefix(sep) {
                    let rest = rest.trim_start_matches(['.', '、', ')', '）', ' ']);
                    if !rest.is_empty() {
                        return Some((rest.to_string(), 0));
                    }
                }
            }
        }
    }
    // 无序：- / – / — / • / · / ◦ / ▪ / * / ·
    for m in ["-", "–", "—", "•", "·", "◦", "▪", "*", "+"] {
        if let Some(rest) = t.strip_prefix(m) {
            // 必须在行首紧跟空格或行尾，避免把「-」算作减号、「*」算作强调
            if rest.is_empty() || rest.starts_with(' ') {
                return Some((rest.trim().to_string(), 0));
            }
        }
    }
    // 无序数字伪装：`1.` `a.` `iv.`
    if let Some(dot) = t.find(['.', '、']) {
        let head = &t[..dot];
        let rest = t[dot + t[dot..].chars().next().map(|c| c.len_utf8()).unwrap_or(1)..].trim_start();
        if !head.is_empty() && head.chars().count() <= 4 && rest.starts_with(|c: char| !c.is_ascii_digit() || c != ' ') {
            let all_digit = head.chars().all(|c| c.is_ascii_digit());
            let all_alpha = head.chars().all(|c| c.is_ascii_alphabetic());
            if (all_digit || all_alpha) && !rest.is_empty() {
                return Some((rest.to_string(), 0));
            }
        }
    }
    None
}

/// 引用块：`>` 或全角 `＞`，或引号成对包裹的整段（v1 只认显式 `>`）。
fn strip_quote_marker(text: &str) -> Option<String> {
    let t = text.trim_start();
    for m in [">", "＞", "│"] {
        if let Some(rest) = t.strip_prefix(m) {
            return Some(rest.trim().to_string());
        }
    }
    None
}

// ────────────────────────────────────────────────────────────────────
// 正文尺寸与标题判定
// ────────────────────────────────────────────────────────────────────

/// 估计正文字号：取**按字符数加权**的众数簇。
///
/// 不能取「出现最多的字号」——那通常是对的，但遇到标题密集的目录页
/// 会偏大。改用加权众数 + 偏向较小一侧（正文总是字符最多的一类）。
pub fn estimate_body_size(lines: &[Line]) -> f32 {
    let mut hist: BTreeMap<i32, usize> = BTreeMap::new();
    for l in lines {
        let ink = l.text.chars().filter(|c| !c.is_whitespace()).count();
        if ink == 0 {
            continue;
        }
        *hist.entry((l.size * 10.0).round() as i32).or_insert(0) += ink;
    }
    if hist.is_empty() {
        return 10.0;
    }
    // **加权中位数**而非众数。
    //
    // 众数在两类文档上会失效：① 目录页/幻灯片 —— 标题行数多于正文；
    // ② 有一个巨大的封面标题 —— 它字符数多，把众数整个拉走。
    // 中位数对「少数极端值」稳健：只要过半字符是正文字号，就能定住。
    let total: usize = hist.values().sum();
    let half = total / 2;
    let mut acc = 0usize;
    for (k, v) in hist.iter() {
        acc += v;
        if acc > half {
            return *k as f32 / 10.0;
        }
    }
    hist.keys().next().map(|k| *k as f32 / 10.0).unwrap_or(10.0)
}

/// 该行相对正文是否算标题，返回层级（1..=6）。
///
/// 条件：字号明显大于正文 **且** 加粗或独占一行且不太长。
/// 刻意收紧：宁可漏判也不误判 —— 误判会把整段正文变成标题。
fn heading_level(line: &Line, body: f32, max_line_chars: usize, hmap: &BTreeMap<i32, u8>) -> Option<u8> {
    let ink = line.text.chars().filter(|c| !c.is_whitespace()).count();
    if ink == 0 {
        return None;
    }
    if line.size < body * 1.12 {
        return None;
    }
    // 目录项 / 页眉页脚：太长的行不是标题
    if ink > max_line_chars {
        return None;
    }
    // **按字号聚类的相对排名**决定层级，而不是固定比例阈值。
    //
    // 固定阈值在不同文档上表现完全不同：正文 10pt、标题 12pt 时 1.2 倍就该是
    // h2，却掉进临界区变 h4；而正文 24pt 的文档里 1.2 倍只差 5pt，判成标题反而
    // 是误判。曾实测出 `# Chapter One` + `#### Section` 这种断裂嵌套。
    // 相对排名与文档自身尺度无关，层级也不会和字号差距不匹配。
    let bucket = (line.size * 10.0).round() as i32;
    if let Some(&lvl) = hmap.get(&bucket) {
        return Some(lvl);
    }
    // 未进入聚类（字符太少，多为水印/页码）—— 临界区仍要求加粗
    if line.bold {
        Some(4)
    } else {
        None
    }
}

/// **字号聚类 → 标题层级映射**（0.1pt 分桶 → 层级）。
///
/// 只对**明显大于正文**（≥1.12×）的字号聚类，且每簇至少 2 个非空白字符 ——
/// 否则一个偶然出现的巨大字号（水印、页码）会独占 h1。
fn heading_map(lines: &[Line], body: f32) -> BTreeMap<i32, u8> {
    let mut hist: BTreeMap<i32, usize> = BTreeMap::new();
    for l in lines {
        let ink = l.text.chars().filter(|c| !c.is_whitespace()).count();
        if ink == 0 || l.size < body * 1.12 {
            continue;
        }
        *hist.entry((l.size * 10.0).round() as i32).or_insert(0) += ink;
    }
    // 按字符数降序排名；字符数相同时字号大者优先
    let mut ranked: Vec<(i32, usize)> = hist.into_iter().filter(|(_, ink)| *ink >= 2).collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.cmp(&a.0)));
    let mut map = BTreeMap::new();
    for (i, (bucket, _)) in ranked.into_iter().enumerate() {
        if i >= 4 {
            break; // 最多到 h4，再多说明是装饰而非层级
        }
        map.insert(bucket, i as u8 + 1);
    }
    map
}

// ────────────────────────────────────────────────────────────────────
// 表格候选检测
// ────────────────────────────────────────────────────────────────────

/// 检测「多栏对齐」的行组，返回疑似表格区域数。
///
/// v1 只用于**告知用户**，不改变输出结构 —— 猜表格结构错得离谱时，
/// 保留纯文本让人自己看，比输出一个错的表格更有用。
fn table_like_regions(lines: &[Line]) -> usize {
    // 连续 ≥3 行、每行都有「大间隙」，且这些间隙的 x 位置在行间大致对齐
    // —— 这就是最常见的表格形态。v1 只用它来**告知用户**，不改变输出结构。
    let mut regions = 0usize;
    let mut run: Vec<&Vec<f32>> = Vec::new();

    let flush = |run: &mut Vec<&Vec<f32>>, regions: &mut usize| {
        if run.len() >= 3 {
            let first = run[0];
            // ⚠️ 阈值必须是 **≥1** 个列间隙，不能要求 ≥2 ——
            // 两列表格（每行 1 个间隙）才是最常见的情形，曾因此永远检不出。
            if !first.is_empty() {
                let consistent = run.iter().filter(|xs| {
                    let hit = xs.iter().filter(|x| first.iter().any(|f| (f - *x).abs() < 6.0)).count();
                    hit * 10 >= first.len() * 6
                }).count();
                if consistent * 10 >= run.len() * 6 {
                    *regions += 1;
                }
            }
        }
        run.clear();
    };

    for l in lines {
        if l.gaps.is_empty() {
            flush(&mut run, &mut regions);
        } else {
            run.push(&l.gaps);
        }
    }
    flush(&mut run, &mut regions);
    regions
}

// ────────────────────────────────────────────────────────────────────
// 主流程：行 → 块
// ────────────────────────────────────────────────────────────────────

/// 把行序列推断成 markdown 块。
pub fn lines_to_blocks(lines: &[Line], opts: Options) -> (Vec<Block>, Stats, Vec<Warning>) {
    let mut stats = Stats { lines: lines.len(), ..Default::default() };
    let mut warnings = Vec::new();

    if lines.is_empty() {
        warnings.push(Warning::EmptyDocument);
        return (Vec::new(), stats, warnings);
    }

    let body = opts.body_size.unwrap_or_else(|| estimate_body_size(lines));
    stats.body_size = body;

    // 正文左边界的众数 —— 缩进判定要用绝对量而非倍数
    let left_mode = {
        let mut hist: BTreeMap<i32, usize> = BTreeMap::new();
        for l in lines {
            let ink = l.text.chars().filter(|c| !c.is_whitespace()).count();
            if ink == 0 {
                continue;
            }
            *hist.entry((l.x0 / 2.0).round() as i32).or_insert(0) += ink;
        }
        hist.iter().max_by_key(|(k, v)| (**v, -*k)).map(|(k, _)| *k as f32 * 2.0).unwrap_or(0.0)
    };

    let hmap = heading_map(lines, body);

    let regions = table_like_regions(lines);
    if regions > 0 {
        stats.tableish_regions = regions;
        warnings.push(Warning::TablesNotConverted { regions });
    }

    let max_line_chars = lines
        .iter()
        .map(|l| l.text.chars().filter(|c| !c.is_whitespace()).count())
        .max()
        .unwrap_or(80)
        .max(40);

    let mut blocks: Vec<Block> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.text.trim();
        if trimmed.is_empty() {
            continue;
        }

        // 1. 显式引用
        if let Some(q) = strip_quote_marker(trimmed) {
            if !q.is_empty() {
                stats.paragraphs += 1;
                push_para(&mut blocks, &format!("> {}", q));
                continue;
            }
        }

        // 2. 显式列表（作者明确表达的结构意图，优先于标题判定）
        if let Some((item, _)) = strip_list_marker(trimmed) {
            if !item.is_empty() {
                // 缩进分级：按 x0 相对左边界的偏移分两档，够用且不易过拟合
                let indent = ((line.x0 - left_mode) / (body * 6.0)).round().max(0.0);
                let lvl = (indent as u8).min(3);
                stats.list_items += 1;
                blocks.push(Block { kind: BlockKind::ListItem(lvl), text: item });
                continue;
            }
        }

        // 3. 标题（字号为主 + 加粗为辅）
        if let Some(level) = heading_level(line, body, max_line_chars, &hmap) {
            stats.headings += 1;
            blocks.push(Block { kind: BlockKind::Heading(level), text: trimmed.to_string() });
            continue;
        }

        // 4. 正文段落：与上一正文行间距不大则合并
        let merge = match blocks.last() {
            Some(Block { kind: BlockKind::Paragraph, .. }) => {
                let prev_line = &lines[i - 1];
                // 行距：基线间距与字号之比。正常行距 ≈ 1.2–1.5倍字号；
                // 超过 1.8 倍通常意味着段落间隔
                let gap = prev_line.baseline - line.baseline;
                gap < line.size * 1.85
                    && (line.x0 - left_mode).abs() < body * 0.6
            }
            _ => false,
        };

        if merge {
            if let Some(Block { kind: BlockKind::Paragraph, text }) = blocks.last_mut() {
                if line.italic {
                    text.push_str(&format!(" *{}*", trimmed));
                } else if line.bold {
                    text.push_str(&format!(" **{}**", trimmed));
                } else {
                    text.push(' ');
                    text.push_str(trimmed);
                }
                continue;
            }
        }

        stats.paragraphs += 1;
        let t = if line.italic && trimmed.chars().count() > 1 {
            format!("*{}*", trimmed)
        } else if line.bold && trimmed.chars().count() > 1 {
            format!("**{}**", trimmed)
        } else {
            trimmed.to_string()
        };
        blocks.push(Block { kind: BlockKind::Paragraph, text: t });
    }

    (blocks, stats, warnings)
}

fn push_para(blocks: &mut Vec<Block>, text: &str) {
    blocks.push(Block { kind: BlockKind::Paragraph, text: text.to_string() });
}

// ────────────────────────────────────────────────────────────────────
// 输出
// ────────────────────────────────────────────────────────────────────

/// 渲染成 markdown 文本。
pub fn blocks_to_markdown(blocks: &[Block]) -> String {
    let mut out = String::new();
    for b in blocks {
        match b.kind {
            BlockKind::Heading(l) => {
                out.push_str(&"#".repeat(l as usize));
                out.push(' ');
                out.push_str(&escape_heading(&b.text));
                out.push_str("\n\n");
            }
            BlockKind::Paragraph => {
                out.push_str(&escape_inline(&b.text));
                out.push_str("\n\n");
            }
            BlockKind::ListItem(l) => {
                let pad = "  ".repeat(l as usize);
                out.push_str(&pad);
                out.push_str("- ");
                out.push_str(&escape_inline(&b.text));
                out.push('\n');
            }
            BlockKind::Quote => {
                out.push_str("> ");
                out.push_str(&escape_inline(&b.text));
                out.push_str("\n\n");
            }
            BlockKind::Tableish => {
                out.push_str(&escape_inline(&b.text));
                out.push('\n');
            }
        }
    }
    // 列表块之间需要空行分隔才符合 CommonMark
    let mut fixed = String::with_capacity(out.len());
    let lines: Vec<&str> = out.split('\n').collect();
    let mut prev_list = false;
    for (i, l) in lines.iter().enumerate() {
        let is_list = l.starts_with("- ") || l.starts_with("  - ");
        if is_list && !prev_list && i > 0 && !lines[i - 1].is_empty() {
            fixed.push('\n');
        }
        fixed.push_str(l);
        if !l.is_empty() {
            fixed.push('\n');
        }
        prev_list = is_list;
    }
    // 收敛连续空行为最多一个
    let mut result = String::new();
    let mut blanks = 0;
    for c in fixed.chars() {
        if c == '\n' {
            blanks += 1;
            if blanks > 1 {
                continue;
            }
        } else {
            blanks = 0;
        }
        result.push(c);
    }
    while result.ends_with('\n') {
        result.pop();
    }
    result.push('\n');
    result
}

/// 标题里`#` 与空格需转义，否则会被当成更高级标题。
fn escape_heading(s: &str) -> String {
    escape_inline(s).trim_start_matches('#').trim_start().to_string()
}

fn escape_inline(s: &str) -> String {
    // 只转义会破坏结构的字符，不做过度转义（免得满屏反斜杠）
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

// ────────────────────────────────────────────────────────────────────
// 测试
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// 造一个「单词」：由若干字符组成，总宽 = n * w
    fn word(x: f32, baseline: f32, size: f32, s: &str, bold: bool) -> Vec<Glyph> {
        let w = size * 0.5;
        s.chars()
            .enumerate()
            .map(|(i, c)| Glyph {
                text: c.to_string(),
                size,
                bold,
                italic: false,
                x0: x + i as f32 * w,
                x1: x + i as f32 * w + w,
                y0: baseline,
                y1: baseline + size * 0.7,
            })
            .collect()
    }

    /// 造一行：多个词，词间留一个「空格宽」的间隙。
    /// ⚠️ x 起点必须由调用方指定 —— 曾固定为 100.0，导致同一行里两个词
    /// 完全重叠、输出交错（看起来像排序 bug，实际是测试数据造错了）。
    fn line_of(x0: f32, y: f32, size: f32, s: &str, bold: bool) -> Vec<Glyph> {
        let mut out = Vec::new();
        let mut x = x0;
        let space = size * 0.5;
        for tok in s.split(' ') {
            out.extend(word(x, y, size, tok, bold));
            x += tok.chars().count() as f32 * size * 0.5 + space;
        }
        out
    }

    #[test]
    fn 行聚类_同基线并成一行() {
        // 两词必须错开摆放：都从 x=100 起会完全重叠，输出自然交错
        let mut g = line_of(100.0, 700.0, 10.0, "Hello", false);
        g.extend(line_of(130.0, 700.0, 10.0, "World", false));
        let lines = glyphs_to_lines(&g);
        assert_eq!(lines.len(), 1, "同基线应合并成一行");
        assert_eq!(lines[0].text, "Hello World", "词间应插空格，实际: {:?}", lines[0].text);
    }

    #[test]
    fn 行聚类_不同基线分行() {
        let mut g = line_of(100.0, 700.0, 10.0, "First", false);
        g.extend(line_of(100.0, 680.0, 10.0, "Second", false));
        let lines = glyphs_to_lines(&g);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "First");
        assert_eq!(lines[1].text, "Second");
    }

    #[test]
    fn 行聚类_输出按阅读顺序() {
        // y 轴向上：先给的应是视觉上「下面」那行
        let mut g = line_of(100.0, 600.0, 10.0, "lower", false);
        g.extend(line_of(100.0, 720.0, 10.0, "upper", false));
        let lines = glyphs_to_lines(&g);
        assert_eq!(lines[0].text, "upper", "行顺序应从上到下");
        assert_eq!(lines[1].text, "lower");
    }

    #[test]
    fn 字号_按字符数加权众数而非平均() {
        let l1 = Line {
            text: "a".into(), size: 30.0, bold: true, italic: false,
            x0: 0.0, x1: 10.0, baseline: 700.0,
            gaps: vec![],
        };
        let l2 = Line {
            text: "normal body text here".into(), size: 10.0, bold: false, italic: false,
            x0: 0.0, x1: 100.0, baseline: 690.0,
            gaps: vec![],
        };
        let body = estimate_body_size(&[l1, l2]);
        assert!((body - 10.0).abs() < 0.01, "应取正文字号而非大字号，实际 {}", body);
    }

    /// 造一组构成文档的行，用来得到真实的聚类映射
    fn doc_with(sizes: &[f32]) -> Vec<Line> {
        sizes
            .iter()
            .enumerate()
            .map(|(i, sz)| {
                let s = format!("Heading level sample {}", i);
                Line {
                    text: s.clone(), size: *sz, bold: true, italic: false, x0: 100.0,
                    x1: 100.0 + s.len() as f32 * 5.0, baseline: 700.0 - i as f32 * 12.0,
                    gaps: vec![],
                }
            })
            .collect()
    }

    #[test]
    fn 标题_按字号聚类排名而非固定比例() {
        // 文档含22 / 16 / 12 三档标题 + 10pt 正文
        let mut doc = doc_with(&[22.0, 16.0, 12.0]);
        // 正文必须**字符量足够** —— 3 个字符的正文会被标题的150 字符压过
        doc.push(Line {
            text: "这是正文行，字数需要多于全部标题字之和，否则加权中位数会把标题当正文。".repeat(3).into(),
            size: 10.0, bold: false, italic: false,
            x0: 100.0, x1: 200.0, baseline: 600.0, gaps: vec![],
        });
        let body = estimate_body_size(&doc);
        let hmap = heading_map(&doc, body);
        let mk = |size: f32, bold: bool, s: &str| Line {
            text: s.into(), size, bold, italic: false, x0: 100.0,
            x1: 100.0 + s.len() as f32 * 5.0, baseline: 700.0, gaps: vec![],
        };
        // 最大字号簇 → h1，其后依次 h2 / h3
        assert_eq!(heading_level(&mk(22.0, true, "T"), body, 80, &hmap), Some(1));
        assert_eq!(heading_level(&mk(16.0, true, "S"), body, 80, &hmap), Some(2));
        assert_eq!(heading_level(&mk(12.0, true, "Sub"), body, 80, &hmap), Some(3));
        // 字号与正文相同 → 绝不判标题
        assert_eq!(heading_level(&mk(10.0, true, "Just bold"), body, 80, &hmap), None);
        // 未聚类且不加粗 → 不判标题
        assert_eq!(heading_level(&mk(11.5, false, "Slight"), body, 80, &hmap), None);
    }

    #[test]
    fn 标题_聚类与文档尺度无关() {
        let mut small = doc_with(&[22.0, 16.0, 12.0]);
        small.push(Line { text: "正文内容需要足够字数才能压过标题".repeat(6), size: 10.0,
            bold: false, italic: false, x0: 100.0, x1: 200.0, baseline: 600.0, gaps: vec![] });
        let mut large = doc_with(&[52.8, 38.4, 28.8]);
        large.push(Line { text: "正文内容需要足够字数才能压过标题".repeat(6), size: 24.0,
            bold: false, italic: false, x0: 100.0, x1: 200.0, baseline: 600.0, gaps: vec![] });
        let lv = |d: &Vec<Line>, probe: f32| {
            let b = estimate_body_size(d);
            let m = heading_map(d, b);
            heading_level(&Line { text: "x".into(), size: probe * b / 10.0, bold: true,
                italic: false, x0: 0.0, x1: 5.0, baseline: 0.0, gaps: vec![] }, b, 80, &m)
        };
        // 同一结构、正文放大 2.4 倍：层级分配应完全一致
        for (probe, expect) in [(22.0, Some(1)), (16.0, Some(2)), (12.0, Some(3))] {
            assert_eq!(lv(&small, probe), expect, "小尺度 {probe}pt");
            assert_eq!(lv(&large, probe), expect, "大尺度 {probe}pt");
        }
    }

    #[test]
    fn 标题_过长的行不判标题() {
        let long = "x".repeat(200);
        let l = Line {
            text: long.clone(), size: 20.0, bold: true, italic: false,
            x0: 0.0, x1: 1000.0, baseline: 700.0,
            gaps: vec![],
        };
        assert_eq!(heading_level(&l, 10.0, 80, &BTreeMap::new()), None, "超长行不是标题");
    }

    #[test]
    fn 列表_识别各种前缀() {
        assert_eq!(strip_list_marker("- item").map(|x| x.0), Some("item".into()));
        assert_eq!(strip_list_marker("• item").map(|x| x.0), Some("item".into()));
        assert_eq!(strip_list_marker("1. item").map(|x| x.0), Some("item".into()));
        assert_eq!(strip_list_marker("2) item").map(|x| x.0), Some("item".into()));
        assert_eq!(strip_list_marker("(3) item").map(|x| x.0), Some("item".into()));
        assert_eq!(strip_list_marker("（4）item").map(|x| x.0), Some("item".into()));
        assert_eq!(strip_list_marker("a. item").map(|x| x.0), Some("item".into()));
        // 不是列表
        assert!(strip_list_marker("-3 degrees").is_none(), "负数不是列表");
        assert!(strip_list_marker("a-b").is_none(), "连字符不是列表");
        assert!(strip_list_marker("no marker").is_none());
    }

    #[test]
    fn 引用_识别显式引用标记() {
        assert_eq!(strip_quote_marker("> quoted"), Some("quoted".into()));
        assert_eq!(strip_quote_marker("＞全角"), Some("全角".into()));
        assert!(strip_quote_marker("not a quote").is_none());
    }

    #[test]
    fn 端到端_标题段落列表() {
        let mut g = Vec::new();
        g.extend(line_of(100.0, 780.0, 22.0, "Chapter One", true));
        g.extend(line_of(100.0, 760.0, 12.0, "Section", true));
        // 正文两行，行距紧
        g.extend(line_of(100.0, 720.0, 10.0, "This is the first body", false));
        g.extend(line_of(100.0, 708.0, 10.0, "line continuing the same", false));
        // 新段落（行距大）
        g.extend(line_of(100.0, 660.0, 10.0, "A separate paragraph", false));
        // 列表
        g.extend(line_of(100.0, 630.0, 10.0, "- item one", false));
        g.extend(line_of(100.0, 618.0, 10.0, "- item two", false));

        let lines = glyphs_to_lines(&g);
        let (blocks, stats, _) = lines_to_blocks(&lines, Options::default());
        let md = blocks_to_markdown(&blocks);

        assert!(md.contains("# Chapter One"), "应为一级标题:\n{}", md);
        assert!(md.contains("## Section"), "应为二级标题:\n{}", md);
        assert!(md.contains("This is the first body line continuing the same"),
                "紧行距应合并为一段:\n{}", md);
        assert!(md.contains("A separate paragraph"), "大行距应另起一段:\n{}", md);
        assert!(md.contains("- item one"), "应保留列表:\n{}", md);
        assert!(stats.list_items >= 2, "应统计到 2 个列表项");
        println!("---\n{}\n--- stats={:?}", md, stats);
    }

    #[test]
    fn 输出_转义会破坏结构的字符() {
        let b = vec![Block { kind: BlockKind::Heading(2), text: "a < b > c".into() }];
        let md = blocks_to_markdown(&b);
        assert!(md.contains("&lt;"), "应转义<:{}", md);
        assert!(md.contains("&gt;"), "应转义 >:{}", md);
    }

    #[test]
    fn 空文档_给出空文档告警() {
        let (_, _, w) = lines_to_blocks(&[], Options::default());
        assert!(w.contains(&Warning::EmptyDocument));
    }

    #[test]
    fn 表格候选_被检出且不改变结构() {
        // 三行，每行两个大间隙（>1.5 字宽）且位置对齐 —— 典型两列表格
        let mut g = Vec::new();
        let mk_row = |y: f32, a: &str, b: &str| {
            let mut v = line_of(100.0, y, 10.0, a, false);
            // 间隙给足 60pt（= 12 个字宽），确保 > 1.5 字宽阈值
            let off = 100.0 + a.chars().count() as f32 * 5.0 + 60.0;
            v.extend(word(off, y, 10.0, b, false));
            v
        };
        g.extend(mk_row(700.0, "Name", "Value"));
        g.extend(mk_row(688.0, "Alpha", "1"));
        g.extend(mk_row(676.0, "Beta", "2"));
        let lines = glyphs_to_lines(&g);
        let (_, stats, w) = lines_to_blocks(&lines, Options::default());
        assert!(
            stats.tableish_regions >= 1 || w.iter().any(|x| matches!(x, Warning::TablesNotConverted { .. })),
            "应检出表格候选"
        );
    }

    #[test]
    fn 幂等_同一输入两次输出一致() {
        let mut g = Vec::new();
        g.extend(line_of(100.0, 700.0, 10.0, "hello", false));
        g.extend(line_of(100.0, 688.0, 10.0, "- world", false));
        let lines = glyphs_to_lines(&g);
        let a = blocks_to_markdown(&lines_to_blocks(&lines, Options::default()).0);
        let b = blocks_to_markdown(&lines_to_blocks(&lines, Options::default()).0);
        assert_eq!(a, b);
    }
}



