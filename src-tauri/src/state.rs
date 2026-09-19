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
    /// Whether the file carried password protection when we opened it.
    ///
    /// pdfium does not tell us, and it never writes encryption back out. Without
    /// this flag, "save" on a protected document quietly produced an unprotected
    /// file — the user believes their file still has its password, and it does
    /// not. Recorded once at open time by reading the trailer with lopdf.
    pub was_encrypted: bool,
    pub dirty: bool,
    /// Byte snapshots taken before each mutation (newest last).
    pub undo: Vec<Vec<u8>>,
    /// Snapshots popped by undo, available again via redo (newest last).
    pub redo: Vec<Vec<u8>>,
    /// Radio groups the user explicitly cleared, by field name.
    ///
    /// pdfium cannot unselect a radio button (see `formclear.rs` for why, and
    /// what pdfium-render keeps out of reach), so the request is recorded here
    /// and applied to the file when it is written. Re-checking a group removes
    /// its name again. Stale entries are harmless: `clear_radio_groups` ignores
    /// names that do not belong to a radio group.
    pub radio_clears: std::collections::BTreeSet<String>,
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
