//! Theorem-like blocks: theorem, lemma, proposition, corollary, definition, example, remark and
//! proof, the way a mathematics or science text sets them (LaTeX's `amsthm`).
//!
//! A block is one paragraph, `Theorem 2.1 (Pythagoras). text`, whose number is a `SEQ` field (with
//! the chapter's number before it when asked, as captions do). Its style is based on Caption, so
//! every caption tool works on it with no code of its own: Insert Cross-reference lists the blocks
//! under their label, and a table of figures of that label is a list of theorems.

use super::fields::{CHAPTER_SEPARATORS, seq_format, update_seq};
use super::{Block, CharProps, CmdError, CmdResult, InlineObject, Paragraph, Path, Pos, Selection, Session, Value, p, sel_result};
use letra_doc::styles::{Style, StyleKind};
use letra_doc::{ParaProps, TextColor};

/// What a kind of block looks like.
struct Kind {
    id: &'static str,
    label: &'static str,
    /// Numbered with a `SEQ` field; a proof is not.
    numbered: bool,
    /// The text of the block is set in italics (theorems and their kin); definitions, examples
    /// and remarks stay upright.
    italic: bool,
}

const KINDS: [Kind; 8] = [
    Kind { id: "theorem", label: "Theorem", numbered: true, italic: true },
    Kind { id: "lemma", label: "Lemma", numbered: true, italic: true },
    Kind { id: "proposition", label: "Proposition", numbered: true, italic: true },
    Kind { id: "corollary", label: "Corollary", numbered: true, italic: true },
    Kind { id: "definition", label: "Definition", numbered: true, italic: false },
    Kind { id: "example", label: "Example", numbered: true, italic: false },
    Kind { id: "remark", label: "Remark", numbered: true, italic: false },
    Kind { id: "proof", label: "Proof", numbered: false, italic: false },
];

/// The mark that closes a proof (∎, "end of proof").
const QED: char = '\u{220E}';

/// The paragraph style of a block: Caption's, with the look of a theorem. Added to the document
/// when it is first used, so a file keeps it as an ordinary style of its own.
fn block_style(kind: &Kind, label: &str) -> Style {
    Style {
        id: format!("{}Block", kind.label),
        name: label.to_string(),
        kind: StyleKind::Paragraph,
        based_on: Some("Caption".into()),
        next: Some("Normal".into()),
        para: ParaProps {
            space_before: Some(10.0),
            space_after: Some(10.0),
            line_spacing: Some(letra_doc::props::LineSpacing::Multiple(1.15)),
            ..ParaProps::default()
        },
        chr: CharProps { italic: Some(kind.italic), size: Some(12.0), color: Some(TextColor::Auto), ..CharProps::default() },
        priority: Some(36),
        quick: true,
        ..Style::default()
    }
}

/// `references.theorem`: insert a theorem-like block below the caret's paragraph (or in place of
/// an empty one) and put the caret where its text goes.
pub(super) fn theorem(s: &mut Session, v: &Value) -> CmdResult {
    let name = p::str(v, "kind").unwrap_or("theorem").to_ascii_lowercase();
    let kind = KINDS.iter().find(|k| k.id == name).ok_or_else(|| {
        CmdError::Params(format!("unknown kind {name:?}; use theorem, lemma, proposition, corollary, definition, example, remark or proof"))
    })?;
    // The label as the document should read it (the interface language's word for it).
    let label: String = p::str(v, "label")
        .map(|l| l.chars().filter(|c| !matches!(c, '"' | '\\') && !c.is_control()).take(40).collect())
        .filter(|l: &String| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| kind.label.to_string());
    let title = p::str(v, "title").unwrap_or("").trim().chars().filter(|c| !c.is_control()).take(120).collect::<String>();
    let text = p::str(v, "text").unwrap_or("").to_string();
    let format = seq_format(p::str(v, "format").unwrap_or("ARABIC"));
    let chapter = p::u64(v, "chapter").filter(|l| (1..=9).contains(l));
    let sep = p::str(v, "separator").and_then(|x| CHAPTER_SEPARATORS.iter().find(|s| **s == x)).copied().unwrap_or(".");
    // `shared`: every numbered kind counts in one sequence (Theorem 1, Lemma 2, Definition 3).
    // SEQ identifiers are one word.
    let ident = if p::bool(v, "shared").unwrap_or(false) { "Theorem".to_string() } else { label.split_whitespace().collect::<Vec<_>>().join("_") };

    let style = block_style(kind, &label);
    let id = style.id.clone();
    if s.doc.styles.get(&id).is_none() {
        std::sync::Arc::make_mut(&mut s.doc.styles).upsert(style);
    }

    let bold = CharProps { bold: Some(true), ..CharProps::default() };
    let plain = CharProps::default();
    let field = |instr: String| InlineObject::Field { instr, result: String::new(), locked: false, dirty: false, code: false, form: String::new() };
    let mut para = Paragraph::new().styled(&id);
    para.insert_text(0, &label, &bold)?;
    if kind.numbered {
        para.insert_text(para.len(), " ", &bold)?;
        let mut seq = format!("SEQ {ident} \\* {format}");
        if let Some(l) = chapter {
            para.insert_object(para.len(), field(format!("STYLEREF {l} \\s")), &bold)?;
            para.insert_text(para.len(), sep, &bold)?;
            seq.push_str(&format!(" \\s {l}"));
        }
        para.insert_object(para.len(), field(seq), &bold)?;
    }
    if !title.is_empty() {
        para.insert_text(para.len(), &format!(" ({title})"), &plain)?;
    }
    para.insert_text(para.len(), ". ", &bold)?;
    let at_text = para.len();
    para.insert_text(para.len(), &text, &plain)?;
    if kind.id == "proof" && p::bool(v, "qed").unwrap_or(true) {
        para.insert_text(para.len(), &format!(" {QED}"), &plain)?;
    }

    let f = s.sel.focus.clone();
    let top = f.path.0.first().copied().unwrap_or(0) as usize;
    let at = Path::top(top.saturating_add(1));
    s.doc.insert_block(f.story, &at, Block::Para(para))?;
    update_seq(s)?;
    // The caret goes where the text starts; the fields' results are not part of the text.
    let off = at_text;
    s.sel = Selection::caret(Pos { story: f.story, path: at, off });
    sel_result(s)
}
