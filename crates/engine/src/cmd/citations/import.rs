//! The bibliography formats people already have: BibTeX, RIS and CSL-JSON (what Zotero, Mendeley,
//! Google Scholar, journals and LaTeX write). Reading turns every record into a [`Source`]; writing
//! turns sources back into text another tool reads. Input is hostile: sizes and counts are
//! capped, nothing indexes by an input-derived number and nothing panics.

use super::Source;
use super::render::bare_doi;
use serde_json::{Value, json};

/// Most records one import reads.
pub const MAX_RECORDS: usize = 10_000;

/// Longest field value kept, in characters (an abstract pasted into a title stays bounded).
const MAX_FIELD: usize = 4_000;

/// A bibliography text format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    BibTeX,
    Ris,
    CslJson,
}

impl Format {
    /// The format called `name` (`bibtex`, `bib`, `ris`, `csl-json`, `json`), any case.
    pub fn named(name: &str) -> Option<Format> {
        match name.trim().to_ascii_lowercase().trim_start_matches('.') {
            "bibtex" | "bib" | "biblatex" => Some(Format::BibTeX),
            "ris" => Some(Format::Ris),
            "csl-json" | "csljson" | "csl" | "json" => Some(Format::CslJson),
            _ => None,
        }
    }

    /// The format a file name's extension says.
    pub fn of_path(path: &str) -> Option<Format> {
        Format::named(path.rsplit_once('.')?.1)
    }

    pub fn name(self) -> &'static str {
        match self {
            Format::BibTeX => "bibtex",
            Format::Ris => "ris",
            Format::CslJson => "csl-json",
        }
    }
}

/// What the text is: CSL-JSON starts with `[` or `{`, BibTeX has `@type{`, RIS has a `TY  -` line.
pub fn detect(text: &str) -> Option<Format> {
    let t = text.trim_start_matches('\u{feff}').trim_start();
    if t.starts_with('[') || (t.starts_with('{') && (t.contains("\"type\"") || t.contains("\"items\""))) {
        return Some(Format::CslJson);
    }
    if t.lines().take(200).any(|l| l.trim_start().starts_with("TY  -")) {
        return Some(Format::Ris);
    }
    if t.contains('@') && t.lines().take(2000).any(|l| is_entry_start(l.trim_start())) {
        return Some(Format::BibTeX);
    }
    None
}

/// `@article{` / `@book (` / `@string{`: an at sign, letters, then a bracket.
fn is_entry_start(line: &str) -> bool {
    let Some(rest) = line.strip_prefix('@') else { return false };
    let word: String = rest.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    !word.is_empty() && rest.get(word.len()..).is_some_and(|r| r.trim_start().starts_with(['{', '(']))
}

/// The sources in `text`, in the format it is in (`None`: the text isn't any of them).
pub fn parse_any(text: &str) -> Option<(Format, Vec<Source>)> {
    let f = detect(text)?;
    Some((f, parse(text, f)))
}

pub fn parse(text: &str, format: Format) -> Vec<Source> {
    let mut out = match format {
        Format::BibTeX => parse_bibtex(text),
        Format::Ris => parse_ris(text),
        Format::CslJson => parse_csl(text),
    };
    out.truncate(MAX_RECORDS);
    // A DOI is kept as typed in the DOI box and never also as the URL.
    for s in &mut out {
        s.doi = bare_doi(&s.doi);
        if !s.doi.is_empty() && bare_doi(&s.url).eq_ignore_ascii_case(&s.doi) {
            s.url.clear();
        }
    }
    out.retain(|s| !(s.title.trim().is_empty() && s.author.trim().is_empty() && s.corporate.trim().is_empty()));
    out
}

pub fn export(sources: &[Source], format: Format) -> String {
    match format {
        Format::BibTeX => export_bibtex(sources),
        Format::Ris => export_ris(sources),
        Format::CslJson => export_csl(sources),
    }
}

fn clean(s: &str) -> String {
    let t: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    t.chars().take(MAX_FIELD).collect()
}

/// A tag usable in a `CITATION` field: letters, digits and `_-:.`; empty when nothing is left.
fn tag_of(key: &str) -> String {
    key.chars().filter(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | ':' | '.')).take(60).collect()
}

fn month_name(m: &str) -> String {
    const NAMES: [&str; 12] =
        ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    let t = m.trim().trim_end_matches('.');
    if let Ok(n) = t.parse::<usize>()
        && (1..=12).contains(&n)
    {
        return NAMES.get(n - 1).copied().unwrap_or_default().to_string();
    }
    let lower = t.to_ascii_lowercase();
    NAMES.iter().find(|full| lower.len() >= 3 && full.to_ascii_lowercase().starts_with(&lower)).map_or_else(|| t.to_string(), |full| full.to_string())
}

/// The first four-digit year in `s`.
fn year_of(s: &str) -> String {
    let b = s.as_bytes();
    (0..b.len().saturating_sub(3))
        .find(|&i| {
            let digits = b.get(i..i + 4).is_some_and(|w| w.iter().all(u8::is_ascii_digit));
            let before = i > 0 && b.get(i - 1).is_some_and(u8::is_ascii_digit);
            digits && !before && !b.get(i + 4).is_some_and(u8::is_ascii_digit)
        })
        .and_then(|i| s.get(i..i + 4))
        .unwrap_or_default()
        .to_string()
}

fn pages_of(s: &str) -> String {
    clean(&s.replace("---", "\u{2014}").replace("--", "\u{2013}"))
}

// ---------------------------------------------------------------------------------------------
// LaTeX text

/// Latin letters with an accent: `(accent command, base letters, accented letters)`, the two
/// strings of equal length.
const ACCENTS: [(char, &str, &str); 12] = [
    ('`', "aeiouAEIOU", "àèìòùÀÈÌÒÙ"),
    ('\'', "aeiouyAEIOUYcCnNsSzZlLrR", "áéíóúýÁÉÍÓÚÝćĆńŃśŚźŹĺĹŕŔ"),
    ('^', "aeiouAEIOUcCgGhHjJsSwWyY", "âêîôûÂÊÎÔÛĉĈĝĜĥĤĵĴŝŜŵŴŷŶ"),
    ('"', "aeiouyAEIOUY", "äëïöüÿÄËÏÖÜŸ"),
    ('~', "anoANOiIuU", "ãñõÃÑÕĩĨũŨ"),
    ('=', "aeiouAEIOU", "āēīōūĀĒĪŌŪ"),
    ('.', "cCeEgGzZI", "ċĊėĖġĠżŻİ"),
    ('c', "cCsStTgGkKlLnNrR", "çÇşŞţŢģĢķĶļĻņŅŗŖ"),
    ('v', "cCsSzZrReEnNdDtTaAiIoOuU", "čČšŠžŽřŘěĚňŇďĎťŤǎǍǐǏǒǑǔǓ"),
    ('u', "aAeEgGiIoOuU", "ăĂĕĔğĞĭĬŏŎŭŬ"),
    ('H', "oOuU", "őŐűŰ"),
    ('k', "aAeEiIuU", "ąĄęĘįĮųŲ"),
];

fn accented(accent: char, base: char) -> Option<char> {
    let (_, from, to) = ACCENTS.iter().find(|(a, ..)| *a == accent)?;
    let i = from.chars().position(|c| c == base)?;
    to.chars().nth(i)
}

/// A LaTeX command that is a letter or a ligature on its own: `\ss`, `\o`, `\aa`…
fn letter_command(name: &str) -> Option<&'static str> {
    Some(match name {
        "ss" => "ß",
        "o" => "ø",
        "O" => "Ø",
        "aa" => "å",
        "AA" => "Å",
        "ae" => "æ",
        "AE" => "Æ",
        "oe" => "œ",
        "OE" => "Œ",
        "l" => "ł",
        "L" => "Ł",
        "i" => "i",
        "j" => "j",
        "dh" => "ð",
        "DH" => "Ð",
        "th" => "þ",
        "TH" => "Þ",
        "textendash" => "–",
        "textemdash" => "—",
        "textquoteright" => "’",
        "textquoteleft" => "‘",
        "ldots" | "dots" | "textellipsis" => "…",
        "textasciitilde" => "~",
        "textbackslash" => "\\",
        _ => return None,
    })
}

/// LaTeX text as plain Unicode: accents, `\&`, dashes, braces and text-style commands.
pub fn unlatex(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        i += 1;
        match c {
            '{' | '}' | '$' => {}
            '~' => out.push(' '),
            '-' if chars.get(i) == Some(&'-') => {
                i += 1;
                if chars.get(i) == Some(&'-') {
                    i += 1;
                    out.push('\u{2014}');
                } else {
                    out.push('\u{2013}');
                }
            }
            '`' if chars.get(i) == Some(&'`') => {
                i += 1;
                out.push('“');
            }
            '\'' if chars.get(i) == Some(&'\'') => {
                i += 1;
                out.push('”');
            }
            '\\' => {
                let Some(&n) = chars.get(i) else { break };
                if n.is_alphabetic() {
                    let name: String = chars.iter().skip(i).take_while(|c| c.is_alphabetic()).collect();
                    i += name.chars().count();
                    if let Some(x) = letter_command(&name) {
                        out.push_str(x);
                        if chars.get(i) == Some(&'{') && chars.get(i + 1) == Some(&'}') {
                            i += 2;
                        }
                    } else if name.len() == 1 && ACCENTS.iter().any(|(a, ..)| name.starts_with(*a)) {
                        // `\c c`, `\v{s}`: skip the blank, then the letter (braced or not).
                        while chars.get(i).is_some_and(|c| c.is_whitespace()) {
                            i += 1;
                        }
                        let (base, used) = base_letter(&chars, i);
                        i += used;
                        let accent = name.chars().next().unwrap_or(' ');
                        out.push(base.and_then(|b| accented(accent, b)).or(base).unwrap_or(' '));
                    } else {
                        // `\textit{…}`, `\emph{…}`, an unknown command: its braces are dropped
                        // above and what follows stays; a blank after the name goes with it.
                        while chars.get(i).is_some_and(|c| *c == ' ') {
                            i += 1;
                        }
                    }
                } else if ACCENTS.iter().any(|(a, ..)| *a == n) {
                    i += 1;
                    let (base, used) = base_letter(&chars, i);
                    i += used;
                    out.push(base.and_then(|b| accented(n, b)).or(base).unwrap_or(n));
                } else {
                    // `\&`, `\%`, `\_`, `\#`, `\$`, `\{`: the character itself.
                    i += 1;
                    out.push(n);
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// The letter an accent applies to at `i` (`e`, `{e}`, `{\i}`) and how many characters it takes.
fn base_letter(chars: &[char], i: usize) -> (Option<char>, usize) {
    match chars.get(i) {
        Some('{') => {
            let mut j = i + 1;
            if chars.get(j) == Some(&'\\') && matches!(chars.get(j + 1), Some('i' | 'j')) {
                let b = chars.get(j + 1).copied();
                j += 2;
                if chars.get(j) == Some(&'}') {
                    j += 1;
                }
                return (b, j - i);
            }
            let b = chars.get(j).copied();
            j += 1;
            if chars.get(j) == Some(&'}') {
                j += 1;
            }
            (b, j - i)
        }
        Some('\\') if matches!(chars.get(i + 1), Some('i' | 'j')) => (chars.get(i + 1).copied(), 2),
        Some(c) => (Some(*c), 1),
        None => (None, 0),
    }
}

// ---------------------------------------------------------------------------------------------
// Names

/// `Last, First` pieces of one BibTeX name, `None` for "others" (an unwritten "et al.").
fn bibtex_name(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw == "others" {
        return None;
    }
    let parts = split_top(raw, ',');
    let (last, first) = match parts.as_slice() {
        [last] => {
            let words = split_top_ws(last);
            // "First von Last": the surname starts at the last lowercase particle before the end.
            let cut = match words.len() {
                0 | 1 => 0,
                n => {
                    let particle = words.iter().take(n - 1).rposition(|w| w.chars().next().is_some_and(char::is_lowercase));
                    particle.unwrap_or(n - 1)
                }
            };
            (words.get(cut..).unwrap_or_default().join(" "), words.get(..cut).unwrap_or_default().join(" "))
        }
        [last, first] => (last.trim().to_string(), first.trim().to_string()),
        [last, _jr, first, ..] => (last.trim().to_string(), first.trim().to_string()),
        [] => return None,
    };
    let (last, first) = (clean(&unlatex(&last)), clean(&unlatex(&first)));
    if first.is_empty() { Some(last) } else { Some(format!("{last}, {first}")) }
}

/// Split at `sep` outside braces.
fn split_top(s: &str, sep: char) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut depth = 0usize;
    for c in s.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if c == sep && depth == 0 {
            out.push(String::new());
        } else if let Some(last) = out.last_mut() {
            last.push(c);
        }
    }
    out
}

fn split_top_ws(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if c.is_whitespace() && depth == 0 {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// `A and B and C` as sources write it, `Last, First; Last, First`, and a corporate name when the
/// whole value is one braced group (`{{World Health Organization}}`).
fn bibtex_people(raw: &str) -> (String, String) {
    let raw = raw.trim();
    let mut words = split_top_ws(raw);
    if words.len() == 1 && !raw.contains(',') && raw.starts_with('{') && !raw.contains(" and ") {
        return (String::new(), clean(&unlatex(raw)));
    }
    // Split at the word `and` outside braces.
    let mut people = vec![Vec::<String>::new()];
    for w in words.drain(..) {
        if w.eq_ignore_ascii_case("and") {
            people.push(Vec::new());
        } else if let Some(last) = people.last_mut() {
            last.push(w);
        }
    }
    let names: Vec<String> = people.iter().filter_map(|p| bibtex_name(&p.join(" "))).collect();
    (names.join("; "), String::new())
}

// ---------------------------------------------------------------------------------------------
// BibTeX

/// One field value in a BibTeX entry, braces and quotes kept so names can be told from text.
struct Scan<'a> {
    c: Vec<char>,
    i: usize,
    macros: &'a mut std::collections::HashMap<String, String>,
}

impl Scan<'_> {
    fn peek(&self) -> Option<char> {
        self.c.get(self.i).copied()
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.i += 1;
        }
    }

    fn word(&mut self) -> String {
        let mut w = String::new();
        while let Some(c) = self.peek() {
            if c.is_whitespace() || matches!(c, '=' | ',' | '{' | '}' | '(' | ')' | '"' | '#') {
                break;
            }
            w.push(c);
            self.i += 1;
        }
        w
    }

    /// A braced group's inside (nesting kept), the opening brace at `self.i`.
    fn braced(&mut self) -> String {
        let mut depth = 0usize;
        let mut s = String::new();
        while let Some(c) = self.peek() {
            self.i += 1;
            match c {
                '{' => {
                    depth += 1;
                    if depth > 1 {
                        s.push(c);
                    }
                }
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return s;
                    }
                    s.push(c);
                }
                '\\' => {
                    s.push(c);
                    if let Some(n) = self.peek() {
                        s.push(n);
                        self.i += 1;
                    }
                }
                _ => s.push(c),
            }
        }
        s
    }

    fn quoted(&mut self) -> String {
        self.i += 1;
        let mut depth = 0usize;
        let mut s = String::new();
        while let Some(c) = self.peek() {
            self.i += 1;
            match c {
                '"' if depth == 0 => return s,
                '{' => {
                    depth += 1;
                    s.push(c);
                }
                '}' => {
                    depth = depth.saturating_sub(1);
                    s.push(c);
                }
                '\\' => {
                    s.push(c);
                    if let Some(n) = self.peek() {
                        s.push(n);
                        self.i += 1;
                    }
                }
                _ => s.push(c),
            }
        }
        s
    }

    /// A value: parts joined with `#`, each braced, quoted, a number or a macro name.
    fn value(&mut self) -> String {
        let mut out = String::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some('{') => out.push_str(&self.braced()),
                Some('"') => out.push_str(&self.quoted()),
                Some(_) => {
                    let w = self.word();
                    if w.is_empty() {
                        break;
                    }
                    match self.macros.get(&w.to_ascii_lowercase()) {
                        Some(v) => out.push_str(v),
                        None if w.chars().all(|c| c.is_ascii_digit()) => out.push_str(&w),
                        None => out.push_str(&month_macro(&w)),
                    }
                }
                None => break,
            }
            self.skip_ws();
            if self.peek() == Some('#') {
                self.i += 1;
            } else {
                break;
            }
        }
        out
    }
}

fn month_macro(w: &str) -> String {
    let lower = w.to_ascii_lowercase();
    match lower.as_str() {
        "jan" | "feb" | "mar" | "apr" | "may" | "jun" | "jul" | "aug" | "sep" | "oct" | "nov" | "dec" => month_name(&lower),
        _ => w.to_string(),
    }
}

fn parse_bibtex(text: &str) -> Vec<Source> {
    let mut macros = std::collections::HashMap::new();
    let mut scan = Scan { c: text.chars().collect(), i: 0, macros: &mut macros };
    let mut out = Vec::new();
    // Everything outside `@type{…}` is a comment in BibTeX.
    while let Some(&c) = scan.c.get(scan.i) {
        scan.i += 1;
        if c != '@' {
            continue;
        }
        if out.len() >= MAX_RECORDS {
            break;
        }
        scan.skip_ws();
        let kind = scan.word().to_ascii_lowercase();
        scan.skip_ws();
        let close = match scan.peek() {
            Some('{') => '}',
            Some('(') => ')',
            _ => continue,
        };
        scan.i += 1;
        match kind.as_str() {
            "comment" => {
                skip_to_close(&mut scan, close);
            }
            "preamble" => {
                let _ = scan.value();
                skip_to_close(&mut scan, close);
            }
            "string" => {
                scan.skip_ws();
                let name = scan.word().to_ascii_lowercase();
                scan.skip_ws();
                if scan.peek() == Some('=') {
                    scan.i += 1;
                    let v = scan.value();
                    scan.macros.insert(name, v);
                }
                skip_to_close(&mut scan, close);
            }
            _ => {
                scan.skip_ws();
                let key = scan.word();
                let fields = read_fields(&mut scan, close);
                out.push(bibtex_source(&kind, &key, &fields));
            }
        }
    }
    out
}

/// Skip to the bracket that closes the record just opened (`close` is `}` or `)`).
fn skip_to_close(scan: &mut Scan<'_>, close: char) {
    let open = if close == '}' { '{' } else { '(' };
    let mut depth = 1usize;
    while let Some(c) = scan.peek() {
        scan.i += 1;
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return;
            }
        }
    }
}

fn read_fields(scan: &mut Scan<'_>, close: char) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    loop {
        scan.skip_ws();
        match scan.peek() {
            None => break,
            Some(',') => {
                scan.i += 1;
            }
            Some(c) if c == close => {
                scan.i += 1;
                break;
            }
            Some(_) => {
                let name = scan.word().to_ascii_lowercase();
                scan.skip_ws();
                if name.is_empty() || scan.peek() != Some('=') {
                    // Not a field: skip the character so the loop always moves on.
                    scan.i += 1;
                    continue;
                }
                scan.i += 1;
                let v = scan.value();
                if fields.len() < 200 {
                    fields.push((name, v));
                }
            }
        }
    }
    fields
}

fn bibtex_source(kind: &str, key: &str, fields: &[(String, String)]) -> Source {
    let get = |names: &[&str]| -> String {
        names
            .iter()
            .find_map(|n| fields.iter().find(|(k, v)| k == n && !v.trim().is_empty()).map(|(_, v)| v.as_str()))
            .unwrap_or_default()
            .to_string()
    };
    let text = |names: &[&str]| clean(&unlatex(&get(names)));
    let (author, corporate) = bibtex_people(&get(&["author"]));
    let (editor, _) = bibtex_people(&get(&["editor"]));
    let container = matches!(kind, "inproceedings" | "conference" | "incollection" | "inbook");
    let journal = if container { text(&["booktitle", "journal", "journaltitle"]) } else { text(&["journal", "journaltitle", "booktitle"]) };
    let mut year = year_of(&get(&["year"]));
    let date = get(&["date"]);
    if year.is_empty() {
        year = year_of(&date);
    }
    let publisher = match kind {
        "techreport" | "report" => text(&["institution", "publisher", "organization"]),
        "phdthesis" | "mastersthesis" | "thesis" => text(&["school", "institution", "publisher"]),
        _ => text(&["publisher", "organization", "institution", "school"]),
    };
    let url = clean(&get(&["url"]));
    let kind_name = match kind {
        "article" => "article",
        "book" | "booklet" | "proceedings" | "manual" | "mvbook" => "book",
        "inbook" | "incollection" | "bookinbook" => "bookSection",
        "inproceedings" | "conference" => "conference",
        "techreport" | "report" | "phdthesis" | "mastersthesis" | "thesis" => "report",
        "misc" | "online" | "electronic" | "www" if !url.is_empty() => "website",
        _ => "other",
    };
    Source {
        tag: tag_of(key),
        kind: kind_name.into(),
        author,
        corporate,
        title: text(&["title"]),
        year,
        publisher,
        city: text(&["address", "location"]),
        journal,
        volume: text(&["volume"]),
        pages: pages_of(&unlatex(&get(&["pages"]))),
        url,
        editor,
        edition: text(&["edition"]),
        issue: text(&["number", "issue"]),
        month: month_name(&unlatex(&get(&["month"]))),
        day: String::new(),
        accessed: text(&["urldate"]),
        doi: clean(&get(&["doi"])),
    }
}

fn export_bibtex(sources: &[Source]) -> String {
    let mut out = String::new();
    let mut used = std::collections::HashSet::new();
    for s in sources {
        let ty = match s.kind.as_str() {
            "article" => "article",
            "book" => "book",
            "bookSection" => "incollection",
            "conference" => "inproceedings",
            "report" => "techreport",
            "website" => "misc",
            "periodical" => "article",
            _ => "misc",
        };
        let base = if s.tag.is_empty() { "source".to_string() } else { s.tag.clone() };
        let mut key = base.clone();
        let mut n = 1;
        while !used.insert(key.clone()) {
            n += 1;
            key = format!("{base}{n}");
        }
        out.push_str(&format!("@{ty}{{{key},\n"));
        // A corporate author is one braced name, so BibTeX doesn't split it into a surname.
        if !s.corporate.trim().is_empty() {
            out.push_str(&format!("  author = {{{{{}}}}},\n", escape_bibtex(s.corporate.trim())));
        }
        let mut put = |name: &str, v: &str| {
            if !v.trim().is_empty() {
                out.push_str(&format!("  {name} = {{{}}},\n", escape_bibtex(v.trim())));
            }
        };
        if s.corporate.trim().is_empty() {
            put("author", &s.author.split(';').map(str::trim).filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" and "));
        }
        put("editor", &s.editor.split(';').map(str::trim).filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" and "));
        put("title", &s.title);
        let container = if matches!(s.kind.as_str(), "bookSection" | "conference") { "booktitle" } else { "journal" };
        put(container, &s.journal);
        put("year", &s.year);
        put("month", &s.month);
        put("publisher", &s.publisher);
        put("address", &s.city);
        put("volume", &s.volume);
        put("number", &s.issue);
        put("pages", &s.pages.replace('\u{2013}', "--").replace('\u{2014}', "---"));
        put("edition", &s.edition);
        put("url", &s.url);
        put("doi", &s.doi);
        put("urldate", &s.accessed);
        out.push_str("}\n\n");
    }
    out
}

/// BibTeX text for a value that goes between braces: the characters BibTeX would read as markup.
fn escape_bibtex(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for c in v.chars() {
        match c {
            '&' | '%' | '_' | '#' | '$' => {
                out.push('\\');
                out.push(c);
            }
            '{' | '}' => {}
            _ => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// RIS

fn parse_ris(text: &str) -> Vec<Source> {
    let mut out = Vec::new();
    let mut rec: Vec<(String, String)> = Vec::new();
    let mut last: Option<usize> = None;
    for line in text.trim_start_matches('\u{feff}').lines() {
        let l = line.trim_end();
        let tag = l.get(..2).unwrap_or_default();
        let is_tag =
            l.len() >= 5 && l.get(2..).is_some_and(|r| r.starts_with("  -")) && tag.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
        if is_tag {
            let value = l.get(6..).unwrap_or_default().trim().to_string();
            if tag == "ER" {
                if !rec.is_empty() {
                    out.push(ris_source(&rec));
                    rec.clear();
                    if out.len() >= MAX_RECORDS {
                        return out;
                    }
                }
                last = None;
            } else {
                if rec.len() < 300 {
                    rec.push((tag.to_string(), value));
                }
                last = Some(rec.len().saturating_sub(1));
            }
        } else if !l.trim().is_empty()
            && let Some(i) = last
            && let Some((_, v)) = rec.get_mut(i)
        {
            // A wrapped value continues on the next line.
            v.push(' ');
            v.push_str(l.trim());
        }
    }
    if rec.iter().any(|(t, _)| t == "TY") {
        out.push(ris_source(&rec));
    }
    out
}

fn ris_source(rec: &[(String, String)]) -> Source {
    let all =
        |tags: &[&str]| -> Vec<String> { rec.iter().filter(|(t, v)| tags.contains(&t.as_str()) && !v.is_empty()).map(|(_, v)| v.clone()).collect() };
    let one = |tags: &[&str]| clean(&all(tags).into_iter().next().unwrap_or_default());
    let kind = match one(&["TY"]).as_str() {
        "JOUR" | "JFULL" | "EJOUR" => "article",
        "BOOK" | "EBOOK" | "EDBOOK" | "SER" => "book",
        "CHAP" | "ECHAP" => "bookSection",
        "CONF" | "CPAPER" => "conference",
        "RPRT" | "THES" => "report",
        "ELEC" | "WEB" | "BLOG" => "website",
        "MGZN" | "NEWS" => "periodical",
        _ => "other",
    };
    let people = |tags: &[&str]| all(tags).iter().map(|n| clean(n)).filter(|n| !n.is_empty()).collect::<Vec<_>>().join("; ");
    let journal_tags: &[&str] =
        if matches!(kind, "bookSection" | "conference") { &["BT", "T2", "JO", "JF", "JA"] } else { &["JO", "JF", "T2", "JA", "BT"] };
    let (sp, ep) = (one(&["SP"]), one(&["EP"]));
    let pages = match (sp.is_empty(), ep.is_empty()) {
        (false, false) => format!("{sp}\u{2013}{ep}"),
        (false, true) => sp,
        _ => String::new(),
    };
    let date = one(&["PY", "Y1", "DA"]);
    let mut month = String::new();
    let mut day = String::new();
    let da = one(&["DA", "Y1", "PY"]);
    let mut it = da.split(['/', '-']);
    if it.next().is_some() {
        if let Some(m) = it.next().filter(|m| !m.is_empty()) {
            month = month_name(m);
        }
        if let Some(d) = it.next().filter(|d| !d.is_empty()) {
            day = d.trim_start_matches('0').to_string();
        }
    }
    Source {
        tag: String::new(),
        kind: kind.into(),
        author: people(&["AU", "A1"]),
        corporate: String::new(),
        title: one(&["TI", "T1"]),
        year: year_of(&date),
        publisher: one(&["PB"]),
        city: one(&["CY"]),
        journal: one(journal_tags),
        volume: one(&["VL"]),
        pages: if pages.is_empty() { pages_of(&one(&["SP"])) } else { pages },
        url: one(&["UR", "L1", "L2"]),
        editor: people(&["A2", "ED"]),
        edition: one(&["ET"]),
        issue: one(&["IS"]),
        month,
        day,
        accessed: one(&["Y2"]),
        doi: one(&["DO", "DOI"]),
    }
}

fn export_ris(sources: &[Source]) -> String {
    let mut out = String::new();
    for s in sources {
        let ty = match s.kind.as_str() {
            "article" => "JOUR",
            "book" => "BOOK",
            "bookSection" => "CHAP",
            "conference" => "CPAPER",
            "report" => "RPRT",
            "website" => "ELEC",
            "periodical" => "MGZN",
            _ => "GEN",
        };
        let mut put = |tag: &str, v: &str| {
            if !v.trim().is_empty() {
                out.push_str(&format!("{tag}  - {}\n", v.trim()));
            }
        };
        put("TY", ty);
        if s.corporate.trim().is_empty() {
            for a in s.author.split(';') {
                put("AU", a);
            }
        } else {
            put("AU", &s.corporate);
        }
        for e in s.editor.split(';') {
            put("A2", e);
        }
        put("TI", &s.title);
        put(if matches!(s.kind.as_str(), "bookSection" | "conference") { "BT" } else { "JO" }, &s.journal);
        put("PY", &s.year);
        put("PB", &s.publisher);
        put("CY", &s.city);
        put("VL", &s.volume);
        put("IS", &s.issue);
        let (sp, ep) = s.pages.split_once(['\u{2013}', '-']).map_or((s.pages.as_str(), ""), |(a, b)| (a, b.trim_start_matches('-')));
        put("SP", sp);
        put("EP", ep);
        put("ET", &s.edition);
        put("UR", &s.url);
        put("DO", &s.doi);
        out.push_str("ER  - \n\n");
    }
    out
}

// ---------------------------------------------------------------------------------------------
// CSL-JSON

fn parse_csl(text: &str) -> Vec<Source> {
    let Ok(v) = serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) else { return Vec::new() };
    let items: Vec<&Value> = match &v {
        Value::Array(a) => a.iter().take(MAX_RECORDS).collect(),
        Value::Object(o) => match o.get("items").and_then(Value::as_array) {
            Some(a) => a.iter().take(MAX_RECORDS).collect(),
            None => vec![&v],
        },
        _ => Vec::new(),
    };
    items.into_iter().filter_map(csl_source).collect()
}

fn csl_str(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(s)) => clean(s),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// The people of a CSL name list as `Last, First; …`, and a corporate name when the only one is
/// a literal.
fn csl_people(v: &Value, key: &str) -> (String, String) {
    let Some(list) = v.get(key).and_then(Value::as_array) else { return (String::new(), String::new()) };
    if let [only] = list.as_slice()
        && only.get("family").is_none()
        && let Some(lit) = only.get("literal").and_then(Value::as_str)
    {
        return (String::new(), clean(lit));
    }
    let names: Vec<String> = list
        .iter()
        .take(500)
        .filter_map(|p| {
            let family = csl_str(p, "family");
            let given = csl_str(p, "given");
            let particle = csl_str(p, "non-dropping-particle");
            let family = if particle.is_empty() { family } else { format!("{particle} {family}") };
            match (family.is_empty(), given.is_empty()) {
                (true, true) => Some(csl_str(p, "literal")).filter(|l| !l.is_empty()),
                (false, true) => Some(family),
                (true, false) => Some(given),
                (false, false) => Some(format!("{family}, {given}")),
            }
        })
        .collect();
    (names.join("; "), String::new())
}

/// The `[year, month, day]` of a CSL date.
fn csl_date(v: &Value, key: &str) -> (String, String, String) {
    let Some(d) = v.get(key) else { return Default::default() };
    if let Some(parts) = d.get("date-parts").and_then(Value::as_array).and_then(|a| a.first()).and_then(Value::as_array) {
        let num = |i: usize| parts.get(i).map(|x| x.as_str().map(str::to_string).unwrap_or_else(|| x.to_string())).unwrap_or_default();
        let m = num(1);
        return (num(0), if m.is_empty() { m } else { month_name(&m) }, num(2));
    }
    let raw = d.get("raw").and_then(Value::as_str).or_else(|| d.as_str()).unwrap_or_default();
    (year_of(raw), String::new(), String::new())
}

fn csl_source(v: &Value) -> Option<Source> {
    v.as_object()?;
    let ty = csl_str(v, "type");
    let kind = match ty.as_str() {
        "article-journal" | "article" => "article",
        "book" | "edited-book" | "manual" => "book",
        "chapter" | "entry" | "entry-encyclopedia" | "entry-dictionary" => "bookSection",
        "paper-conference" => "conference",
        "report" | "thesis" | "dataset" => "report",
        "webpage" | "post-weblog" | "post" => "website",
        "article-magazine" | "article-newspaper" => "periodical",
        _ => "other",
    };
    let (author, corporate) = csl_people(v, "author");
    let (editor, _) = csl_people(v, "editor");
    let (year, month, day) = csl_date(v, "issued");
    let (ay, am, ad) = csl_date(v, "accessed");
    let accessed = if ay.is_empty() { String::new() } else { [ad, am, ay].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" ") };
    Some(Source {
        tag: tag_of(&csl_str(v, "id")),
        kind: kind.into(),
        author,
        corporate,
        title: csl_str(v, "title"),
        year,
        publisher: csl_str(v, "publisher"),
        city: csl_str(v, "publisher-place"),
        journal: csl_str(v, "container-title"),
        volume: csl_str(v, "volume"),
        pages: pages_of(&csl_str(v, "page")),
        url: csl_str(v, "URL"),
        editor,
        edition: csl_str(v, "edition"),
        issue: csl_str(v, "issue"),
        month,
        day,
        accessed,
        doi: csl_str(v, "DOI"),
    })
}

fn export_csl(sources: &[Source]) -> String {
    let people = |s: &str| -> Vec<Value> {
        s.split(';')
            .map(str::trim)
            .filter(|x| !x.is_empty())
            .map(|n| match n.split_once(',') {
                Some((f, g)) => json!({"family": f.trim(), "given": g.trim()}),
                None => json!({"literal": n}),
            })
            .collect()
    };
    let items: Vec<Value> = sources
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let ty = match s.kind.as_str() {
                "article" => "article-journal",
                "book" => "book",
                "bookSection" => "chapter",
                "conference" => "paper-conference",
                "report" => "report",
                "website" => "webpage",
                "periodical" => "article-magazine",
                _ => "document",
            };
            let mut o = serde_json::Map::new();
            let id = if s.tag.is_empty() { format!("source{}", i + 1) } else { s.tag.clone() };
            o.insert("id".into(), json!(id));
            o.insert("type".into(), json!(ty));
            let mut put = |k: &str, v: &str| {
                if !v.trim().is_empty() {
                    o.insert(k.into(), json!(v.trim()));
                }
            };
            put("title", &s.title);
            put("container-title", &s.journal);
            put("publisher", &s.publisher);
            put("publisher-place", &s.city);
            put("volume", &s.volume);
            put("issue", &s.issue);
            put("page", &s.pages);
            put("edition", &s.edition);
            put("URL", &s.url);
            put("DOI", &s.doi);
            let authors = if s.corporate.trim().is_empty() { people(&s.author) } else { vec![json!({"literal": s.corporate.trim()})] };
            if !authors.is_empty() {
                o.insert("author".into(), json!(authors));
            }
            let editors = people(&s.editor);
            if !editors.is_empty() {
                o.insert("editor".into(), json!(editors));
            }
            if let Ok(y) = s.year.trim().parse::<i64>() {
                o.insert("issued".into(), json!({"date-parts": [[y]]}));
            }
            Value::Object(o)
        })
        .collect();
    serde_json::to_string_pretty(&items).unwrap_or_else(|_| "[]".into())
}

#[cfg(test)]
mod tests;
