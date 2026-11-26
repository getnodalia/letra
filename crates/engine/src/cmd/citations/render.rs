use super::Source;
use super::styles::{Cite, Name, Names, style};

/// A source's corporate author, when it has one: written as it is in place of the person names
/// (Word's Corporate Author, #4848).
pub(super) fn corporate(src: &Source) -> Option<&str> {
    Some(src.corporate.trim()).filter(|c| !c.is_empty())
}

pub(super) fn authors(a: &str) -> Vec<(String, String)> {
    a.split(';')
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(|x| match x.split_once(',') {
            Some((last, first)) => (last.trim().to_string(), first.trim().to_string()),
            None => match x.rsplit_once(' ') {
                Some((first, last)) => (last.trim().to_string(), first.trim().to_string()),
                None => (x.to_string(), String::new()),
            },
        })
        .collect()
}

/// "Alex Mei" → "A. M." (`dots`), or "A M" without them.
fn initials(first: &str, dots: bool) -> String {
    let sep = if dots { ". " } else { " " };
    let mut out = first.split_whitespace().filter_map(|w| w.chars().next()).map(String::from).collect::<Vec<_>>().join(sep);
    if dots && !out.is_empty() {
        out.push('.');
    }
    out
}

/// "Alex Beth Mei" → "ABM": bare initials run together (Vancouver).
fn compact_initials(first: &str) -> String {
    first.split(|c: char| c.is_whitespace() || c == '-').filter_map(|w| w.chars().next()).flat_map(char::to_uppercase).collect()
}

fn name(form: Name, last: &str, first: &str) -> String {
    let join = |a: String, sep: &str, b: String| if a.is_empty() || b.is_empty() { format!("{a}{b}") } else { format!("{a}{sep}{b}") };
    match form {
        Name::LastFirst => join(last.to_string(), ", ", first.to_string()),
        Name::FirstLast => join(first.to_string(), " ", last.to_string()),
        Name::LastInitials => join(last.to_string(), ", ", initials(first, true)),
        Name::InitialsLast => join(initials(first, true), " ", last.to_string()),
        Name::LastSpaceInitials => join(last.to_string(), " ", initials(first, true)),
        Name::UpperLastFirst => join(last.to_uppercase(), ", ", first.to_string()),
        Name::UpperLastBareInitials => join(last.to_uppercase(), " ", initials(first, false)),
        Name::LastCompactInitials => join(last.to_string(), " ", compact_initials(first)),
    }
}

fn name_list(au: &[(String, String)], form: &Names) -> String {
    let shown = if form.max > 0 { au.get(..form.max).unwrap_or(au) } else { au };
    let parts: Vec<String> = shown.iter().enumerate().map(|(i, (l, f))| name(if i == 0 { form.first } else { form.rest }, l, f)).collect();
    let mut out = match parts.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [a, b] => format!("{a}{}{b}", form.two),
        [init @ .., z] => format!("{}{}{z}", init.join(form.sep), form.last),
    };
    if shown.len() < au.len() {
        out.push_str(form.et_al);
    }
    out
}

/// Starts and ends an italic span in a bibliography entry made by [`entry_marked`]: private-use
/// characters no source text holds, taken out again by [`split_marks`].
pub const ITALIC_ON: char = '\u{E000}';
pub const ITALIC_OFF: char = '\u{E001}';

/// The text of a marked entry without its marks, and the italic spans as character ranges of that
/// text (what paragraph offsets count).
pub fn split_marks(marked: &str) -> (String, Vec<(usize, usize)>) {
    let mut text = String::with_capacity(marked.len());
    let mut spans = Vec::new();
    let mut open = None;
    let mut len = 0;
    for c in marked.chars() {
        match c {
            ITALIC_ON => open = Some(len),
            ITALIC_OFF => {
                if let Some(a) = open.take()
                    && len > a
                {
                    spans.push((a, len));
                }
            }
            _ => {
                text.push(c);
                len += 1;
            }
        }
    }
    (text, spans)
}

/// Append `s`, dropping its leading full stop when `out` already ends with one ("M." + ". ").
/// An italic mark between them doesn't hide the full stop ("M.⟨/i⟩" + ". ").
fn push_text(out: &mut String, s: &str) {
    let ends_dot = out.trim_end_matches([ITALIC_ON, ITALIC_OFF]).ends_with('.');
    match s.strip_prefix('.') {
        Some(rest) if ends_dot => out.push_str(rest),
        _ => out.push_str(s),
    }
}

/// Fill one template segment; `None` when `optional` and a field in it is empty.
fn fill_segment(seg: &str, field: &dyn Fn(&str) -> String, optional: bool) -> Option<String> {
    let mut out = String::new();
    let mut rest = seg;
    while let Some((before, after)) = rest.split_once('{') {
        push_text(&mut out, before);
        let Some((key, after)) = after.split_once('}') else {
            push_text(&mut out, after);
            return Some(out);
        };
        // `{*field}` is the field in italics: its text between the two marks (see [`ITALIC_ON`]).
        let (italic, key) = match key.strip_prefix('*') {
            Some(k) => (true, k),
            None => (false, key),
        };
        let value = field(key);
        if value.is_empty() && optional {
            return None;
        }
        if italic && !value.is_empty() {
            out.push(ITALIC_ON);
            push_text(&mut out, &value);
            out.push(ITALIC_OFF);
        } else {
            push_text(&mut out, &value);
        }
        rest = after;
    }
    push_text(&mut out, rest);
    Some(out)
}

/// Fill an entry template (see [`CiteStyle`]).
fn render(template: &str, field: &dyn Fn(&str) -> String) -> String {
    let mut out = String::new();
    let mut rest = template;
    while !rest.is_empty() {
        let (seg, optional, after) = match rest.strip_prefix('⟦') {
            Some(group) => match group.split_once('⟧') {
                Some((g, after)) => (g, true, after),
                None => (group, true, ""),
            },
            None => match rest.split_once('⟦') {
                Some((plain, _)) => (plain, false, rest.get(plain.len()..).unwrap_or("")),
                None => (rest, false, ""),
            },
        };
        if let Some(text) = fill_segment(seg, field, optional) {
            push_text(&mut out, &text);
        }
        rest = after;
    }
    out
}

/// In-text citation for a style; `n` is the source's number in numbered styles.
pub fn cite(src: &Source, style_name: &str, n: usize, pages: &str) -> String {
    let st = style(style_name);
    let au = authors(&src.author);
    let lasts: Vec<&str> = au.iter().map(|a| a.0.as_str()).collect();
    let who = match (corporate(src), lasts.as_slice()) {
        (Some(c), _) => c.to_string(),
        (None, []) => src.title.clone(),
        (None, [one]) => one.to_string(),
        (None, [first, ..]) if lasts.len() >= st.et_al_from.max(2) => format!("{first} et al."),
        (None, [a, b]) => format!("{a} {} {b}", st.and),
        (None, [init @ .., z]) => format!("{} {} {z}", init.join(", "), st.and),
    };
    match st.cite {
        Cite::AuthorPage if pages.is_empty() => format!("({who})"),
        Cite::AuthorPage => format!("({who} {pages})"),
        Cite::AuthorDate { year_sep, .. } if pages.is_empty() => format!("({who}{year_sep}{})", src.year),
        Cite::AuthorDate { year_sep, pages_sep } => format!("({who}{year_sep}{}{pages_sep}{pages})", src.year),
        Cite::Number { open, close, pages_sep: Some(sep) } if !pages.is_empty() => format!("{open}{n}{sep}{pages}{close}"),
        Cite::Number { open, close, .. } => format!("{open}{n}{close}"),
    }
}

/// GB/T 7714 document type code: M book, J journal article, R report, EB/OL web page, Z other;
/// "/OL" marks a source read online.
fn gb_type(src: &Source) -> String {
    let code = match src.kind.to_ascii_lowercase().as_str() {
        "website" | "web" => return "EB/OL".into(),
        _ if !src.journal.is_empty() => "J",
        "book" | "" => "M",
        "article" => "J",
        "report" => "R",
        _ => "Z",
    };
    if src.url.is_empty() { code.into() } else { format!("{code}/OL") }
}

/// The DOI as it is written in a URL, whatever form it was typed in (`doi:10.1/x`, a
/// `doi.org` link or the bare `10.1/x`); empty when there is none.
pub fn bare_doi(doi: &str) -> String {
    let d = doi.trim();
    let lower = d.to_ascii_lowercase();
    let start = ["https://doi.org/", "http://doi.org/", "https://dx.doi.org/", "http://dx.doi.org/", "doi.org/", "doi:"]
        .iter()
        .find(|p| lower.starts_with(**p))
        .map_or(0, |p| p.len());
    d.get(start..).unwrap_or(d).trim().to_string()
}

/// The link an entry ends with: the source's URL, or the DOI as a `doi.org` link.
fn link(src: &Source) -> String {
    let url = src.url.trim();
    if !url.is_empty() {
        return url.to_string();
    }
    match bare_doi(&src.doi) {
        d if d.is_empty() => String::new(),
        d => format!("https://doi.org/{d}"),
    }
}

/// Bibliography entry for a style, as plain text; `n` is the source's number in numbered styles.
pub fn entry(src: &Source, style_name: &str, n: usize) -> String {
    split_marks(&entry_marked(src, style_name, n)).0
}

/// Bibliography entry for a style with its italic spans marked ([`split_marks`] reads them).
pub fn entry_marked(src: &Source, style_name: &str, n: usize) -> String {
    let st = style(style_name);
    let au = authors(&src.author);
    let editors = authors(&src.editor);
    let field = |key: &str| -> String {
        match key {
            "n" => n.to_string(),
            "names" => corporate(src).map_or_else(|| name_list(&au, &st.names), str::to_string),
            "editors" => name_list(&editors, &st.names),
            "year" if src.year.is_empty() => "n.d.".into(),
            "year" => src.year.clone(),
            "title" => src.title.clone(),
            "city" => src.city.clone(),
            "publisher" => src.publisher.clone(),
            "pub" => [src.city.as_str(), src.publisher.as_str()].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(": "),
            "journal" => src.journal.clone(),
            "volume" => src.volume.clone(),
            "issue" => src.issue.clone(),
            "edition" => src.edition.clone(),
            "month" => src.month.clone(),
            "accessed" => src.accessed.clone(),
            "pages" => src.pages.clone(),
            "url" => src.url.clone(),
            "doi" => bare_doi(&src.doi),
            "link" => link(src),
            "type" => gb_type(src),
            _ => String::new(),
        }
    };
    render(if src.journal.is_empty() { st.book } else { st.article }, &field)
}
