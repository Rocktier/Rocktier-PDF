//! Document-level password protection.
//!
//! The viewer engine (Pdfium) can *read* encrypted PDFs but cannot write
//! encryption, so both operations here go through `lopdf` — the standard
//! pure-Rust library for producing encrypted PDFs.

use std::collections::BTreeMap;
use std::sync::Arc;

use lopdf::encryption::crypt_filters::{Aes128CryptFilter, CryptFilter};
use lopdf::{Document, EncryptionState, EncryptionVersion, Permissions};
use rand::Rng;

use crate::pdf::PathResult;

/// Encryption requires a file identifier; synthesise one when a document
/// (usually a freshly created one) does not carry an `/ID` already.
fn ensure_id(doc: &mut Document) {
    if doc.trailer.get(b"ID").is_ok() {
        return;
    }

    let mut id = [0u8; 16];
    rand::rng().fill(&mut id);
    let value = lopdf::Object::Array(vec![
        lopdf::Object::String(id.to_vec(), lopdf::StringFormat::Literal),
        lopdf::Object::String(id.to_vec(), lopdf::StringFormat::Literal),
    ]);
    doc.trailer.set("ID", value);
}

/// Loads a PDF, decrypting it in place when a password is required.
fn load_decrypted(path: &str, password: &str) -> Result<Document, String> {
    let mut doc = Document::load(path).map_err(|e| format!("Cannot read PDF: {e}"))?;

    if doc.trailer.get(b"Encrypt").is_ok() {
        doc.decrypt(password)
            .map_err(|_| "PASSWORD_INCORRECT".to_string())?;
    }

    Ok(doc)
}

/// Writes an unencrypted copy of `input` to `output`.
pub fn remove_password(input: &str, output: &str, password: &str) -> Result<PathResult, String> {
    let mut doc = load_decrypted(input, password)?;

    // Drop the encryption dictionary so the copy is genuinely unprotected.
    doc.trailer.remove(b"Encrypt");

    let out = ensure_pdf(output);
    doc.save(&out).map_err(|e| format!("Cannot save PDF: {e}"))?;

    let size = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    Ok(PathResult { path: out, size })
}

/// Writes an AES-128 encrypted copy of `input` to `output`.
pub fn set_password(
    input: &str,
    output: &str,
    user_password: &str,
    owner_password: &str,
) -> Result<PathResult, String> {
    if user_password.is_empty() {
        return Err("Password must not be empty".to_string());
    }

    let mut doc = Document::load(input).map_err(|e| format!("Cannot read PDF: {e}"))?;
    if doc.trailer.get(b"Encrypt").is_ok() {
        return Err("This PDF is already encrypted".to_string());
    }

    let mut filters: BTreeMap<Vec<u8>, Arc<dyn CryptFilter>> = BTreeMap::new();
    filters.insert(b"StdCF".to_vec(), Arc::new(Aes128CryptFilter));

    ensure_id(&mut doc);

    let state = EncryptionState::try_from(EncryptionVersion::V4 {
        document: &doc,
        encrypt_metadata: true,
        crypt_filters: filters,
        stream_filter: b"StdCF".to_vec(),
        string_filter: b"StdCF".to_vec(),
        owner_password,
        user_password,
        permissions: Permissions::default(),
    })
    .map_err(|e| format!("Cannot configure encryption: {e}"))?;

    doc.encrypt(&state)
        .map_err(|e| format!("Cannot encrypt PDF: {e}"))?;

    let out = ensure_pdf(output);
    doc.save(&out).map_err(|e| format!("Cannot save PDF: {e}"))?;

    let size = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    Ok(PathResult { path: out, size })
}

fn ensure_pdf(path: &str) -> String {
    if path.to_lowercase().ends_with(".pdf") {
        path.to_string()
    } else {
        format!("{path}.pdf")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Setting a password then removing it must round-trip back to a readable,
    /// unencrypted document.
    #[test]
    fn encrypt_then_remove_password_roundtrips() {
        let dir = std::env::temp_dir().join("rocktier-pdf-editor-security");
        std::fs::create_dir_all(&dir).expect("temp dir");

        // Start from a document lopdf can produce on its own.
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let content_id = doc.add_object(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            b"BT /F1 24 Tf 72 720 Td (hello) Tj ET".to_vec(),
        ));
        let page_id = doc.add_object(lopdf::Dictionary::from_iter(vec![
            ("Type", lopdf::Object::Name(b"Page".to_vec())),
            ("Parent", lopdf::Object::Reference(pages_id)),
            (
                "MediaBox",
                lopdf::Object::Array(vec![0.into(), 0.into(), 595.into(), 842.into()]),
            ),
            (
                "Contents",
                lopdf::Object::Reference(content_id),
            ),
        ]));
        doc.objects.insert(
            pages_id,
            lopdf::Object::Dictionary(lopdf::Dictionary::from_iter(vec![
                ("Type", lopdf::Object::Name(b"Pages".to_vec())),
                ("Kids", lopdf::Object::Array(vec![page_id.into()])),
                ("Count", lopdf::Object::Integer(1)),
            ])),
        );
        let catalog_id = doc.add_object(lopdf::Dictionary::from_iter(vec![
            ("Type", lopdf::Object::Name(b"Catalog".to_vec())),
            ("Pages", lopdf::Object::Reference(pages_id)),
        ]));
        doc.trailer.set("Root", lopdf::Object::Reference(catalog_id));

        let plain = dir.join("plain.pdf");
        doc.save(&plain).expect("save plain");

        let locked = dir.join("locked.pdf");
        let unlocked = dir.join("unlocked.pdf");

        set_password(
            plain.to_str().unwrap(),
            locked.to_str().unwrap(),
            "s3cret",
            "s3cret",
        )
        .expect("set password");

        // The encrypted copy must be marked as encrypted.
        let encrypted = Document::load(&locked).expect("load encrypted");
        assert!(
            encrypted.trailer.get(b"Encrypt").is_ok(),
            "encrypted copy must carry an Encrypt dictionary"
        );
        drop(encrypted);

        remove_password(
            locked.to_str().unwrap(),
            unlocked.to_str().unwrap(),
            "s3cret",
        )
        .expect("remove password");

        let plain_again = Document::load(&unlocked).expect("load unlocked");
        assert!(
            plain_again.trailer.get(b"Encrypt").is_err(),
            "unlocked copy must not be encrypted"
        );

        // A wrong password must be rejected.
        assert!(remove_password(
            locked.to_str().unwrap(),
            unlocked.to_str().unwrap(),
            "nope"
        )
        .is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
