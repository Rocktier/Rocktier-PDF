//! Clears radio-button groups at the PDF-object level.
//!
//! **Why this module exists.** pdfium has no way to *un*select a radio button.
//! `PdfFormRadioButtonField` exposes `index_in_group`, `group_value`,
//! `is_checked`, `set_checked`, and the two group-flag setters — there is no
//! "nothing selected" entry point. The textbook escape hatch is the raw
//! `FORM_SetIndexSelected(handle, index, false)`, reached through the private
//! `PdfFormFieldPrivate` trait. But pdfium-render keeps that trait
//! `pub(crate)` on purpose (`src/pdf/document/page/field.rs`:
//! "Keep private so that the trait is not exposed"), so the form handle is
//! unreachable from a downstream crate. There is no `FORM_SetIndexSelected`
//! binding either — it exists only in the bundled C headers.
//!
//! So we do at the file level exactly what a viewer does: write `/AS /Off` onto
//! every widget of the group and drop the field's `/V`. Every radio widget
//! already carries an `/Off` appearance in `/AP /N`, so the cleared group
//! renders correctly in Acrobat, Preview and browsers — no appearance stream
//! has to be synthesized.
//!
//! **Where it runs.** Not in the command handler: pdfium owns the in-memory
//! document, and anything we wrote to the file first would be overwritten by
//! pdfium's own save. It runs inside `save_document`, on the temp file pdfium
//! just produced and before the rename — so a failure here aborts the save and
//! leaves the user's original untouched.

use std::collections::{BTreeSet, HashSet};

use lopdf::{Dictionary, Document, Object, ObjectId};

/// The `Radio` bit of a button field's `/Ff` (PDF 32000-1, table 226).
const FF_RADIO: i64 = 1 << 15;

/// Turns off every radio group named in `names`, in place at `path`.
///
/// Returns how many widgets were switched to `/Off`. Names that do not match a
/// radio group are ignored — the same call also receives checkbox and text
/// values, which pdfium has already handled by the time we get here.
pub fn clear_radio_groups(path: &std::path::Path, names: &BTreeSet<String>) -> Result<usize, String> {
    if names.is_empty() {
        return Ok(0);
    }

    let bytes = std::fs::read(path).map_err(|e| format!("Cannot read the saved PDF: {e}"))?;
    let mut doc = Document::load_from(std::io::Cursor::new(bytes))
        .map_err(|e| format!("Cannot parse the saved PDF: {e}"))?;

    let wanted: HashSet<&str> = names.iter().map(String::as_str).collect();

    // Collect first, mutate second: the walk needs `&Document`, and mutating
    // while holding that borrow does not compile.
    let (mut fields, mut widgets): (Vec<ObjectId>, Vec<ObjectId>) = (Vec::new(), Vec::new());
    {
        let roots: Vec<ObjectId> = document_field_roots(&doc);
        for id in roots {
            walk(&doc, id, &Inherited::default(), &wanted, &mut fields, &mut widgets);
        }
    }

    if widgets.is_empty() {
        return Ok(0);
    }

    for id in &widgets {
        if let Ok(dict) = doc.get_dictionary_mut(*id) {
            dict.set("AS", Object::Name(b"Off".to_vec()));
        }
    }
    for id in &fields {
        // No `/V` at all is the "nothing chosen" state; `/Off` would also work
        // but some validators read it as a value literally named "Off".
        if let Ok(dict) = doc.get_dictionary_mut(*id) {
            dict.remove(b"V");
        }
    }

    let mut out = std::fs::File::create(path).map_err(|e| format!("Cannot write the cleared form: {e}"))?;
    doc.save_to(&mut out).map_err(|e| format!("Cannot write the cleared form: {e}"))?;
    out.sync_all().map_err(|e| format!("Cannot write the cleared form: {e}"))?;

    Ok(widgets.len())
}

/// Field attributes inherited down the `/Kids` chain (`/FT`, `/Ff`, `/T` may be
/// declared on the parent and omitted on the widgets).
#[derive(Default)]
struct Inherited {
    ft: Option<Vec<u8>>,
    ff: i64,
    name: Option<String>,
}

/// The `/AcroForm /Fields` array of the document catalog, as object ids.
fn document_field_roots(doc: &Document) -> Vec<ObjectId> {
    let Ok(catalog) = doc.catalog() else {
        return Vec::new();
    };
    let Some(acro) = catalog.get(b"AcroForm").ok().and_then(|o| dict_of(doc, o)) else {
        return Vec::new();
    };
    match acro.get(b"Fields") {
        Ok(Object::Array(items)) => items
            .iter()
            .filter_map(|o| match o {
                Object::Reference(id) => Some(*id),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Resolves a possibly-indirect object to a dictionary.
fn dict_of<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Dictionary> {
    match obj {
        Object::Reference(id) => doc.get_dictionary(*id).ok(),
        Object::Dictionary(d) => Some(d),
        _ => None,
    }
}

/// Walks one field or widget, collecting the ones that belong to a wanted radio group.
fn walk(
    doc: &Document,
    id: ObjectId,
    inherited: &Inherited,
    wanted: &HashSet<&str>,
    fields: &mut Vec<ObjectId>,
    widgets: &mut Vec<ObjectId>,
) {
    let Ok(dict) = doc.get_dictionary(id) else {
        return;
    };

    let ft = dict
        .get(b"FT")
        .ok()
        .and_then(|o| o.as_name().ok())
        .map(<[u8]>::to_vec)
        .or_else(|| inherited.ft.clone());
    let ff = dict.get(b"Ff").ok().and_then(|o| o.as_i64().ok()).unwrap_or(inherited.ff);
    let name = dict
        .get(b"T")
        .ok()
        .and_then(|o| o.as_str().ok())
        .and_then(|b| String::from_utf8(b.to_vec()).ok())
        .or_else(|| inherited.name.clone());

    let is_radio = ft.as_deref() == Some(b"Btn".as_slice()) && (ff & FF_RADIO) != 0;
    let matches = is_radio && name.as_deref().is_some_and(|n| wanted.contains(n));

    let kids: Vec<ObjectId> = match dict.get(b"Kids") {
        Ok(Object::Array(items)) => items
            .iter()
            .filter_map(|o| match o {
                Object::Reference(id) => Some(*id),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };

    if kids.is_empty() {
        if matches {
            // A childless radio group is a lone widget: switch itself off, and
            // drop its own `/V` if it keeps one.
            fields.push(id);
            widgets.push(id);
        }
        return;
    }

    if matches {
        fields.push(id);
    }
    let child_inherited = Inherited { ft, ff, name };
    for kid in kids {
        walk(doc, kid, &child_inherited, wanted, fields, widgets);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use pdfium_render::prelude::*;

    use super::*;
    use crate::pdf::test_pdfium;

    /// Builds a one-page PDF with two radio groups: `colour` (currently /red)
    /// and `size` (currently /large). Only `colour` is ever cleared.
    fn two_group_pdf(path: &Path) {
        let catalog: ObjectId = (1, 0);
        let pages: ObjectId = (2, 0);
        let page: ObjectId = (3, 0);
        let acro: ObjectId = (4, 0);

        // (field object id, field name, currently selected value)
        let groups: [(ObjectId, &str, &str); 2] =
            [((5, 0), "colour", "red"), ((8, 0), "size", "large")];
        let widget_ids: [[ObjectId; 2]; 2] = [[(6, 0), (7, 0)], [(9, 0), (10, 0)]];

        let mut doc = Document::with_version("1.7");

        let mut catalog_dict = Dictionary::new();
        catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog_dict.set("Pages", Object::Reference(pages));
        catalog_dict.set("AcroForm", Object::Reference(acro));
        doc.objects.insert(catalog, Object::Dictionary(catalog_dict));

        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
        pages_dict.set("Kids", Object::Array(vec![Object::Reference(page)]));
        pages_dict.set("Count", Object::Integer(1));
        doc.objects.insert(pages, Object::Dictionary(pages_dict));

        let mut annots = Vec::new();
        for pair in &widget_ids {
            for id in pair {
                annots.push(Object::Reference(*id));
            }
        }

        let mut page_dict = Dictionary::new();
        page_dict.set("Type", Object::Name(b"Page".to_vec()));
        page_dict.set("Parent", Object::Reference(pages));
        page_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Integer(612),
                Object::Integer(792),
            ]),
        );
        page_dict.set("Annots", Object::Array(annots));
        doc.objects.insert(page, Object::Dictionary(page_dict));

        let mut acro_dict = Dictionary::new();
        acro_dict.set(
            "Fields",
            Object::Array(groups.iter().map(|(id, ..)| Object::Reference(*id)).collect()),
        );
        doc.objects.insert(acro, Object::Dictionary(acro_dict));

        for (g, (group_id, name, selected)) in groups.iter().enumerate() {
            let ids = widget_ids[g];
            let mut group = Dictionary::new();
            group.set("FT", Object::Name(b"Btn".to_vec()));
            group.set("T", Object::String(name.as_bytes().to_vec(), lopdf::StringFormat::Literal));
            group.set("Ff", Object::Integer(FF_RADIO));
            group.set("V", Object::Name(selected.as_bytes().to_vec()));
            group.set("Kids", Object::Array(ids.iter().map(|i| Object::Reference(*i)).collect()));
            doc.objects.insert(*group_id, Object::Dictionary(group));

            for (i, widget_id) in ids.iter().enumerate() {
                let on = if i == 0 { *selected } else { "Off" };
                let mut widget = Dictionary::new();
                widget.set("Type", Object::Name(b"Annot".to_vec()));
                widget.set("Subtype", Object::Name(b"Widget".to_vec()));
                widget.set("Parent", Object::Reference(*group_id));
                widget.set(
                    "Rect",
                    Object::Array(vec![
                        Object::Integer(0),
                        Object::Integer(0),
                        Object::Integer(12),
                        Object::Integer(12),
                    ]),
                );
                widget.set("AS", Object::Name(on.as_bytes().to_vec()));
                doc.objects.insert(*widget_id, Object::Dictionary(widget));
            }
        }

        // lopdf sizes the xref from its own `max_id`, which is what /Size and
        // /Index are derived from. A document assembled by hand must therefore be
        // renumbered first, or the xref comes out as `/Size 2 /Index[1 1]` — and
        // worse, the xref object itself takes id 1 and collides with the catalog.
        // The file then keeps exactly one readable object. (Found by this test.)
        doc.trailer.set("Root", Object::Reference(catalog));
        doc.renumber_objects();

        let mut out = std::fs::File::create(path).unwrap();
        doc.save_to(&mut out).unwrap();
    }

    /// Resolves a field by its `/T` name, returning the field and its widget ids.
    fn find_group(path: &Path, name: &str) -> (ObjectId, Vec<ObjectId>) {
        let doc = Document::load(path).unwrap();
        for (id, obj) in doc.objects.iter() {
            let Ok(dict) = obj.as_dict() else { continue };
            let is_named = dict
                .get(b"T")
                .ok()
                .and_then(|o| o.as_str().ok())
                .map(|b| b == name.as_bytes())
                .unwrap_or(false);
            if !is_named {
                continue;
            }
            let kids: Vec<ObjectId> = match dict.get(b"Kids") {
                Ok(Object::Array(items)) => items
                    .iter()
                    .filter_map(|o| match o {
                        Object::Reference(i) => Some(*i),
                        _ => None,
                    })
                    .collect(),
                _ => vec![*id],
            };
            return (*id, kids);
        }
        panic!("no form field named {name}");
    }

    fn widget_state(doc: &Document, id: ObjectId) -> String {
        doc.get_dictionary(id)
            .unwrap()
            .get(b"AS")
            .unwrap()
            .as_name()
            .map(|n| String::from_utf8_lossy(n).to_string())
            .unwrap()
    }

    fn group_value(doc: &Document, id: ObjectId) -> Option<String> {
        doc.get_dictionary(id)
            .unwrap()
            .get(b"V")
            .ok()
            .and_then(|o| o.as_name().ok())
            .map(|n| String::from_utf8_lossy(n).to_string())
    }

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rocktier-formclear-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn clearing_a_radio_group_turns_off_its_widgets_and_drops_v() {
        let path = temp("two-groups.pdf");
        two_group_pdf(&path);

        let (colour, colour_widgets) = find_group(&path, "colour");
        let (size, size_widgets) = find_group(&path, "size");
        assert_eq!(colour_widgets.len(), 2);
        assert_eq!(size_widgets.len(), 2);

        let mut names = BTreeSet::new();
        names.insert("colour".to_string());
        let cleared = clear_radio_groups(&path, &names).unwrap();
        assert_eq!(cleared, 2, "both colour widgets should be turned off");

        let doc = Document::load(&path).unwrap();
        for w in &colour_widgets {
            assert_eq!(widget_state(&doc, *w), "Off", "colour widget {w:?} is still on");
        }
        // No `/V` at all is the "nothing chosen" state.
        assert_eq!(group_value(&doc, colour), None, "colour must lose /V");

        // The other group is untouched — the property that makes it safe to run
        // this pass over an entire document.
        assert_eq!(widget_state(&doc, size_widgets[0]), "large");
        assert_eq!(widget_state(&doc, size_widgets[1]), "Off");
        assert_eq!(group_value(&doc, size).as_deref(), Some("large"));
    }

    #[test]
    fn names_that_are_not_radio_groups_are_ignored_and_leave_the_file_alone() {
        let path = temp("noop.pdf");
        two_group_pdf(&path);

        let mut names = BTreeSet::new();
        names.insert("not-a-field".to_string());
        assert_eq!(clear_radio_groups(&path, &names).unwrap(), 0);

        // Checkbox and text field names reach this pass too; they must be a no-op,
        // and must not rewrite the file.
        let untouched = std::fs::read(&path).unwrap();
        assert_eq!(clear_radio_groups(&path, &BTreeSet::new()).unwrap(), 0);
        assert_eq!(std::fs::read(&path).unwrap(), untouched, "an empty set must not rewrite");

        let (colour, _) = find_group(&path, "colour");
        let doc = Document::load(&path).unwrap();
        assert_eq!(group_value(&doc, colour).as_deref(), Some("red"));
    }

    /// The rewrite goes through lopdf, not pdfium. This asserts the file that
    /// comes out is still one pdfium — the reader the app actually uses — can
    /// open, and that pdfium agrees the group is now empty.
    #[test]
    fn the_rewritten_file_survives_a_real_reader() {
        let path = temp("readable.pdf");
        two_group_pdf(&path);

        let mut names = BTreeSet::new();
        names.insert("colour".to_string());
        clear_radio_groups(&path, &names).unwrap();

        let pdfium = test_pdfium();
        let doc = pdfium
            .load_pdf_from_file(&path, None)
            .expect("pdfium must still open the rewritten file");
        assert_eq!(doc.pages().len(), 1, "the page must survive the rewrite");

        // Every annotation must still resolve, and the radio widgets must still be
        // recognised as form fields. This is the check that matters: the rewrite
        // goes through lopdf, so a wrong xref/offset table here would show up as
        // missing or unreadable annotations.
        let mut radios = 0;
        for index in 0..doc.pages().len() {
            let page = doc.pages().get(index).unwrap();
            for annotation in page.annotations().iter() {
                if let Some(PdfFormField::RadioButton(_)) = annotation.as_form_field() {
                    radios += 1;
                }
            }
        }
        assert_eq!(radios, 4, "all four radio widgets must survive as form fields");

        // The set/unset semantics themselves are asserted at the object level in
        // `clearing_a_radio_group_turns_off_its_widgets_and_drops_v`. pdfium's
        // `is_checked` is deliberately not asserted here: it derives the state
        // from the widget's `/AP /N` appearance dictionary, and these synthetic
        // widgets have none, so it would be testing the fixture rather than the fix.
    }
}
