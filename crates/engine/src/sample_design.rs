//! The look of the built-in templates: one calm palette and one set of rules (accent titles,
//! hairline rules, quiet tables, a footer with page numbers) applied to a template's blocks, so
//! every template in the gallery reads as part of the same family.

use letra_doc::para::InlineObject;
use letra_doc::props::{Align, Border, BorderStyle, Borders, CharProps, LineSpacing, Rgb, TextColor};
use letra_doc::{Block, Document, Paragraph, PartKind, Table, para_block};
use std::sync::Arc;

/// The accent: a deep, calm teal.
pub(crate) const ACCENT: Rgb = Rgb(0x2B, 0x6E, 0x78);
/// The accent, very light, for bands and callouts.
pub(crate) const TINT: Rgb = Rgb(0xEC, 0xF3, 0xF4);
/// Body-text ink for headings that are not accent coloured.
const INK: Rgb = Rgb(0x22, 0x31, 0x3A);
/// Secondary text.
const DIM: Rgb = Rgb(0x6B, 0x7A, 0x82);
/// Hairlines.
const RULE: Rgb = Rgb(0xCB, 0xD5, 0xD8);

fn colour(c: Rgb) -> Option<TextColor> {
    Some(TextColor::Rgb(c))
}

/// A line of the given weight and colour, `space` points from the text.
pub(crate) fn line(width: f32, color: Rgb, space: f32) -> Border {
    Border { style: BorderStyle::Single, width, color: Some(color), space }
}

fn format_all(p: &mut Paragraph, f: &dyn Fn(&mut CharProps)) {
    let n = p.len();
    if n > 0 {
        let _ = p.format(0, n, f);
    }
}

/// Which template a paragraph is the first of, by role, so the generic pass can dress it.
fn style_of(p: &Paragraph) -> &str {
    p.props.style.as_deref().unwrap_or("")
}

fn dress_paragraph(p: &mut Paragraph) {
    match style_of(p).to_string().as_str() {
        "Title" => {
            format_all(p, &|c| {
                c.color = colour(ACCENT);
                c.bold = Some(true);
                c.size = Some(30.0);
            });
            p.props.space_after = Some(4.0);
        }
        "Subtitle" => {
            format_all(p, &|c| {
                c.color = colour(DIM);
                c.size = Some(12.0);
            });
            p.props.borders = Some(Borders { bottom: Some(line(0.75, RULE, 8.0)), ..Default::default() });
            p.props.space_after = Some(16.0);
        }
        "Heading1" => {
            format_all(p, &|c| {
                c.color = colour(ACCENT);
                c.bold = Some(true);
                c.size = Some(15.0);
            });
            p.props.space_before = Some(18.0);
            p.props.space_after = Some(5.0);
            p.props.keep_next = Some(true);
        }
        "Heading2" => {
            format_all(p, &|c| {
                c.color = colour(INK);
                c.bold = Some(true);
                c.size = Some(12.5);
            });
            p.props.space_before = Some(12.0);
            p.props.space_after = Some(3.0);
            p.props.keep_next = Some(true);
        }
        _ => {}
    }
}

/// A quiet table: an accent header row, hairlines between rows, comfortable cells, and the last
/// row tinted when `total` is set. Replaces the saturated built-in grid style.
pub(crate) fn quiet_table(t: &mut Table, total: bool) {
    t.props.style = None;
    t.props.look.header_row = false;
    t.props.look.banded_rows = false;
    t.props.look.first_column = false;
    t.props.borders = None;
    let rows = t.rows.len();
    for (r, row) in t.rows.iter_mut().enumerate() {
        let header = r == 0;
        let last = total && r + 1 == rows && rows > 1;
        for cell in &mut row.cells {
            cell.props.margins = Some([6.0, 8.0, 6.0, 8.0]);
            cell.props.shading = if header {
                Some(ACCENT)
            } else if last {
                Some(TINT)
            } else {
                None
            };
            cell.props.borders = Some(Borders { bottom: Some(line(0.5, if header { ACCENT } else { RULE }, 0.0)), ..Default::default() });
            for block in &mut cell.blocks {
                if let Block::Para(p) = Arc::make_mut(block) {
                    p.props.space_after = Some(0.0);
                    if header {
                        format_all(p, &|c| {
                            c.color = colour(Rgb(0xFF, 0xFF, 0xFF));
                            c.bold = Some(true);
                            c.size = Some(10.5);
                        });
                    } else if last {
                        format_all(p, &|c| c.bold = Some(true));
                    }
                }
            }
        }
    }
}

/// A paragraph set as a callout: a tinted band with an accent bar on its left.
pub(crate) fn callout(p: &mut Paragraph) {
    p.props.shading = Some(TINT);
    p.props.indent_left = Some(10.0);
    p.props.indent_right = Some(10.0);
    p.props.space_before = Some(6.0);
    p.props.space_after = Some(10.0);
    p.props.borders = Some(Borders {
        left: Some(line(3.0, ACCENT, 8.0)),
        top: Some(line(0.0, TINT, 6.0)),
        bottom: Some(line(0.0, TINT, 6.0)),
        ..Default::default()
    });
}

/// A centred page number in a dim footer, and optionally a running header with a hairline.
fn page_parts(d: &mut Document, header: Option<&str>) {
    let mut f = Paragraph::new().styled("Footer");
    f.props.align = Some(Align::Center);
    let _ = f.insert_object(
        0,
        InlineObject::Field { instr: "PAGE".into(), result: "1".into(), locked: false, dirty: false, code: false, form: String::new() },
        &CharProps { color: colour(DIM), size: Some(9.0), ..Default::default() },
    );
    let fid = d.add_part(PartKind::Footer, vec![para_block(f)]);
    d.last_section.footers.default = Some(fid);
    if let Some(text) = header {
        let mut h = Paragraph::with_text(text, CharProps { color: colour(DIM), size: Some(9.0), ..Default::default() }).styled("Header");
        h.props.borders = Some(Borders { bottom: Some(line(0.5, RULE, 4.0)), ..Default::default() });
        let hid = d.add_part(PartKind::Header, vec![para_block(h)]);
        d.last_section.headers.default = Some(hid);
    }
}

fn paragraphs_mut(d: &mut Document) -> impl Iterator<Item = &mut Paragraph> {
    d.body.iter_mut().filter_map(|b| match Arc::make_mut(b) {
        Block::Para(p) => Some(p),
        _ => None,
    })
}

/// Dresses the template `id`: the shared look first, then what that template adds.
pub(crate) fn polish(id: &str, d: &mut Document) {
    for p in paragraphs_mut(d) {
        dress_paragraph(p);
    }
    let total = id == "invoice";
    for b in d.body.iter_mut() {
        if let Block::Table(t) = Arc::make_mut(b) {
            quiet_table(t, total);
        }
    }
    let title = d.core.title.clone();
    match id {
        "letter" | "cover-letter" => letterhead(d),
        "resume" => resume(d),
        "report" => {
            callout_after_first_heading(d);
            page_parts(d, Some(&title));
        }
        "minutes" | "proposal" => page_parts(d, Some(&title)),
        "essay" => {
            for p in paragraphs_mut(d) {
                if style_of(p).is_empty() {
                    p.props.line_spacing = Some(LineSpacing::Multiple(1.5));
                    p.props.space_after = Some(6.0);
                }
            }
            page_parts(d, Some(&title));
        }
        "invoice" => invoice(d),
        "newsletter" => newsletter(d),
        "flyer" => flyer(d),
        "recipe" => page_parts(d, None),
        _ => {}
    }
}

/// A letter: the sender's name over a hairline, a little air before the recipient.
fn letterhead(d: &mut Document) {
    let mut contact_seen = false;
    for p in paragraphs_mut(d) {
        if style_of(p) == "Title" {
            continue;
        }
        if !contact_seen {
            contact_seen = true;
            format_all(p, &|c| {
                c.color = colour(DIM);
                c.size = Some(10.0);
            });
            p.props.borders = Some(Borders { bottom: Some(line(1.0, ACCENT, 8.0)), ..Default::default() });
            p.props.space_after = Some(18.0);
        } else if style_of(p).is_empty() {
            p.props.space_after = Some(6.0);
        }
    }
}

/// A résumé: the contact line in dim type, section headings in small caps over a hairline, and
/// each entry's role in bold ahead of its dates.
fn resume(d: &mut Document) {
    let mut contact_seen = false;
    for p in paragraphs_mut(d) {
        match style_of(p).to_string().as_str() {
            "Title" => format_all(p, &|c| c.size = Some(34.0)),
            "Heading1" => {
                format_all(p, &|c| {
                    c.small_caps = Some(true);
                    c.size = Some(13.0);
                    c.spacing = Some(0.6);
                });
                p.props.borders = Some(Borders { bottom: Some(line(0.75, RULE, 3.0)), ..Default::default() });
                p.props.space_before = Some(16.0);
            }
            _ if !contact_seen => {
                contact_seen = true;
                format_all(p, &|c| {
                    c.color = colour(DIM);
                    c.size = Some(10.5);
                });
                p.props.space_after = Some(6.0);
            }
            _ => {
                let text = p.plain_text();
                if let Some(b) = text.find('\t') {
                    let end = text[..b].chars().count();
                    if end > 0 {
                        let _ = p.format(0, end, &|c| c.bold = Some(true));
                    }
                }
                p.props.space_after = Some(4.0);
            }
        }
    }
}

/// The report's first body paragraph, the summary, as a callout.
fn callout_after_first_heading(d: &mut Document) {
    let mut seen_heading = false;
    for p in paragraphs_mut(d) {
        if style_of(p) == "Heading1" {
            if seen_heading {
                return;
            }
            seen_heading = true;
        } else if seen_heading && style_of(p).is_empty() {
            callout(p);
            return;
        }
    }
}

/// An invoice: its title large and right-aligned beside the number, From/To as quiet labels.
fn invoice(d: &mut Document) {
    for p in paragraphs_mut(d) {
        match style_of(p).to_string().as_str() {
            "Title" => {
                p.props.align = Some(Align::Right);
                format_all(p, &|c| {
                    c.size = Some(36.0);
                    c.spacing = Some(1.5);
                    c.caps = Some(true);
                });
            }
            "Subtitle" => p.props.align = Some(Align::Right),
            "Heading2" => format_all(p, &|c| {
                c.color = colour(DIM);
                c.size = Some(10.0);
                c.caps = Some(true);
                c.spacing = Some(0.8);
            }),
            _ => {}
        }
    }
}

/// A newsletter: a centred masthead between two rules, and a drop cap on the lead story.
fn newsletter(d: &mut Document) {
    let mut lead_done = false;
    let mut in_lead = false;
    for p in paragraphs_mut(d) {
        match style_of(p).to_string().as_str() {
            "Title" => {
                p.props.align = Some(Align::Center);
                p.props.borders = Some(Borders { top: Some(line(2.5, ACCENT, 10.0)), ..Default::default() });
                p.props.space_before = Some(6.0);
                format_all(p, &|c| c.size = Some(34.0));
            }
            "Subtitle" => {
                p.props.align = Some(Align::Center);
                p.props.borders = Some(Borders { bottom: Some(line(0.75, RULE, 8.0)), ..Default::default() });
            }
            "Heading1" => in_lead = !lead_done,
            "" if in_lead && !lead_done => {
                p.props.drop_cap = Some(2);
                p.props.align = Some(Align::Justify);
                lead_done = true;
            }
            _ => {}
        }
    }
}

/// A flyer: everything centred, the title large, the details set apart.
fn flyer(d: &mut Document) {
    for p in paragraphs_mut(d) {
        p.props.align = Some(Align::Center);
        match style_of(p).to_string().as_str() {
            "Title" => {
                format_all(p, &|c| {
                    c.size = Some(46.0);
                    c.bold = Some(true);
                });
                p.props.space_before = Some(48.0);
                p.props.space_after = Some(6.0);
            }
            "Subtitle" => {
                format_all(p, &|c| c.size = Some(16.0));
                p.props.borders = Some(Borders { bottom: Some(line(2.0, ACCENT, 14.0)), ..Default::default() });
                p.props.space_after = Some(28.0);
            }
            "Heading1" => {
                format_all(p, &|c| {
                    c.caps = Some(true);
                    c.size = Some(12.0);
                    c.spacing = Some(1.5);
                });
                p.props.space_before = Some(22.0);
            }
            "" => {
                format_all(p, &|c| c.size = Some(14.0));
                p.props.space_after = Some(4.0);
            }
            _ => {}
        }
    }
}
