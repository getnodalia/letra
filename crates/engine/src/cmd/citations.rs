//! References: citations and bibliography (APA, ACM, MLA, Chicago, IEEE, Vancouver, Harvard, ISO 690,
//! Turabian, GB/T 7714, GOST, SIST02), source manager, source import and export, personal library, index, table of figures, cross-references, table
//! of authorities, footnote options.

use letra_doc::collate::{SortKey, heading_letter, sort_key};
use letra_doc::para::InlineObject;
use letra_doc::props::{CharProps, TabAlign, TabLeader, TabStop};
use letra_doc::{Block, Document, Paragraph, Path, Pos, Source, StoryRef};
use serde_json::{Value, json};

use super::references::FieldCode;
use super::{delete_selection, sel_result};
use crate::{CmdError, CmdResult, CommandSpec, Selection, Session, p};

mod cross_refs;
mod generated;
pub mod import;
pub mod library;
mod render;
mod sources;
mod styles;
mod transfer;

use cross_refs::cross_ref;
pub(crate) use cross_refs::leading_breaks;
pub use cross_refs::{caption_labels, ref_targets};
#[cfg(test)]
use generated::ENTRY_STYLE;
pub(crate) use generated::generated_list;
use generated::table_of_figures;
pub use generated::{figures_instr, update_generated};
pub use render::{bare_doi, cite, entry, entry_marked, split_marks};
pub use sources::update_citations;
use sources::{citation, sources};
pub use styles::{Cite, CiteStyle, Name, Names, Order, STYLES, find_style, style};
use transfer::{export_cmd, import_cmd, library_cmd};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("references.sources", "Manage Sources", "References › Citations & Bibliography", sources)
            .params(r#"{"add"?: Source, "remove"?: tag, "import"?: path (BibTeX, RIS, CSL-JSON or a Word bibliography file), "fromLibrary"?: tag | [tag] (copy from My Library)} → list"#),
        CommandSpec::new("references.import", "Import Sources", "References › Citations & Bibliography", import_cmd)
            .params(r#"{"path"?: file, "text"?: pasted bibliography, "format"?: "bibtex|ris|csl-json" (else from the name or the text), "to"?: "document|library|both"} → {format, found, document?, library?}; a work already there is not added twice"#),
        CommandSpec::new("references.export", "Export Sources", "References › Citations & Bibliography", export_cmd)
            .params(r#"{"format"?: "bibtex|ris|csl-json", "scope"?: "document|library", "tags"?: [tag], "path"?: file} → {format, count, text}"#)
            .pure(),
        CommandSpec::new("references.library", "My Library", "References › Citations & Bibliography", library_cmd)
            .params(r#"{"add"?: Source | tag of a document source, "edit"?: tag + "source": Source, "remove"?: tag} → {sources, tag?, new?}; the list is kept in the settings folder and shared by every document"#)
            .pure(),
        CommandSpec::new("references.citation", "Insert Citation", "References › Citations & Bibliography", citation).params(r#"{"tag"?: string, "source"?: Source (added if new), "fromLibrary"?: tag of a My Library source (added to the document if new), "pages"?: string}"#),
        CommandSpec::new("references.citationStyle", "Style", "References › Citations & Bibliography", |s, v| {
            let st = p::str(v, "style").unwrap_or("APA");
            let st = find_style(st).ok_or_else(|| {
                CmdError::Params(format!("style must be one of {:?}", STYLES.iter().map(|x| x.name).collect::<Vec<_>>()))
            })?;
            s.bib_style = st.name.to_string();
            // Recorded in the document too, so a save keeps it.
            s.doc.settings.bib_style = Some(st.name.to_string());
            update_citations(s)?;
            Ok(json!({"style": s.bib_style}))
        })
        .params(
            r#"{"style": "APA|ACM|Chicago|GB/T 7714|GOST (name sort)|GOST (title sort)|Harvard (Anglia)|Harvard (Cite Them Right)|IEEE|ISO 690 (author-date)|ISO 690 (numerical)|MLA|SIST02|Turabian|Vancouver" (Word's style names work too)}"#,
        ),
        CommandSpec::new("references.bibliography", "Bibliography", "References › Citations & Bibliography", |s, v| {
            let title = p::str(v, "title").unwrap_or(style(&s.bib_style).bib_title).to_string();
            generated_list(s, "BIBLIOGRAPHY", &title)?;
            sel_result(s)
        })
        .params(r#"{"title"?: string}"#),
        CommandSpec::new("references.tableOfFigures", "Insert Table of Figures", "References › Captions", table_of_figures).params(
            r#"{"label"?: "Figure|Table|Equation" or a caption label of your own, "includeLabel"?: bool, "pageNumbers"?: bool, "rightAlign"?: bool, "tabLeader"?: "dot|hyphen|underscore|none"}"#,
        ),
        CommandSpec::new("references.updateFigures", "Update Table", "References › Captions", |s, _| {
            update_generated(s)?;
            sel_result(s)
        }),
        CommandSpec::new("references.markCitation", "Mark Citation", "References › Table of Authorities", |s, v| {
            let entry = p::str(v, "entry").map(str::to_string).unwrap_or_else(|| s.selected_text().trim().to_string());
            if entry.is_empty() {
                return Err(CmdError::Params("select a citation or give `entry`".into()));
            }
            let (_, b) = s.sel.ordered();
            s.doc.insert_object(&b, InlineObject::Field { instr: format!("TA \\l \"{}\"", entry.replace('"', "")), result: String::new(), locked: false, dirty: false, code: false, form: String::new() }, &CharProps { hidden: Some(true), ..Default::default() })?;
            Ok(json!({"entry": entry}))
        })
        .params(r#"{"entry"?: string}"#)
        .key(super::non_mac("Alt+Shift+I")),
        CommandSpec::new("references.tableOfAuthorities", "Insert Table of Authorities", "References › Table of Authorities", |s, _| {
            generated_list(s, "TOA", "Table of Authorities")?;
            sel_result(s)
        }),
        CommandSpec::new("references.updateAuthorities", "Update Table", "References › Table of Authorities", |s, _| {
            update_generated(s)?;
            sel_result(s)
        }),
        CommandSpec::new("insert.crossReference", "Cross-reference", "Insert › Links", cross_ref)
            .params(r#"{"to": "heading|bookmark|figure|table|equation|label:<Name> (a caption label of your own)", "target": string (text / name / number), "show"?: "text|page|number|aboveBelow"} or {} to list targets"#),
        CommandSpec::new("references.noteOptions", "Footnote and Endnote", "References › Footnotes", super::references::note_options).params(
            r#"{"kind"?: "footnote|endnote", "pos"?: "pageBottom|beneathText" (footnotes) | "sectEnd|docEnd" (endnotes), "numFmt"?: "decimal|lowerRoman|upperRoman|lowerLetter|upperLetter|…", "numStart"?: 1-32767, "numRestart"?: "continuous|eachSect|eachPage" (each page: footnotes only), "scope"?: "document|section", "footnoteFormat"?: …, "endnoteFormat"?: …} — no settings: the options in effect at the caret"#,
        ),
        CommandSpec::new("references.researcher", "Researcher", "References › Research", |s, _| {
            s.status = "Researcher needs an online service; Letra keeps your documents offline.".into();
            Ok(json!({"available": false}))
        })
        .pure(),
    ]
}

#[cfg(test)]
mod tests;
