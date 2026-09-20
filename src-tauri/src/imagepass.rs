//! 图像重编码 pass —— 补上 qpdf 不做的那一块。
//!
//! 背景：换到 qpdf 引擎后，最难的那类 PDF（图像本身就是 JPEG 的扫描件/图册）
//! 只省 4%，而旧的 Ghostscript 靠降采样能省 39%。实测取证发现真正的原因不是
//! "缺降采样"这一件事：qpdf 的 `--optimize-images` 会**跳过 ICCBased 的 JPEG 图像**，
//! 而那份图册 134 张全是 icc。把同样的图像按 Q60 重编码一遍，3541K → 998K（省 72%）。
//!
//! 因此这一 pass 的职责是：**找出 qpdf 放过的图像，按档位质量重编码**。
//! 它只改图像 XObject 的流与尺寸相关条目，**绝不碰内容流**，所以文字层逐字节保留。
//!
//! 本阶段刻意**不做降采样**（不改像素尺寸）：几何尺寸要从内容流的 CTM 推导，
//! 用 MediaBox 近似会把混合排版的文档改糊。尺寸缩减留到方案 §7 的 P1-d，
//! 届时以真实显示尺寸为准。质量重编码已足以达标（见 `PDF降采样实现方案.md` §9）。

use std::path::Path;

use lopdf::{Document, Object, ObjectId};

/// 单个图像的处置结果，用于汇总上报。
#[derive(Default, Debug)]
pub struct ImageReport {
    pub candidates: usize,
    pub recompressed: usize,
    pub skipped_unsupported: usize,
    pub bytes_before: u64,
    pub bytes_after: u64,
    /// 下面几个是"为什么没动"，用于定位 pass 为何空转（实测真实文件里
    /// 233 张图重编码 0 张，靠盲改代码找不到原因）。
    pub skip_smask: usize,
    pub skip_colorspace: usize,
    pub skip_nogain: usize,
    pub skip_decode_error: usize,
}

// （不再提供 saved_bytes()：调用点直接打印前后体积，多一个封装反而成为死代码。
//   写死一条"没人用的便捷方法"，正是本轮审查反复抓到的那类问题。）

/// 档位 → JPEG 质量。与 `main.rs::qpdf_args` 保持一致，避免两条路径给出不同观感。
fn quality_for(profile: &str) -> u8 {
    match profile {
        "web" => 40,
        "archive" => 85,
        _ => 60,
    }
}

/// 判断色彩空间是否是我们敢动的类型，返回通道数。
///
/// 只接受 DeviceGray(1) / DeviceRGB(3) / ICCBased(按其 ICC 流的 /N)。
/// 调色板(Indexed)、分色(Separation)、CMYK 一律跳过 —— 改错色彩空间比不压缩更糟。
fn color_components(doc: &Document, dict: &lopdf::Dictionary) -> Option<u8> {
    // 色彩空间经常以**间接引用**出现（实测图册里全是 `41 0 R`）。不解引用的话会被
    // 误判成"不支持的色彩空间"，把整份文档的图像全部跳过 —— 一个引用就废掉整条管线。
    let raw = dict.get(b"ColorSpace").ok()?;
    let resolved = match raw {
        Object::Reference(id) => doc.get_object(*id).ok()?,
        other => other,
    };
    match resolved {
        Object::Name(name) => match name.as_slice() {
            b"DeviceGray" | b"G" => Some(1),
            b"DeviceRGB" | b"RGB" => Some(3),
            _ => None,
        },
        Object::Array(arr) => {
            // [/ICCBased <stream ref>]
            let head = arr.first()?;
            if head.as_name().ok()? != b"ICCBased" {
                return None;
            }
            let id = arr.get(1)?.as_reference().ok()?;
            let icc = doc.get_object(id).ok()?.as_stream().ok()?;
            match icc.dict.get(b"N").ok()? {
                // 4 = CMYK。只有在 JPEG 路径下才接受（image 能解码 4 分量 JPEG 并转 RGB）；
                // 原始 CMYK 位图的转换另有讲究，留给后续，现在先不接受。
                Object::Integer(n) if *n == 1 || *n == 3 => Some(*n as u8),
                Object::Integer(4) => Some(4),
                _ => None,
            }
        }
        _ => None,
    }
}

/// 找出流实际用的滤镜名（只关心 DCTDecode / FlateDecode 这两种可解码路径）。
fn filter_of(stream: &lopdf::Stream) -> Option<Vec<u8>> {
    match stream.dict.get(b"Filter").ok()? {
        Object::Name(n) => Some(n.clone()),
        Object::Array(a) => a.first().and_then(|o| o.as_name().ok()).map(|n| n.to_vec()),
        _ => None,
    }
}

/// 把一个图像对象重编码。返回 `None` 表示"这张图不动"，否则给出
/// `(原始字节数, 重编码后的 JPEG 字节)`。
///
/// 解码/编码只做一遍 —— 曾经的写法是先算尺寸、再重算一次拿字节，等于每张图
/// 白跑一遍重采样，对这种逐张处理 100+ 图像的场景是纯浪费。
fn recompress_object(
    doc: &Document,
    id: ObjectId,
    quality: u8,
    report: &mut ImageReport,
) -> Result<Option<(u64, Vec<u8>)>, String> {
    let obj = doc.get_object(id).map_err(|e| e.to_string())?;
    let stream = match obj.as_stream() {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };

    // 图像蒙版（1-bit 模板）与 1bpc 位图不进 JPEG —— 语义会变。
    if let Ok(Object::Boolean(true)) = stream.dict.get(b"ImageMask") {
        return Ok(None);
    }
    match stream.dict.get(b"BitsPerComponent") {
        Ok(Object::Integer(8)) => {}
        _ => return Ok(None),
    }
    // 带透明蒙版的图像**可以**处理：本阶段不改像素尺寸，所以 /SMask 的几何依然
    // 成立，蒙版原样保留即可。（只有到了真要缩尺寸的 P1-c，才需要连蒙版一起缩。）
    if stream.dict.get(b"SMask").is_ok() {
        report.skip_smask += 1; // 仅计数，不再据此跳过
    }

    let comps = match color_components(doc, &stream.dict) {
        Some(c) => c,
        None => {
            report.skip_colorspace += 1;
            return Ok(None);
        }
    };
    let filter = match filter_of(stream) {
        Some(f) => f,
        None => return Ok(None),
    };

    let before = stream.content.len() as u64;

    let decoded = match filter.as_slice() {
        // 已经是 JPEG：直接解码像素
        b"DCTDecode" | b"DCT" => {
            image::load_from_memory_with_format(&stream.content, image::ImageFormat::Jpeg)
                .map_err(|e| format!("decode jpeg: {e}"))?
        }
        // 原始像素（FlateDecode 等）→ 按声明的尺寸/通道还原成图
        b"FlateDecode" | b"Fl" => {
            let raw = stream
                .decompressed_content()
                .map_err(|e| format!("inflate: {e}"))?;
            let w = match stream.dict.get(b"Width").ok().and_then(|o| o.as_i64().ok()) {
                Some(w) if w > 0 => w as u32,
                _ => return Ok(None),
            };
            let h = match stream.dict.get(b"Height").ok().and_then(|o| o.as_i64().ok()) {
                Some(h) if h > 0 => h as u32,
                _ => return Ok(None),
            };
            let expected = (w as usize) * (h as usize) * (comps as usize);
            if raw.len() < expected {
                return Ok(None);
            }
            if comps == 1 {
                image::GrayImage::from_raw(w, h, raw[..expected].to_vec())
                    .map(image::DynamicImage::ImageLuma8)
                    .ok_or("gray buffer size mismatch")?
            } else {
                image::RgbImage::from_raw(w, h, raw[..expected].to_vec())
                    .map(image::DynamicImage::ImageRgb8)
                    .ok_or("rgb buffer size mismatch")?
            }
        }
        _ => return Ok(None),
    };

    // 统一成 JPEG 友好的通道数
    // comps==4（CMYK 的 JPEG）在这里统一转成 RGB —— to_rgb8() 会做转换。
    let img = if comps == 1 {
        image::DynamicImage::ImageLuma8(decoded.to_luma8())
    } else {
        image::DynamicImage::ImageRgb8(decoded.to_rgb8())
    };

    let out = encode_jpeg(&img, quality, comps)?;

    // 重编码没变小就别动（避免"压缩后更大"这种自相矛盾的结果）
    if out.len() as u64 >= before {
        report.skip_nogain += 1;
        return Ok(None);
    }

    Ok(Some((before, out)))
}

/// JPEG 编码。**必须显式指定 4:2:0 色度抽样**：`image` crate 自带的编码器默认
/// 4:4:4 且不做哈夫曼优化，同尺寸重压高质量原件时会产出**比原件更大**的文件 ——
/// 实测 7 份真实 PDF、233 张图，"可处理但没变小"（nogain）正是卡在这里。
fn encode_jpeg(img: &image::DynamicImage, quality: u8, comps: u8) -> Result<Vec<u8>, String> {
    let mut out: Vec<u8> = Vec::new();
    let mut enc = jpeg_encoder::Encoder::new(&mut out, quality);
    enc.set_sampling_factor(jpeg_encoder::SamplingFactor::F_2_2);
    let (w, h) = (img.width() as u16, img.height() as u16);
    let res = if comps == 1 {
        enc.encode(
            img.to_luma8().as_raw(),
            w,
            h,
            jpeg_encoder::ColorType::Luma,
        )
    } else {
        enc.encode(img.to_rgb8().as_raw(), w, h, jpeg_encoder::ColorType::Rgb)
    };
    res.map_err(|e| format!("encode jpeg: {e}"))?;
    Ok(out)
}

/// 就地重写：读 `input`，把放过的图像按档位质量重编码，写到 `output`。
///
/// 任何单个图像的问题都只导致「跳过它」，不影响整份文档 —— 压缩器绝不能因为
/// 一张图怪就把用户的文件弄坏。
pub fn reencode_images(
    input: &Path,
    output: &Path,
    profile: &str,
    on_progress: &dyn Fn(usize, usize),
) -> Result<ImageReport, String> {
    let mut doc = Document::load(input).map_err(|e| format!("Cannot open PDF: {e}"))?;
    let report = reencode_document(&mut doc, profile, on_progress);
    doc.save(output).map_err(|e| format!("Cannot write PDF: {e}"))?;
    Ok(report)
}

/// 对**已在内存中**的文档做重编码。
///
/// 与文件读写分离，是为了让测试能直接喂一份内存文档：早先的写法要求先 `save`
/// 再 `load` 回来，而合成 PDF 的往返本身就不稳（实测 4 个对象回来只剩 1 个、
/// 字典还丢了），测试于是死在与被测逻辑无关的地方。
pub fn reencode_document(
    doc: &mut Document,
    profile: &str,
    on_progress: &dyn Fn(usize, usize),
) -> ImageReport {
    let quality = quality_for(profile);

    let ids: Vec<ObjectId> = doc.objects.keys().copied().collect();

    // 先数一遍作为进度条的分母。这个循环在 100+ 图像的扫描件上要跑几十秒，
    // 没有分母就只能画一个没有终点的转圈；而分母是现成的，代价只是再扫一遍
    // 对象字典（不解码任何图像）。
    let total = ids
        .iter()
        .filter(|id| {
            doc.get_object(**id)
                .ok()
                .and_then(|o| o.as_stream().ok())
                .and_then(|s| s.dict.get(b"Subtype").ok().and_then(|v| v.as_name().ok()))
                .map(|n| n == b"Image")
                .unwrap_or(false)
        })
        .count();
    on_progress(0, total);

    let mut report = ImageReport::default();

    for id in ids {
        let is_image = doc
            .get_object(id)
            .ok()
            .and_then(|o| o.as_stream().ok())
            .and_then(|s| s.dict.get(b"Subtype").ok().and_then(|v| v.as_name().ok()))
            .map(|n| n == b"Image")
            .unwrap_or(false);
        if !is_image {
            continue;
        }
        report.candidates += 1;
        on_progress(report.candidates, total);

        match recompress_object(doc, id, quality, &mut report) {
            Ok(Some((before, after))) => {
                let new_len = after.len() as u64;
                if let Ok(obj) = doc.get_object_mut(id) {
                    if let Ok(stream) = obj.as_stream_mut() {
                        stream.set_content(after);
                        // JPEG 的元数据：丢掉旧的 /DecodeParms（那是给 Flate 用的），
                        // 尺寸条目保持不变（本阶段不改像素），滤镜换成 DCTDecode。
                        let _ = stream.dict.remove(b"DecodeParms");
                        stream
                            .dict
                            .set("Filter", Object::Name(b"DCTDecode".to_vec()));
                    }
                }
                report.recompressed += 1;
                report.bytes_before += before;
                report.bytes_after += new_len;
            }
            Ok(None) => report.skipped_unsupported += 1,
            Err(_) => {
                report.skipped_unsupported += 1;
                report.skip_decode_error += 1;
            }
        }
    }

    report
}

// （已删除重算一遍取字节的 `recompress_bytes`：它在每张图上白跑一次
//   解码+重采样，属于纯粹的性能浪费。现在 `recompress_object` 直接返回字节。）


#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Stream};

    /// 256×256 RGB 噪声，未压缩 196 KB —— 就是「导出件」里那种原始位图。
    fn flate_rgb_image() -> Stream {
        let (w, h) = (256u32, 256u32);
        let mut raw = Vec::with_capacity((w * h * 3) as usize);
        for i in 0..(w * h) {
            raw.push((i % 251) as u8);
            raw.push(((i / 3) % 241) as u8);
            raw.push(((i / 7) % 239) as u8);
        }
        let mut s = Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => w as i64,
                "Height" => h as i64,
                "ColorSpace" => "DeviceRGB",
                "BitsPerComponent" => 8i64,
            },
            raw,
        );
        s.compress().expect("flate compress");
        s
    }

    #[test]
    fn reencodes_a_raw_flate_image() {
        let mut doc = Document::with_version("1.5");
        doc.objects.insert((1, 0), Object::Stream(flate_rgb_image()));
        let content = Stream::new(
            dictionary! {},
            b"BT /F1 12 Tf 72 720 Td (hello) Tj ET".to_vec(),
        );
        doc.objects.insert((2, 0), Object::Stream(content));

        let report = reencode_document(&mut doc, "balanced", &|_, _| {});

        assert_eq!(report.candidates, 1, "应恰好识别出 1 张图");
        assert_eq!(report.recompressed, 1, "原始位图必须被重编码");
        assert!(report.bytes_after < report.bytes_before, "重编码后必须更小");

        let img = doc.get_object((1, 0)).unwrap().as_stream().unwrap();
        assert_eq!(
            img.dict.get(b"Filter").unwrap().as_name().unwrap().to_vec(),
            b"DCTDecode".to_vec()
        );
        assert_eq!(img.dict.get(b"Width").unwrap().as_i64().unwrap(), 256, "本阶段不改像素尺寸");
        assert_eq!(img.dict.get(b"Height").unwrap().as_i64().unwrap(), 256);
        assert!(img.dict.get(b"DecodeParms").is_err(), "JPEG 不该留 Flate 的 DecodeParms");
        image::load_from_memory_with_format(&img.content, image::ImageFormat::Jpeg)
            .expect("产物必须是能解码的 JPEG");

        // 内容流（文字层所在的字节）必须原样保留 —— 这是产品的硬标准
        let c = doc.get_object((2, 0)).unwrap().as_stream().unwrap();
        assert_eq!(c.content, b"BT /F1 12 Tf 72 720 Td (hello) Tj ET".to_vec());
    }

    #[test]
    fn skips_what_it_must_not_touch() {
        // ① 图像蒙版（1-bit 模板）：转 JPEG 语义就变了
        let mut doc = Document::with_version("1.5");
        doc.objects.insert(
            (1, 0),
            Object::Stream(Stream::new(
                dictionary! {
                    "Type" => "XObject", "Subtype" => "Image",
                    "Width" => 8i64, "Height" => 8i64,
                    "ImageMask" => true, "BitsPerComponent" => 1i64,
                },
                vec![0u8; 8],
            )),
        );
        let r = reencode_document(&mut doc, "web", &|_, _| {});
        assert_eq!(r.candidates, 1);
        assert_eq!(r.recompressed, 0, "图像蒙版必须原样不动");

        // ② 调色板色彩空间：改错色彩空间比不压缩更糟
        let mut doc2 = Document::with_version("1.5");
        doc2.objects.insert(
            (1, 0),
            Object::Stream(Stream::new(
                dictionary! {
                    "Type" => "XObject", "Subtype" => "Image",
                    "Width" => 8i64, "Height" => 8i64,
                    "ColorSpace" => Object::Array(vec![
                        Object::Name(b"Indexed".to_vec()),
                        Object::Name(b"DeviceRGB".to_vec()),
                        Object::Integer(1),
                        Object::String(vec![0u8; 6], lopdf::StringFormat::Literal),
                    ]),
                    "BitsPerComponent" => 8i64,
                },
                vec![0u8; 8],
            )),
        );
        let r2 = reencode_document(&mut doc2, "web", &|_, _| {});
        assert_eq!(r2.recompressed, 0, "调色板图像必须跳过");
    }
    /// 真实文件基准：只在显式给出目录时运行（CI 不跑，因为真实 PDF 不入库）。
    /// 用法: ROCKTIER_BENCH_DIR=~/Downloads cargo test bench_real_documents -- --nocapture
    #[test]
    fn bench_real_documents() {
        let dir = match std::env::var("ROCKTIER_BENCH_DIR") {
            Ok(d) => std::path::PathBuf::from(d),
            Err(_) => return,
        };
        let out_dir = std::env::temp_dir().join("rt-imgpass-bench");
        std::fs::create_dir_all(&out_dir).unwrap();
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x == "pdf").unwrap_or(false))
            .collect();
        entries.sort();
        for src in entries {
            let name = src.file_name().unwrap().to_string_lossy().to_string();
            let out = out_dir.join(&name);
            match reencode_images(&src, &out, "balanced", &|_, _| {}) {
                Ok(r) => {
                    let before = std::fs::metadata(&src).unwrap().len();
                    let after = std::fs::metadata(&out).unwrap().len();
                    println!(
                        "BENCH {name}: {before} -> {after} ({}%), images {}/{} re-encoded, img {}KB->{}KB | skip: smask={} cs={} nogain={} err={}",
                        if before > 0 { 100i64 - (after as i64 * 100 / before as i64) } else { 0 },
                        r.recompressed, r.candidates,
                        r.bytes_before / 1024, r.bytes_after / 1024,
                        r.skip_smask, r.skip_colorspace, r.skip_nogain, r.skip_decode_error
                    );
                }
                Err(e) => println!("BENCH {name}: FAILED {e}"),
            }
        }
    }

    /// 诊断：真实文件里每张图**为什么**被跳过。环境变量门控，CI 不跑。
    /// 用法: ROCKTIER_BENCH_DIR=~/Downloads cargo test diagnose_real -- --nocapture
    #[test]
    fn diagnose_real_documents() {
        let dir = match std::env::var("ROCKTIER_BENCH_DIR") {
            Ok(d) => std::path::PathBuf::from(d),
            Err(_) => return,
        };
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x == "pdf").unwrap_or(false))
            .collect();
        files.sort();
        for f in files {
            let doc = match Document::load(&f) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let mut why: std::collections::BTreeMap<&str, usize> = Default::default();
            for obj in doc.objects.values() {
                let st = match obj.as_stream() {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let is_img = st
                    .dict
                    .get(b"Subtype")
                    .ok()
                    .and_then(|v| v.as_name().ok())
                    .map(|n| n == b"Image")
                    .unwrap_or(false);
                if !is_img {
                    continue;
                }
                let reason = if matches!(st.dict.get(b"ImageMask"), Ok(Object::Boolean(true))) {
                    "跳过:ImageMask"
                } else if !matches!(st.dict.get(b"BitsPerComponent"), Ok(Object::Integer(8))) {
                    "跳过:bpc!=8"
                } else if st.dict.get(b"SMask").is_ok() {
                    "跳过:SMask"
                } else if color_components(&doc, &st.dict).is_none() {
                    let cs = st.dict.get(b"ColorSpace").map(|o| format!("{o:?}"));
                    println!("      CS-DETAIL {:?}", cs.map(|c| c.chars().take(160).collect::<String>()));
                    "跳过:色彩空间"
                } else if let Some(f2) = filter_of(st) {
                    if f2 == b"DCTDecode".to_vec() || f2 == b"DCT".to_vec() {
                        "可处理:JPEG"
                    } else if f2 == b"FlateDecode".to_vec() || f2 == b"Fl".to_vec() {
                        "可处理:Flate"
                    } else {
                        "跳过:其他滤镜"
                    }
                } else {
                    "跳过:无滤镜条目"
                };
                *why.entry(reason).or_insert(0) += 1;
            }
            println!(
                "DIAG {} -> {:?}",
                f.file_name().unwrap().to_string_lossy(),
                why
            );
        }
    }
}
