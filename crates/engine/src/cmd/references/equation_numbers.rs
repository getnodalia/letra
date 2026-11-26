//! Numbered equations that follow the chapter, the way a mathematics text sets them: the equation
//! centred on its line and its number, `(2.3)`, at the right margin (LaTeX's `equation`
//! environment).
//!
//! The paragraph becomes `tab, equation, tab, (chapter.SEQ)` with a centre and a right tab stop at
//! the text width. The number is a `SEQ Equation` field with the chapter's number in front of it
//! when asked, as captions have it, and the style is based on Caption, so Insert Cross-reference
//! lists these equations under the label `Equation`, and a table of figures of that label is a list
//! of equations.

use super::fields::{CHAPTER_SEPARATORS, seq_format, update_seq};
use super::{CharProps, CmdError, CmdResult, InlineObject, Path, Pos, Selection, Session, Value, p, sel_result};
use letra_doc::props::{TabAlign, TabLeader, TabStop};
use letra_doc::styles::{Style, StyleKind};
use letra_doc::{ParaProps, TextColor};

/// The id of the paragraph style of a numbered equation.
const STYLE_ID: &str = "EquationBlock";

/// The paragraph style: Caption's, set upright in the size of body text.
fn block_style() -> Style {
    Style {
        id: STYLE_ID.into(),
        name: "Equation".into(),
        kind: StyleKind::Paragraph,
        based_on: Some("Caption".into()),
        next: Some("Normal".into()),
        para: ParaProps { space_before: Some(6.0), space_after: Some(6.0), ..ParaProps::default() },
        chr: CharProps { italic: Some(false), size: Some(12.0), color: Some(TextColor::Auto), ..CharProps::default() },
        priority: Some(37),
        quick: true,
        ..Style::default()
    }
}

/// The width of a column of the section that holds body block `block`: the numbers sit at its
/// right edge, so in a two-column section they sit at the column's edge and not the page's.
fn column_width(doc: &letra_doc::Document, block: usize) -> f32 {
    let sections = doc.sections();
    let section = sections.iter().find(|(end, _)| block <= *end).or(sections.last()).map(|(_, sp)| *sp);
    let Some(sp) = section else { return crate::io::text_width(doc) };
    let n = sp.columns.count.clamp(1, 12) as f32;
    ((sp.text_width() - sp.columns.space.max(0.0) * (n - 1.0)) / n).clamp(36.0, 4000.0)
}

/// `references.numberEquation`: set the equation in the caret's paragraph on its own line with a
/// number at the right margin, and put the caret after the number.
pub(super) fn number_equation(s: &mut Session, v: &Value) -> CmdResult {
    let label: String = p::str(v, "label")
        .map(|l| l.chars().filter(|c| !matches!(c, '"' | '\\') && !c.is_control()).take(40).collect::<String>())
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| "Equation".to_string());
    let ident = label.split_whitespace().collect::<Vec<_>>().join("_");
    let format = seq_format(p::str(v, "format").unwrap_or("ARABIC"));
    let chapter = p::u64(v, "chapter").filter(|l| (1..=9).contains(l));
    let sep = p::str(v, "separator").and_then(|x| CHAPTER_SEPARATORS.iter().find(|s| **s == x)).copied().unwrap_or(".");

    let f = s.sel.focus.clone();
    let at = Path::top(f.path.0.first().copied().unwrap_or(0) as usize);
    let story = f.story;
    let para = s.doc.para(story, &at).ok_or_else(|| CmdError::Params("put the caret in the paragraph with the equation".into()))?;
    let eq_off = para
        .object_offsets()
        .into_iter()
        .find(|o| matches!(para.object_at(*o), Some(InlineObject::Equation { .. })))
        .ok_or_else(|| CmdError::Params("there is no equation in this paragraph; insert one first".into()))?;
    let numbered = para
        .object_offsets()
        .into_iter()
        .any(|o| matches!(para.object_at(o), Some(InlineObject::Field { instr, .. }) if instr.trim_start().to_ascii_uppercase().starts_with("SEQ ")));
    if numbered {
        return Err(CmdError::Failed("this equation is already numbered".into()));
    }

    if s.doc.styles.get(STYLE_ID).is_none() {
        std::sync::Arc::make_mut(&mut s.doc.styles).upsert(block_style());
    }
    let width = column_width(&s.doc, at.0.first().copied().unwrap_or(0) as usize);
    let plain = CharProps::default();
    let field = |instr: String| InlineObject::Field { instr, result: String::new(), locked: false, dirty: false, code: false, form: String::new() };

    {
        let para = s.doc.para_mut(story, &at)?;
        para.props.style = Some(STYLE_ID.into());
        para.props.tabs = Some(vec![
            TabStop { pos: width / 2.0, align: TabAlign::Center, leader: TabLeader::None },
            TabStop { pos: width, align: TabAlign::Right, leader: TabLeader::None },
        ]);
        // On the tab line the equation is part of the text, not a display block of its own.
        if let Some(InlineObject::Equation { display, .. }) = para.object_at_mut(eq_off) {
            *display = false;
        }
        // The number goes at the end first, so the offsets before it stay valid.
        para.insert_text(para.len(), "\t(", &plain)?;
        let mut seq = format!("SEQ {ident} \\* {format}");
        if let Some(l) = chapter {
            para.insert_object(para.len(), field(format!("STYLEREF {l} \\s")), &plain)?;
            para.insert_text(para.len(), sep, &plain)?;
            seq.push_str(&format!(" \\s {l}"));
        }
        para.insert_object(para.len(), field(seq), &plain)?;
        para.insert_text(para.len(), ")", &plain)?;
        para.insert_text(0, "\t", &plain)?;
    }
    update_seq(s)?;
    let end = s.doc.para(story, &at).map_or(0, |p| p.len());
    s.sel = Selection::caret(Pos { story, path: at, off: end });
    sel_result(s)
}
