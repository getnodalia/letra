//! LaTeX (`.tex`) import and export via `nodalia-latex`.

use letra_doc::{Document, SectionProps};

pub use nodalia_latex::{escape_text, latex_to_linear, linear_to_latex};

pub fn import(bytes: &[u8]) -> Document {
    letra_docx::from_shared_model(nodalia_latex::import(bytes)).unwrap_or_default()
}

pub fn import_on(bytes: &[u8], page: &SectionProps) -> Document {
    let mut d = import(bytes);
    d.last_section = page.clone();
    d
}

pub fn parse(text: &str) -> nodalia_flow::Flow {
    nodalia_latex::parse(text)
}

pub fn export(doc: &Document) -> String {
    match letra_docx::to_shared_model(doc) {
        Ok(shared) => nodalia_latex::export(&shared),
        Err(_) => String::new(),
    }
}
