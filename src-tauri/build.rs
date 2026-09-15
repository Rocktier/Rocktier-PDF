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
    // `bundle.resources`, and it will hard-fail if pdfium-runtime is missing.
    stage_pdfium();

    tauri_build::build();
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
