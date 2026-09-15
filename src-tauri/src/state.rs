//! Application state.
//!
//! A single document is open at a time — that is the whole editing model for
//! v0.1, and it keeps the UI honest. `PdfDocument` is `Send + Sync` (pdfium
//! serialises access internally), but we still wrap it in a `Mutex` so that
//! every command sees a consistent document.

use std::sync::Mutex;

use pdfium_render::prelude::*;

pub struct OpenDoc {
    /// `'static` because it borrows the process-wide `Pdfium` instance.
    pub document: PdfDocument<'static>,
    pub path: String,
    pub dirty: bool,
}

pub struct AppState {
    pub doc: Mutex<Option<OpenDoc>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            doc: Mutex::new(None),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
