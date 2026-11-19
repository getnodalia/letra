//! Rich Text Format (`.rtf`) import and export via `nodalia-rtf`.

use letra_doc::Document;

pub fn import(bytes: &[u8]) -> Result<Document, String> {
    let shared = nodalia_rtf::doc::import(bytes)?;
    letra_docx::from_shared_model(shared).map_err(|e| e.to_string())
}

pub fn parse(bytes: &[u8]) -> Result<crate::model::Flow, String> {
    let shared = nodalia_rtf::doc::import(bytes)?;
    Ok(nodalia_flow::from_doc(&shared))
}

pub fn export(doc: &Document) -> String {
    let Ok(shared) = letra_docx::to_shared_model(doc) else {
        return String::new();
    };
    nodalia_rtf::doc::export(&shared)
}
