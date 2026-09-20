//! Build script.
//!
//! Copies only the Pdfium binary that matches the *current* target triple into
//! `resources/pdfium-runtime/`, which is what actually gets bundled. We ship
//! four platform builds in `resources/pdfium/` but bundling all of them would
//! add ~28MB to every installer — that violates "light as air".

use std::env;
use std::fs;
use std::path::Path;

/// Map a Rust target triple to our vendored directory name.
fn triple_to_dir(triple: &str) -> Option<&'static str> {
    match triple {
        t if t.starts_with("x86_64-pc-windows") => Some("windows-x64"),
        t if t.starts_with("aarch64-apple-darwin") => Some("macos-arm64"),
        t if t.starts_with("x86_64-apple-darwin") => Some("macos-x64"),
        t if t.starts_with("x86_64-unknown-linux") => Some("linux-x64"),
        _ => None,
    }
}

fn main() {
    // Stage first: `tauri_build::build()` validates every path listed in
    // `bundle.resources`, and it will hard-fail if one is missing.
    stage_pdfium();
    stage_legal_dir();

    tauri_build::build();
}

/// 确保 `resources/legal` 存在。
///
/// 它的**内容**由 `scripts/sync-legal.mjs` 放进去了，而那个脚本挂在
/// `beforeBuildCommand` —— 也就是只在 `tauri build` 时跑。**`cargo test` 与
/// `cargo check` 不会跑它**，于是 `tauri_build::build()` 在全新克隆上与 CI 里直接
/// 硬失败：`resource path "resources/legal" doesn't exist`。
///
/// 这正是 v1.0.12 那次发版失败、以及其后每一次 main 检查失败的原因（原因不在
/// 许可证内容，而在"声明了资源却没有保证它在 build script 跑之前存在"）。
///
/// 这里只建目录、不复制文件：目录存在即可通过校验，而真正的打包必然发生在
/// `sync:legal` 之后，那时里面已经是根目录那两份文件的副本（唯一事实来源不搬家）。
fn stage_legal_dir() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let dest = Path::new(&manifest_dir).join("resources").join("legal");
    if let Err(e) = fs::create_dir_all(&dest) {
        println!("cargo:warning=Cannot create resources/legal: {e}");
    }
}

fn stage_pdfium() {
    let target = env::var("TARGET").unwrap_or_default();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_default();

    println!("cargo:rerun-if-changed=resources/pdfium");

    let Some(dir) = triple_to_dir(&target) else {
        println!("cargo:warning=No vendored Pdfium for target '{target}'.");
        return;
    };
    let src = Path::new(&manifest_dir)
        .join("resources")
        .join("pdfium")
        .join(dir);
    let dest = Path::new(&manifest_dir).join("resources").join("pdfium-runtime");

    match fs::read_dir(&src) {
        Ok(entries) => {
            if let Err(e) = fs::create_dir_all(&dest) {
                println!("cargo:warning=Cannot create pdfium-runtime dir: {e}");
                return;
            }
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let Some(name) = path.file_name() else { continue };
                if let Err(e) = fs::copy(&path, dest.join(name)) {
                    println!("cargo:warning=Failed to copy {}: {e}", path.display());
                }
            }
        }
        Err(_) => {
            println!(
                "cargo:warning=Pdfium for '{dir}' not found at {}. Run `npm run fetch:pdfium`.",
                src.display()
            );
        }
    }
}
