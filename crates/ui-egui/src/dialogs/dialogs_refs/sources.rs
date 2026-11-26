use super::shell::{Next, go};
#[cfg(not(target_arch = "wasm32"))]
use super::{AfterPick, Asked, FileDialogRequest};
use super::{RefsDialog, Serialize, Source, Tokens, Ui, Value, WordApp, buttons, json, vec2};
use egui::text::{LayoutJob, TextFormat};
use letra_engine::cmd::citations::{cite, entry_marked, split_marks};

// ---------------------------------------------------------------------------------------------
// Sources

/// Source types with their fields: (kind, label, recommended fields). A field is (key, label).
const SOURCE_TYPES: [(&str, &str, &[(&str, &str)]); 9] = [
    ("book", "Book", &[("author", "Author"), ("title", "Title"), ("year", "Year"), ("city", "City"), ("publisher", "Publisher")]),
    (
        "bookSection",
        "Chapter in a book",
        &[
            ("author", "Author"),
            ("title", "Title"),
            ("journal", "Book title"),
            ("year", "Year"),
            ("pages", "Pages"),
            ("city", "City"),
            ("publisher", "Publisher"),
        ],
    ),
    (
        "article",
        "Journal article",
        &[("author", "Author"), ("title", "Title"), ("journal", "Journal"), ("year", "Year"), ("volume", "Volume"), ("pages", "Pages")],
    ),
    (
        "periodical",
        "Magazine or newspaper article",
        &[
            ("author", "Author"),
            ("title", "Title"),
            ("journal", "Periodical"),
            ("year", "Year"),
            ("month", "Month"),
            ("day", "Day"),
            ("pages", "Pages"),
        ],
    ),
    (
        "conference",
        "Conference paper",
        &[
            ("author", "Author"),
            ("title", "Title"),
            ("journal", "Conference"),
            ("year", "Year"),
            ("pages", "Pages"),
            ("city", "City"),
            ("publisher", "Publisher"),
        ],
    ),
    ("report", "Report", &[("author", "Author"), ("title", "Title"), ("year", "Year"), ("publisher", "Institution"), ("city", "City")]),
    (
        "website",
        "Website",
        &[
            ("author", "Author"),
            ("title", "Page title"),
            ("journal", "Site name"),
            ("year", "Year"),
            ("month", "Month"),
            ("day", "Day"),
            ("url", "URL"),
        ],
    ),
    ("film", "Film", &[("author", "Author"), ("title", "Title"), ("publisher", "Studio"), ("year", "Year")]),
    ("other", "Other", &[("author", "Author"), ("title", "Title"), ("year", "Year"), ("city", "City"), ("publisher", "Publisher")]),
];

/// Every field "show all bibliography fields" adds, after the type's own.
const ALL_FIELDS: [(&str, &str); 17] = [
    ("author", "Author"),
    ("editor", "Editor"),
    ("title", "Title"),
    ("journal", "Container Title"),
    ("year", "Year"),
    ("month", "Month"),
    ("day", "Day"),
    ("city", "City"),
    ("publisher", "Publisher"),
    ("edition", "Edition"),
    ("volume", "Volume"),
    ("issue", "Issue"),
    ("pages", "Pages"),
    ("url", "URL"),
    ("doi", "DOI"),
    ("accessed", "Accessed on"),
    ("tag", "Tag"),
];

pub(super) fn field_mut<'a>(s: &'a mut Source, key: &str) -> Option<&'a mut String> {
    Some(match key {
        "author" => &mut s.author,
        "editor" => &mut s.editor,
        "title" => &mut s.title,
        "journal" => &mut s.journal,
        "year" => &mut s.year,
        "month" => &mut s.month,
        "day" => &mut s.day,
        "city" => &mut s.city,
        "publisher" => &mut s.publisher,
        "edition" => &mut s.edition,
        "volume" => &mut s.volume,
        "issue" => &mut s.issue,
        "pages" => &mut s.pages,
        "url" => &mut s.url,
        "doi" => &mut s.doi,
        "accessed" => &mut s.accessed,
        "tag" => &mut s.tag,
        "corporate" => &mut s.corporate,
        _ => return None,
    })
}

/// Create Source / Edit Source.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceForm {
    pub source: Source,
    pub show_all: bool,
    /// Editing a source that is there (its tag stays: citations point at it).
    pub edit: bool,
    /// Insert a citation of the source on OK (Insert Citation › Add New Source…).
    pub cite: bool,
    /// Opened from the Source Manager, which comes back when this closes.
    pub back: bool,
    /// Corporate Author is checked: the Author box holds one organisation's name (#4848).
    pub corporate: bool,
    /// The source belongs to My Library, not to this document.
    pub lib: bool,
}

impl SourceForm {
    pub fn new(cite: bool, back: bool) -> SourceForm {
        SourceForm::of(Source { kind: "book".into(), ..Default::default() }, false, cite, back)
    }

    /// The form for `source`; Corporate Author starts checked when the source has one.
    pub fn of(source: Source, edit: bool, cite: bool, back: bool) -> SourceForm {
        let corporate = !source.corporate.trim().is_empty();
        SourceForm { source, show_all: false, edit, cite, back, corporate, lib: false }
    }

    /// The same form for a source of My Library.
    pub fn in_library(mut self) -> SourceForm {
        self.lib = true;
        self
    }

    /// The source as OK adds it: the person authors or the corporate author, whichever box is
    /// shown, never both.
    pub fn finished(&self) -> Source {
        let mut src = self.source.clone();
        if self.corporate {
            src.author.clear();
        } else {
            src.corporate.clear();
        }
        src
    }

    /// The fields shown: the type's own, or every field.
    pub fn fields(&self) -> Vec<(&'static str, &'static str)> {
        let own = SOURCE_TYPES.iter().find(|(k, ..)| *k == self.source.kind).map(|(_, _, f)| *f).unwrap_or(SOURCE_TYPES[0].2);
        let mut v: Vec<(&str, &str)> = own.to_vec();
        if self.show_all {
            v.extend(ALL_FIELDS.iter().filter(|(k, _)| !own.iter().any(|(o, _)| o == k) && *k != "tag").copied());
        }
        v.push(("tag", "Tag"));
        v
    }
}

/// Draws a source's type and fields, optionally every bibliography field, and adds it as a
/// bibliography source or inserts a citation on OK.
pub(super) fn source_ui(app: &mut WordApp, ui: &mut Ui, f: &mut SourceForm) -> Next {
    crate::rtl::Table::new("src").columns(2).spacing(vec2(10.0, 5.0)).show(ui, |t| {
        t.row(|r| {
            let kind_label = r.cell(|ui| ui.label(tl!("Source type:")));
            r.cell(|ui| {
                let shown = SOURCE_TYPES.iter().find(|(k, ..)| *k == f.source.kind).map(|(_, l, _)| tl!(l).to_string()).unwrap_or_default();
                let combo = nodalia_ui::ComboBox::from_id_salt("src_kind").selected_text(shown).width(200.0).show_ui(ui, |ui| {
                    for (k, l, _) in SOURCE_TYPES {
                        ui.selectable_value(&mut f.source.kind, k.to_string(), tl!(l));
                    }
                });
                combo.response.labelled_by(kind_label.id);
            });
        });
        let edit = f.edit;
        let corporate = f.corporate;
        for (key, label) in f.fields() {
            // Corporate Author swaps the Author box for one organisation's name (#4848).
            let (key, label) = if key == "author" && corporate { ("corporate", "Corporate Author") } else { (key, label) };
            t.row(|r| {
                let field_label = r.cell(|ui| ui.label(format!("{}:", tl!(label))));
                r.cell(|ui| {
                    if let Some(v) = field_mut(&mut f.source, key) {
                        let hint = match key {
                            "author" => tl!("Last, First; Last, First"),
                            "tag" => tl!("Made from the author and year when empty"),
                            _ => "",
                        };
                        ui.add_enabled(!(edit && key == "tag"), egui::TextEdit::singleline(v).desired_width(260.0).hint_text(hint))
                            .labelled_by(field_label.id);
                    }
                });
            });
            if matches!(key, "author" | "corporate") {
                t.row(|r| {
                    r.cell(|ui| ui.label(""));
                    r.cell(|ui| ui.checkbox(&mut f.corporate, tl!("Corporate Author")));
                });
            }
        }
    });
    ui.checkbox(&mut f.show_all, tl!("Show every bibliography field"));
    let (ok, cancel) = buttons(ui, tl!("OK"));
    if ok {
        let finished = f.finished();
        let src = serde_json::to_value(&finished).unwrap_or(Value::Null);
        let r = if f.lib && f.edit {
            app.run("references.library", json!({"edit": finished.tag, "source": src}))
        } else if f.lib {
            app.run("references.library", json!({"add": src}))
        } else if f.cite {
            app.run("references.citation", json!({"source": src}))
        } else {
            app.run("references.sources", json!({"add": src}))
        };
        if let Err(e) = r {
            app.status(e);
        }
    }
    if (ok || cancel) && f.back {
        return go(RefsDialog::Sources(SourcesView { library: f.lib, ..Default::default() }));
    }
    if ok || cancel { Next::Close } else { Next::Stay }
}

/// What the Source Manager shows: this document's sources or My Library, filtered by a search,
/// with the selected source's formatted entry and citation beside the list.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcesView {
    /// My Library instead of this document's list.
    pub library: bool,
    /// The selected source, by tag (a search keeps it selected while the list changes).
    pub selected: Option<String>,
    pub query: String,
    /// The paste-to-import box, open when there is text (an empty string is open and empty).
    pub paste: Option<String>,
}

/// Whether `src` matches the search `q` (every word of it, in any field).
fn matches_query(src: &Source, q: &str) -> bool {
    let hay = [&src.tag, &src.author, &src.corporate, &src.title, &src.year, &src.journal, &src.publisher, &src.doi, &src.editor]
        .iter()
        .map(|f| f.to_lowercase())
        .collect::<Vec<_>>()
        .join("\u{1f}");
    q.split_whitespace().all(|w| hay.contains(&w.to_lowercase()))
}

/// The author line of a source: the corporate author, else the people.
fn who(src: &Source) -> &str {
    if src.corporate.trim().is_empty() { src.author.as_str() } else { src.corporate.trim() }
}

/// The fields a citation of `src` would lack in most styles, as the words the form uses.
fn missing(src: &Source) -> Vec<&'static str> {
    let mut m = Vec::new();
    if who(src).trim().is_empty() && src.editor.trim().is_empty() {
        m.push(tl!("author"));
    }
    if src.title.trim().is_empty() {
        m.push(tl!("title"));
    }
    if src.year.trim().is_empty() {
        m.push(tl!("year"));
    }
    m
}

/// A formatted bibliography entry as text with its italic spans (what a document gets).
fn entry_job(marked: &str, color: egui::Color32, width: f32) -> LayoutJob {
    let (text, spans) = split_marks(marked);
    let chars: Vec<char> = text.chars().collect();
    let font = egui::FontId::proportional(14.0);
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    let mut push = |range: Option<&[char]>, italics: bool| {
        if let Some(r) = range.filter(|r| !r.is_empty()) {
            job.append(&r.iter().collect::<String>(), 0.0, TextFormat { font_id: font.clone(), color, italics, ..Default::default() });
        }
    };
    let mut at = 0;
    for (a, b) in spans {
        push(chars.get(at..a), false);
        push(chars.get(a..b), true);
        at = b;
    }
    push(chars.get(at..), false);
    job
}

/// One row of the list: the title, then who and when.
fn row_job(src: &Source, t: &Tokens, width: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    job.wrap.max_rows = 2;
    job.wrap.break_anywhere = true;
    let title = if src.title.trim().is_empty() { tl!("(no title)") } else { src.title.trim() };
    job.append(title, 0.0, TextFormat { font_id: egui::FontId::proportional(14.0), color: t.text, ..Default::default() });
    let meta: Vec<&str> = [who(src), src.year.as_str()].into_iter().filter(|x| !x.trim().is_empty()).collect();
    job.append(
        &format!("\n{}", meta.join(" · ")),
        0.0,
        TextFormat { font_id: egui::FontId::proportional(12.0), color: t.text_dim, ..Default::default() },
    );
    job
}

/// The Source Manager: this document's sources beside My Library (the person's own list, shared
/// by every document), a search, the formatted entry and in-text citation of the selected source in
/// the current style, and New…, Import…, Paste…, Export…, Edit…, Delete, Insert Citation and the
/// copy between the two lists. Each button runs the command an agent would.
pub(super) fn sources_ui(app: &mut WordApp, ui: &mut Ui, view: &mut SourcesView) -> Next {
    let t = Tokens::get(ui.ctx());
    ui.set_min_width(760.0);
    let style = app.session.bib_style.clone();
    let doc_n = app.session.doc.sources.len();
    let lib_n = app.session.library.items().len();
    let mut next = Next::Stay;

    // The lists, and the style the preview follows.
    ui.horizontal(|ui| {
        if ui.selectable_label(!view.library, format!("{} ({doc_n})", tl!("This document"))).clicked() {
            view.library = false;
            view.selected = None;
        }
        if ui.selectable_label(view.library, format!("{} ({lib_n})", tl!("My library"))).clicked() {
            view.library = true;
            view.selected = None;
        }
        ui.with_layout(crate::rtl::far_side(ui, egui::Align::Center), |ui| {
            let combo = nodalia_ui::ComboBox::from_id_salt("src_style").selected_text(style.clone()).width(190.0).show_ui(ui, |ui| {
                for st in &letra_engine::cmd::citations::STYLES {
                    if ui.selectable_label(st.name == style, st.name).clicked()
                        && let Err(e) = app.run("references.citationStyle", json!({"style": st.name}))
                    {
                        app.status(e);
                    }
                }
            });
            combo.response.on_hover_text(tl!("Citation style"));
            ui.label(format!("{}:", tl!("Style")));
        });
    });
    ui.add_space(4.0);
    ui.add(egui::TextEdit::singleline(&mut view.query).desired_width(760.0).hint_text(tl!("Search by author, title, year, DOI or tag")));
    ui.add_space(6.0);

    let all: Vec<Source> = if view.library { app.session.library.items().to_vec() } else { app.session.doc.sources.clone() };
    let shown: Vec<(usize, &Source)> = all.iter().enumerate().filter(|(_, s)| matches_query(s, &view.query)).collect();
    let chosen = view.selected.as_ref().and_then(|tag| all.iter().position(|s| &s.tag == tag));

    ui.horizontal_top(|ui| {
        egui::Frame::new().stroke(egui::Stroke::new(1.0, t.border_strong)).inner_margin(4.0).show(ui, |ui| {
            ui.set_width(330.0);
            ui.set_min_height(300.0);
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                egui::ScrollArea::vertical().id_salt("src_list").max_height(300.0).auto_shrink([false, false]).show(ui, |ui| {
                    if shown.is_empty() {
                        let empty = if !view.query.trim().is_empty() {
                            tl!("No sources match your search.")
                        } else if view.library {
                            tl!("My library is empty. Import a bibliography or save sources from a document.")
                        } else {
                            tl!("No sources yet.")
                        };
                        ui.label(egui::RichText::new(empty).color(t.text_dim));
                    }
                    for (_, s) in &shown {
                        let on = view.selected.as_deref() == Some(s.tag.as_str());
                        if ui.selectable_label(on, row_job(s, &t, 300.0)).clicked() {
                            view.selected = Some(s.tag.clone());
                        }
                    }
                })
            });
        });
        ui.add_space(8.0);
        ui.vertical(|ui| {
            ui.set_width(390.0);
            ui.set_min_height(300.0);
            match chosen.and_then(|i| all.get(i).map(|s| (i, s))) {
                None => {
                    ui.label(egui::RichText::new(tl!("Select a source to see how it is cited and listed in the current style.")).color(t.text_dim));
                }
                Some((i, s)) => {
                    // Numbered styles number a source by its place in the document's list.
                    let n = if view.library { 1 } else { i + 1 };
                    ui.label(egui::RichText::new(tl!("Bibliography entry")).color(t.text_dim));
                    ui.add(egui::Label::new(entry_job(&entry_marked(s, &style, n), t.text, 380.0)).wrap());
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(tl!("In the text")).color(t.text_dim));
                    ui.label(cite(s, &style, n, ""));
                    let gaps = missing(s);
                    if !gaps.is_empty() {
                        ui.add_space(8.0);
                        let list = gaps.join(", ");
                        ui.label(egui::RichText::new(crate::i18n::fmt(tl!("Missing: {fields}"), &[("fields", &list)])).color(t.text_dim));
                    }
                    if !s.doi.trim().is_empty() {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(format!("DOI {}", s.doi.trim())).color(t.text_dim));
                    }
                }
            }
        });
    });

    // What can be done with the selected source.
    let src = chosen.and_then(|i| all.get(i)).cloned();
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let some = src.is_some();
        if view.library {
            if ui.add_enabled(some, egui::Button::new(tl!("Insert Citation"))).clicked()
                && let Some(s) = &src
            {
                if let Err(e) = app.run("references.citation", json!({"fromLibrary": s.tag})) {
                    app.status(e);
                }
                next = Next::Close;
            }
            if ui.add_enabled(some, egui::Button::new(tl!("Add to document"))).clicked()
                && let Some(s) = &src
            {
                match app.run("references.sources", json!({"fromLibrary": s.tag})) {
                    Ok(_) => app.status(tl!("Added to this document")),
                    Err(e) => app.status(e),
                }
            }
        } else {
            if ui.add_enabled(some, egui::Button::new(tl!("Insert Citation"))).clicked()
                && let Some(s) = &src
            {
                if let Err(e) = app.run("references.citation", json!({"tag": s.tag})) {
                    app.status(e);
                }
                next = Next::Close;
            }
            if ui.add_enabled(some, egui::Button::new(tl!("Save to My library"))).clicked()
                && let Some(s) = &src
            {
                match app.run("references.library", json!({"add": s.tag})) {
                    Ok(r) if r["new"] == json!(false) => app.status(tl!("Already in My library")),
                    Ok(_) => app.status(tl!("Saved to My library")),
                    Err(e) => app.status(e),
                }
            }
        }
        if ui.add_enabled(some, egui::Button::new(tl!("Edit…"))).clicked()
            && let Some(s) = &src
        {
            let form = SourceForm::of(s.clone(), true, false, true);
            next = go(RefsDialog::Source(Box::new(if view.library { form.in_library() } else { form })));
        }
        if ui.add_enabled(some, egui::Button::new(tl!("Delete"))).clicked()
            && let Some(s) = &src
        {
            let r = if view.library {
                app.run("references.library", json!({"remove": s.tag}))
            } else {
                app.run("references.sources", json!({"remove": s.tag}))
            };
            if let Err(e) = r {
                app.status(e);
            }
            view.selected = None;
        }
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button(tl!("New…")).clicked() {
            let form = SourceForm::new(false, true);
            next = go(RefsDialog::Source(Box::new(if view.library { form.in_library() } else { form })));
        }
        // Import… and Export… (desktop): BibTeX, RIS, CSL-JSON or Word's own Sources.xml into the
        // list on show (#2640); the list follows on the next frame.
        #[cfg(not(target_arch = "wasm32"))]
        if ui.button(tl!("Import…")).clicked()
            && let Asked::Done(Err(e)) = app.ask_file(
                FileDialogRequest::Open { purpose: "sources".into() },
                if view.library { AfterPick::SourcesLibrary } else { AfterPick::Sources },
            )
        {
            app.status(e);
        }
        if ui.selectable_label(view.paste.is_some(), tl!("Paste…")).clicked() {
            view.paste = if view.paste.is_some() { None } else { Some(String::new()) };
        }
        #[cfg(not(target_arch = "wasm32"))]
        if ui.button(tl!("Export…")).clicked()
            && let Asked::Done(Err(e)) =
                app.ask_file(FileDialogRequest::Save { name: "sources.bib".into(), dir: None }, AfterPick::SourcesExport { library: view.library })
        {
            app.status(e);
        }
        ui.with_layout(crate::rtl::far_side(ui, egui::Align::Center), |ui| {
            if ui.button(tl!("Close")).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                next = Next::Close;
            }
        });
    });
    if let Some(text) = view.paste.as_mut() {
        ui.add_space(6.0);
        ui.add(egui::TextEdit::multiline(text).desired_rows(5).desired_width(760.0).hint_text(tl!("Paste BibTeX, RIS or CSL-JSON here")));
        let mut done = false;
        ui.horizontal(|ui| {
            if ui.add_enabled(!text.trim().is_empty(), egui::Button::new(tl!("Import pasted text"))).clicked() {
                let to = if view.library { "library" } else { "document" };
                match app.run("references.import", json!({"text": text.clone(), "to": to})) {
                    Ok(r) => {
                        app.status(crate::file_dialogs::imported_message(r[to]["added"].as_array().map_or(0, Vec::len)));
                        done = true;
                    }
                    Err(e) => app.status(e),
                }
            }
        });
        if done {
            view.paste = None;
        }
    }
    next
}

#[cfg(test)]
mod manager_tests {
    use super::*;

    fn src(author: &str, title: &str, year: &str) -> Source {
        Source { tag: "T1".into(), author: author.into(), title: title.into(), year: year.into(), doi: "10.1/AbC".into(), ..Default::default() }
    }

    #[test]
    fn search_matches_every_word_in_any_field_ignoring_case() {
        let s = src("García, María", "Deep learning", "2020");
        assert!(matches_query(&s, ""));
        assert!(matches_query(&s, "garcía 2020"));
        assert!(matches_query(&s, "DEEP abc"));
        assert!(matches_query(&s, "t1"));
        assert!(!matches_query(&s, "deep 1999"));
    }

    #[test]
    fn a_source_lacking_author_title_or_year_says_so() {
        assert!(missing(&src("A, B", "T", "2000")).is_empty());
        assert_eq!(missing(&src("", "T", "")).len(), 2);
        let corporate = Source { corporate: "WHO".into(), title: "T".into(), year: "2021".into(), ..Default::default() };
        assert!(missing(&corporate).is_empty());
    }

    #[test]
    fn the_entry_preview_italicises_what_the_style_does_with_characters_not_bytes() {
        let marked = format!("García (2020). {}Título{}. Ed.", '\u{E000}', '\u{E001}');
        let job = entry_job(&marked, egui::Color32::BLACK, 300.0);
        let italic: String = job.sections.iter().filter(|s| s.format.italics).map(|s| &job.text[s.byte_range.start.0..s.byte_range.end.0]).collect();
        assert_eq!(italic, "Título");
        assert_eq!(job.text, "García (2020). Título. Ed.");
    }
}
