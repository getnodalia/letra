//! Moving sources in and out: import from BibTeX, RIS, CSL-JSON or Word's bibliography file (a
//! file or pasted text), export to the same, and My Library.

use super::import::{self, Format};
use super::library::{Merged, merge};
use super::{CmdError, CmdResult, Session, Source, Value, json, p};

/// The largest bibliography file read from disk.
#[cfg(not(target_arch = "wasm32"))]
const MAX_FILE: u64 = 32 << 20;

/// The largest pasted bibliography, in bytes.
const MAX_TEXT: usize = 32 << 20;

/// Where imported sources go.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    Document,
    Library,
    Both,
}

impl Target {
    fn of(v: &Value) -> Result<Target, CmdError> {
        match p::str(v, "to").unwrap_or("document") {
            "document" | "doc" => Ok(Target::Document),
            "library" | "mine" => Ok(Target::Library),
            "both" => Ok(Target::Both),
            other => Err(CmdError::Params(format!("`to` must be document, library or both, not `{other}`"))),
        }
    }
}

fn merged_json(m: &Merged) -> Value {
    json!({"added": m.added, "duplicates": m.duplicates, "dropped": m.dropped})
}

/// What a bibliography holds, as `(format name, sources)`: BibTeX, RIS and CSL-JSON by the name
/// given, else the file's extension, else what the text looks like; a Word bibliography file
/// (`b:Sources`) by its content.
fn read_any(bytes: &[u8], hint: Option<Format>) -> Result<(&'static str, Vec<Source>), CmdError> {
    if bytes.len() > MAX_TEXT {
        return Err(CmdError::Failed("the bibliography is larger than 32 MB".into()));
    }
    let text = crate::io::decode_text(bytes);
    let head: String = text.chars().take(2000).collect();
    if head.contains("b:Sources") || head.contains("<Sources") {
        let file = letra_docx::read_sources(bytes).map_err(|e| CmdError::Failed(e.to_string()))?;
        return Ok(("word", file.sources));
    }
    let format = hint
        .or_else(|| import::detect(&text))
        .ok_or_else(|| CmdError::Failed("not a bibliography Letra reads: use BibTeX (.bib), RIS, CSL-JSON or a Word sources file".into()))?;
    Ok((format.name(), import::parse(&text, format)))
}

/// The bytes the command names: `text`, or the file at `path`.
fn input(v: &Value) -> Result<(Vec<u8>, Option<Format>), CmdError> {
    let named = match p::str(v, "format") {
        Some(f) => Some(Format::named(f).ok_or_else(|| CmdError::Params(format!("unknown format `{f}`: bibtex, ris or csl-json")))?),
        None => None,
    };
    if let Some(t) = p::str(v, "text") {
        return Ok((t.as_bytes().to_vec(), named));
    }
    let path = p::str(v, "path").ok_or_else(|| CmdError::Params("give `path` or `text`".into()))?;
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        Err(CmdError::Failed("reading files by path isn't available on the web; paste the text".into()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let bytes = crate::io::read_file_capped(path, MAX_FILE, "bibliography file").map_err(CmdError::Failed)?;
        Ok((bytes, named.or_else(|| Format::of_path(path))))
    }
}

/// Add `list` to the document's sources; the `Merged` says what happened. Tags stay unique and a
/// work the document already has is not added again.
fn into_document(s: &mut Session, list: Vec<Source>) -> Merged {
    let m = merge(&mut s.doc.sources, list, letra_docx::MAX_SOURCES);
    if !m.added.is_empty() {
        s.touch();
    }
    m
}

/// `references.import`: read sources from a file or pasted text into the document, My Library or
/// both.
pub(super) fn import_cmd(s: &mut Session, v: &Value) -> CmdResult {
    let target = Target::of(v)?;
    let (bytes, hint) = input(v)?;
    let (format, list) = read_any(&bytes, hint)?;
    if list.is_empty() {
        return Err(CmdError::Failed("no sources found in that text".into()));
    }
    let found = list.len();
    let mut out = json!({"format": format, "found": found});
    if matches!(target, Target::Library | Target::Both) {
        let m = s.library.add_all(list.clone());
        out["library"] = merged_json(&m);
        if let Err(e) = s.library.save() {
            out["libraryError"] = Value::String(e);
        }
    }
    if matches!(target, Target::Document | Target::Both) {
        out["document"] = merged_json(&into_document(s, list));
    }
    Ok(out)
}

/// What `references.sources` does with `import`: a Word bibliography file as before, and now
/// BibTeX, RIS and CSL-JSON files too. Returns an error when nothing in it could be read.
pub(super) fn import_path(s: &mut Session, path: &str) -> Result<(), CmdError> {
    let (bytes, hint) = input(&json!({"path": path}))?;
    let (_, list) = read_any(&bytes, hint)?;
    into_document(s, list);
    Ok(())
}

/// `references.sources` › `fromLibrary`: copy these sources (a tag, or a list of tags) of My
/// Library into the document. Returns the tag each has in the document: its own, or that of the
/// source the document already had for the same work.
pub(super) fn from_library(s: &mut Session, which: &Value) -> Result<Vec<String>, CmdError> {
    let tags: Vec<&str> = match which {
        Value::String(t) => vec![t.as_str()],
        Value::Array(a) => a.iter().filter_map(Value::as_str).collect(),
        _ => return Err(CmdError::Params("`fromLibrary` is a tag or a list of tags".into())),
    };
    let mut list = Vec::new();
    for t in tags {
        list.push(s.library.get(t).cloned().ok_or_else(|| CmdError::Params(format!("no source `{t}` in My Library")))?);
    }
    into_document(s, list.clone());
    Ok(list.iter().filter_map(|src| s.doc.sources.iter().find(|x| super::library::same_work(x, src)).map(|x| x.tag.clone())).collect())
}

/// `references.export`: the document's or the library's sources as BibTeX, RIS or CSL-JSON, and
/// written to `path` when given.
pub(super) fn export_cmd(s: &mut Session, v: &Value) -> CmdResult {
    let format = match p::str(v, "format") {
        Some(f) => Format::named(f).ok_or_else(|| CmdError::Params(format!("unknown format `{f}`: bibtex, ris or csl-json")))?,
        None => match p::str(v, "path").and_then(Format::of_path) {
            Some(f) => f,
            None => Format::BibTeX,
        },
    };
    let all: &[Source] = match p::str(v, "scope").unwrap_or("document") {
        "document" | "doc" => &s.doc.sources,
        "library" | "mine" => s.library.items(),
        other => return Err(CmdError::Params(format!("`scope` must be document or library, not `{other}`"))),
    };
    let chosen: Vec<Source> = match v.get("tags").and_then(Value::as_array) {
        Some(tags) => {
            let tags: Vec<&str> = tags.iter().filter_map(Value::as_str).collect();
            all.iter().filter(|x| tags.contains(&x.tag.as_str())).cloned().collect()
        }
        None => all.to_vec(),
    };
    let text = import::export(&chosen, format);
    let mut out = json!({"format": format.name(), "count": chosen.len(), "text": text});
    if let Some(path) = p::str(v, "path") {
        crate::io::write_file(path, text.as_bytes()).map_err(CmdError::Failed)?;
        out["path"] = Value::String(path.to_string());
    }
    Ok(out)
}

/// `references.library`: My Library. `add` is a source, or the tag of one of the document's;
/// `remove` a tag; `edit` a tag with `source` its new fields. Always returns the list.
pub(super) fn library_cmd(s: &mut Session, v: &Value) -> CmdResult {
    let mut out = json!({});
    if let Some(a) = v.get("add") {
        let src: Source = match a {
            Value::String(tag) => s
                .doc
                .sources
                .iter()
                .find(|x| &x.tag == tag)
                .cloned()
                .ok_or_else(|| CmdError::Params(format!("no source `{tag}` in this document")))?,
            other => serde_json::from_value(other.clone()).map_err(|e| CmdError::Params(e.to_string()))?,
        };
        match s.library.add(src) {
            Some((tag, new)) => {
                out["tag"] = Value::String(tag);
                out["new"] = Value::Bool(new);
            }
            None => return Err(CmdError::Failed("My Library is full".into())),
        }
    }
    if let (Some(tag), Some(src)) = (p::str(v, "edit"), v.get("source")) {
        let src: Source = serde_json::from_value(src.clone()).map_err(|e| CmdError::Params(e.to_string()))?;
        if !s.library.replace(tag, src) {
            return Err(CmdError::Params(format!("no source `{tag}` in My Library")));
        }
    }
    if let Some(tag) = p::str(v, "remove") {
        s.library.remove(tag);
    }
    if (v.get("add").is_some() || v.get("remove").is_some() || v.get("edit").is_some())
        && let Err(e) = s.library.save()
    {
        out["error"] = Value::String(e);
    }
    out["sources"] = serde_json::to_value(s.library.items()).unwrap_or(Value::Null);
    Ok(out)
}
