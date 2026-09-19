//! 压缩能力 —— 自 Rocktier PDF Squeeze 合并而来。
//!
//! 两步：qpdf 做结构规范化与图像重压（Apache-2.0，只重写图像流与对象结构、
//! 不动内容流，因此文字层逐字节保留），再用我们自己的 imagepass 补上 qpdf
//! 跳过的那一类图像（ICCBased JPEG）。详见 `PDF产品线合并方案.md` §11。

use std::path::{Path, PathBuf};
use std::process::Command;

/// 档位 → qpdf 的 JPEG 质量。与 `imagepass` 的口径必须一致，否则两条路径
/// 会给出不同观感。
pub fn qpdf_args(profile: &str) -> Vec<String> {
    let jpeg_q = match profile {
        "web" => "40",
        "archive" => "85",
        _ => "60",
    };
    vec![
        "--optimize-images".to_string(),
        format!("--jpeg-quality={jpeg_q}"),
        "--object-streams=generate".to_string(),
        "--compress-streams=y".to_string(),
        "--recompress-flate".to_string(),
        "--compression-level=9".to_string(),
    ]
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// 定位 qpdf。打包后它在 `resources/qpdf/`，开发时可回退到 PATH。
pub fn find_qpdf() -> Result<PathBuf, String> {
    let name = if cfg!(windows) { "qpdf.exe" } else { "qpdf" };
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("qpdf").join(name));
            candidates.push(dir.join("resources").join("qpdf").join(name));
            candidates.push(dir.join(name));
        }
    }
    candidates.push(PathBuf::from("src-tauri/resources/qpdf").join(name));
    if let Some(p) = find_in_path(name) {
        candidates.push(p);
    }
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    Err(format!(
        "qpdf engine not found. Searched: {}",
        candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

/// 产物结构自检：非空、`%PDF-` 头、`%%EOF` 尾。
/// qpdf 退出码为 0 不代表产出可用，压缩器绝不能拿用户原件去赌。
pub fn verify_pdf(path: &Path) -> Result<(), String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    if data.len() < 64 {
        return Err(format!("output is only {} bytes", data.len()));
    }
    if !data.starts_with(b"%PDF-") {
        return Err("missing %PDF- header".to_string());
    }
    let tail = &data[data.len().saturating_sub(1024)..];
    if !tail.windows(5).any(|w| w == b"%%EOF") {
        return Err("missing %%EOF trailer".to_string());
    }
    Ok(())
}

/// 压缩 `input` 到 `output`，返回产物字节数。
///
/// 每一步都只在**确实更小**时才采用上一步的产物 —— 图像 pass 用 lopdf 重存
/// 会丢掉 qpdf 生成的压缩对象流，实测有文件因此变大 1–2%，少了这道判断
/// 就会把"压缩"做成"变大"。
pub fn compress(input: &Path, output: &Path, profile: &str) -> Result<u64, String> {
    let qpdf = find_qpdf()?;

    let staged = output.with_extension("qpdf.pdf");
    let _ = std::fs::remove_file(&staged);
    let status = Command::new(&qpdf)
        .args(qpdf_args(profile))
        .arg(input)
        .arg(&staged)
        .status()
        .map_err(|e| format!("Failed to start qpdf: {e}"))?;
    if !status.success() {
        let _ = std::fs::remove_file(&staged);
        return Err(format!("qpdf exited with {:?}", status.code()));
    }
    verify_pdf(&staged)?;

    let after_images = output.with_extension("imgpass.pdf");
    let _ = std::fs::remove_file(&after_images);
    if let Ok(r) = crate::imagepass::reencode_images(&staged, &after_images, profile) {
        if r.recompressed > 0 {
            let smaller = std::fs::metadata(&after_images)
                .map(|m| m.len())
                .unwrap_or(u64::MAX)
                < std::fs::metadata(&staged).map(|m| m.len()).unwrap_or(0);
            if smaller && std::fs::rename(&after_images, &staged).is_ok() {
                verify_pdf(&staged)?;
            }
        }
    }
    let _ = std::fs::remove_file(&after_images);

    std::fs::rename(&staged, output).map_err(|e| format!("Cannot write output: {e}"))?;
    Ok(std::fs::metadata(output).map(|m| m.len()).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 合并后的验收：在**真实文档**上跑完整压缩路径（qpdf + 图像 pass），
    /// 断言页数不变、书签仍在。环境变量门控，CI 不跑（真实文档不入库）。
    ///   ROCKTIER_TEST_DIR=~/Downloads cargo test acceptance -- --nocapture
    #[test]
    fn acceptance_on_real_documents() {
        let dir = match std::env::var("ROCKTIER_TEST_DIR") {
            Ok(d) => d,
            Err(_) => return,
        };
        let out_dir = std::env::temp_dir().join("rt-pdf-acceptance");
        std::fs::create_dir_all(&out_dir).unwrap();

        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x == "pdf").unwrap_or(false))
            .collect();
        files.sort();
        assert!(!files.is_empty(), "目录里没有 PDF");

        let has_outlines = |d: &lopdf::Document| {
            d.trailer
                .get(b"Root")
                .ok()
                .and_then(|r| r.as_reference().ok())
                .and_then(|r| d.get_object(r).ok())
                .and_then(|o| o.as_dict().ok())
                .and_then(|dd| dd.get(b"Outlines").ok())
                .is_some()
        };

        for src in files {
            let name = src.file_name().unwrap().to_string_lossy().to_string();
            let out = out_dir.join(&name);
            let _ = std::fs::remove_file(&out);

            let before = std::fs::metadata(&src).unwrap().len();
            match compress(&src, &out, "balanced") {
                Ok(after) => {
                    let src_doc = lopdf::Document::load(&src).ok();
                    let out_doc = lopdf::Document::load(&out).expect("产物必须可解析");
                    let pages_before = src_doc.as_ref().map(|d| d.get_pages().len()).unwrap_or(0);
                    let pages_after = out_doc.get_pages().len();
                    assert_eq!(pages_before, pages_after, "{name}: 页数变了");

                    // 书签：原文件有，产物就必须还有
                    if src_doc.as_ref().map(&has_outlines).unwrap_or(false) {
                        assert!(has_outlines(&out_doc), "{name}: 书签丢了");
                    }
                    println!(
                        "ACCEPT {name}: {before} -> {after} (省 {}%), 页 {pages_before}->{pages_after}",
                        if before > 0 {
                            100i64 - (after as i64 * 100 / before as i64)
                        } else {
                            0
                        }
                    );
                }
                Err(e) => println!("ACCEPT {name}: 压缩失败 — {e}"),
            }
        }
    }
}
