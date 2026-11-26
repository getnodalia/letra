//! Templates for long, structured documents: a thesis or final-year project. They are built the
//! way a person would build them: blocks first, then a scratch session fills in what only the
//! engine can make (the table of contents, the lists of figures and tables, numbered captions,
//! a numbered equation, the bibliography), so the template opens complete and already correct.

use crate::Session;
use crate::sample::{Lang, add, body, para, styled, text_table};
use crate::sample_design::{ACCENT, callout, line, quiet_table};
use letra_doc::para::InlineObject;
use letra_doc::props::{Align, Borders, CharProps, LineSpacing, NumRef, Rgb, TextColor};
use letra_doc::section::{NumFormat, PageVAlign, SectionProps};
use letra_doc::styles::{Style, StyleKind};
use letra_doc::{Block, Document, ParaProps, Paragraph, PartKind, Source, para_block};
use serde_json::{Value, json};
use std::sync::Arc;

/// Dim text for secondary lines.
const DIM: Rgb = Rgb(0x6B, 0x7A, 0x82);

/// The page: A4 for Spanish, Letter otherwise, with a wider inner margin for binding.
fn page(lang: Lang) -> SectionProps {
    let (page_w, page_h) = match lang {
        Lang::En => (612.0, 792.0),
        Lang::Es => (595.3, 841.9),
    };
    SectionProps { page_w, page_h, margin_top: 72.0, margin_bottom: 72.0, margin_left: 85.0, margin_right: 72.0, ..SectionProps::default() }
}

fn colour(c: Rgb) -> Option<TextColor> {
    Some(TextColor::Rgb(c))
}

fn field(instr: &str, result: &str) -> InlineObject {
    InlineObject::Field { instr: instr.into(), result: result.into(), locked: false, dirty: false, code: false, form: String::new() }
}

/// A page-number footer, centred, in dim small type.
fn footer(d: &mut Document) -> u32 {
    let mut f = Paragraph::new().styled("Footer");
    f.props.align = Some(Align::Center);
    let _ = f.insert_object(0, field("PAGE", "1"), &CharProps { color: colour(DIM), size: Some(10.0), ..Default::default() });
    d.add_part(PartKind::Footer, vec![para_block(f)])
}

/// A running header: the title of the current chapter over a hairline.
fn chapter_header(d: &mut Document) -> u32 {
    let mut h = Paragraph::new().styled("Header");
    let _ = h.insert_object(0, field("STYLEREF 1", ""), &CharProps { color: colour(DIM), size: Some(9.5), italic: Some(true), ..Default::default() });
    h.props.borders = Some(Borders { bottom: Some(line(0.5, Rgb(0xCB, 0xD5, 0xD8), 4.0)), ..Default::default() });
    d.add_part(PartKind::Header, vec![para_block(h)])
}

/// The paragraph styles the long templates add on top of the built-in ones.
fn add_styles(d: &mut Document, lang: Lang) {
    let styles = Arc::make_mut(&mut d.styles);
    let serif = Some("Cambria".to_string());
    let sans = Some("Calibri".to_string());
    // Body text: a serif at 11.5 pt with room between lines, justified, the way a long text reads.
    styles.default_chr.font = serif.clone();
    styles.default_chr.size = Some(11.5);
    styles.default_chr.lang = Some(lang.pick("en-US", "es-ES").into());
    styles.default_para.line_spacing = Some(LineSpacing::Multiple(1.3));
    styles.default_para.space_after = Some(8.0);
    styles.default_para.align = Some(Align::Justify);
    let heading = |id: &str, size: f32, before: f32, after: f32, break_before: bool| {
        let st = styles_heading(id);
        Style {
            para: ParaProps {
                space_before: Some(before),
                space_after: Some(after),
                keep_next: Some(true),
                page_break_before: break_before.then_some(true),
                align: Some(Align::Left),
                ..st.para
            },
            chr: CharProps { font: sans.clone(), size: Some(size), bold: Some(true), color: colour(ACCENT), ..st.chr },
            ..st
        }
    };
    styles.upsert(heading("Heading1", 22.0, 0.0, 22.0, true));
    styles.upsert(heading("Heading2", 15.0, 18.0, 6.0, false));
    styles.upsert(heading("Heading3", 12.5, 12.0, 4.0, false));
    // Headings of the parts of a thesis that are not chapters: the same look, no number.
    styles.upsert(Style {
        id: "FrontHeading".into(),
        name: "Front matter heading".into(),
        kind: StyleKind::Paragraph,
        based_on: Some("Heading1".into()),
        next: Some("Normal".into()),
        para: ParaProps { numbering: Some(NumRef { num: 0, level: 0 }), ..ParaProps::default() },
        priority: Some(9),
        quick: true,
        ..Style::default()
    });
    // The contents' own title is a heading style too, and must not take a chapter number.
    if let Some(t) = styles.get_mut("TOCHeading") {
        t.para.numbering = Some(NumRef { num: 0, level: 0 });
    }
    styles.upsert(Style {
        id: "AppendixHeading".into(),
        name: "Appendix heading".into(),
        kind: StyleKind::Paragraph,
        based_on: Some("FrontHeading".into()),
        next: Some("Normal".into()),
        priority: Some(10),
        quick: true,
        ..Style::default()
    });
    styles.upsert(Style {
        id: "CoverLine".into(),
        name: "Cover line".into(),
        kind: StyleKind::Paragraph,
        based_on: Some("Normal".into()),
        para: ParaProps { align: Some(Align::Center), space_after: Some(6.0), ..ParaProps::default() },
        chr: CharProps { font: sans.clone(), size: Some(13.0), color: colour(DIM), ..CharProps::default() },
        priority: Some(11),
        quick: true,
        ..Style::default()
    });
    styles.upsert(Style {
        id: "Abstract".into(),
        name: "Abstract".into(),
        kind: StyleKind::Paragraph,
        based_on: Some("Normal".into()),
        para: ParaProps { indent_left: Some(18.0), indent_right: Some(18.0), ..ParaProps::default() },
        chr: CharProps { size: Some(11.0), ..CharProps::default() },
        priority: Some(12),
        quick: true,
        ..Style::default()
    });
    // The title style of a cover: large, light, centred.
    if let Some(t) = styles.get_mut("Title") {
        t.para.align = Some(Align::Center);
        t.para.space_after = Some(12.0);
        t.chr.font = sans.clone();
        t.chr.size = Some(34.0);
        t.chr.bold = Some(true);
        t.chr.color = colour(ACCENT);
    }
    if let Some(t) = styles.get_mut("Subtitle") {
        t.para.align = Some(Align::Center);
        t.chr.font = sans;
        t.chr.size = Some(16.0);
        t.chr.color = colour(DIM);
    }
}

/// The built-in heading style `id`, as a starting point (the sheet always has them).
fn styles_heading(id: &str) -> Style {
    letra_doc::styles::StyleSheet::builtin().get(id).cloned().unwrap_or_else(|| Style { id: id.into(), name: id.into(), ..Style::default() })
}

/// A paragraph that stays empty until the scratch session puts something there.
fn marker() -> Block {
    Block::Para(Paragraph::new())
}

/// What the scratch session makes at a marker.
enum Make {
    Toc {
        title: &'static str,
    },
    Figures {
        label: &'static str,
        title: &'static str,
    },
    Bibliography {
        title: &'static str,
    },
    /// A caption for the block at the marker's place (which is the block, not an empty paragraph).
    Caption {
        label: &'static str,
        text: &'static str,
        above: bool,
    },
    /// The paragraph there is an equation made a numbered display.
    Equation {
        latex: &'static str,
    },
}

/// A block list with the work still to do at some of its blocks.
struct Draft {
    blocks: Vec<Block>,
    makes: Vec<(usize, Make)>,
}

impl Draft {
    fn new() -> Draft {
        Draft { blocks: Vec::new(), makes: Vec::new() }
    }
    fn push(&mut self, p: Paragraph) {
        self.blocks.push(Block::Para(p));
    }
    fn text(&mut self, t: &str) {
        self.push(para(t));
    }
    fn style(&mut self, t: &str, style: &str) {
        self.push(styled(t, style));
    }
    /// A marker paragraph with work to do at it.
    fn make(&mut self, m: Make) {
        self.makes.push((self.blocks.len(), m));
        self.blocks.push(marker());
    }
    /// A block (a table, a placeholder) with a caption made next to it.
    fn captioned(&mut self, block: Block, label: &'static str, text: &'static str, above: bool) {
        self.makes.push((self.blocks.len(), Make::Caption { label, text, above }));
        self.blocks.push(block);
    }
    /// The last paragraph ends a section with `props`.
    fn end_section(&mut self, props: SectionProps) {
        let mut p = Paragraph::new();
        p.section = Some(Box::new(props));
        self.blocks.push(Block::Para(p));
    }
}

/// A line of the cover: `style` text, or an empty line of the given size to space the page.
fn cover_line(text: &str) -> Paragraph {
    styled(text, "CoverLine")
}

/// A thesis or final-year project (TFG/TFM): cover, front matter in roman numerals, numbered
/// chapters with a running header, the bibliography and an appendix.
pub fn thesis(lang: Lang) -> Document {
    let mut d = Document::new();
    d.core.title = lang.pick("Title of the thesis", "Título del trabajo").into();
    d.core.creator = "Letra".into();
    add_styles(&mut d, lang);
    d.sources = vec![
        Source {
            journal: "The Computer Journal".into(),
            volume: "27".into(),
            pages: "97-111".into(),
            ..source("knuth1984", "article", "Knuth, Donald", "Literate Programming", "1984", "")
        },
        Source {
            city: "Reading".into(),
            ..source("lamport1994", "book", "Lamport, Leslie", "LaTeX: A Document Preparation System", "1994", "Addison-Wesley")
        },
    ];
    let mut b = Draft::new();

    // Cover: one page, set in the middle of it.
    let mut c = para("").styled("CoverLine");
    c.insert_text(0, lang.pick("University name · School or faculty", "Nombre de la universidad · Facultad o escuela"), &CharProps::default()).ok();
    b.push(c);
    b.push(cover_line(lang.pick("Degree in … (or Master's in …)", "Grado en … (o Máster en …)")));
    b.push(styled("", "CoverLine"));
    b.style(lang.pick("Title of the thesis", "Título del trabajo"), "Title");
    b.style(lang.pick("A subtitle that says what it is about", "Un subtítulo que diga de qué trata"), "Subtitle");
    b.push(styled("", "CoverLine"));
    b.push(cover_line(lang.pick("Final degree project", "Trabajo de fin de grado")));
    b.push(cover_line(&format!("{}: {}", lang.pick("Author", "Autor/a"), lang.pick("Your name", "Tu nombre"))));
    b.push(cover_line(&format!("{}: {}", lang.pick("Supervisor", "Tutor/a"), lang.pick("Name of your supervisor", "Nombre de tu tutor/a"))));
    b.push(cover_line(&lang.long_date()));
    b.end_section(SectionProps { valign: PageVAlign::Center, ..page(lang) });

    // Front matter: roman page numbers.
    b.style(lang.pick("Declaration of originality", "Declaración de originalidad"), "FrontHeading");
    b.text(lang.pick(
        "I declare that this work is my own, that the ideas and results of others are credited in the bibliography, and that it has not been submitted for any other degree.",
        "Declaro que este trabajo es original, que las ideas y los resultados de otras personas se citan en la bibliografía y que no se ha presentado para obtener ningún otro título.",
    ));
    b.push(styled(&format!("{} · {}", lang.pick("Your name", "Tu nombre"), lang.long_date()), "CoverLine"));

    let (first, second) = match lang {
        Lang::En => (
            (
                "Abstract",
                "Say in about two hundred words what the work asks, how it goes about answering, what it finds and why that matters. Write it last, and write it for someone who will read nothing else.",
                "Keywords: keyword one, keyword two, keyword three",
            ),
            (
                "Resumen",
                "Resume en unas doscientas palabras qué se pregunta el trabajo, cómo lo responde, qué encuentra y por qué importa. Es la versión en español del resumen anterior.",
                "Palabras clave: palabra clave uno, palabra clave dos, palabra clave tres",
            ),
        ),
        Lang::Es => (
            (
                "Resumen",
                "Resume en unas doscientas palabras qué se pregunta el trabajo, cómo lo responde, qué encuentra y por qué importa. Escríbelo al final, pensando en quien no leerá nada más.",
                "Palabras clave: palabra clave uno, palabra clave dos, palabra clave tres",
            ),
            (
                "Abstract",
                "Say in about two hundred words what the work asks, how it goes about answering, what it finds and why that matters. This is the English version of the summary above.",
                "Keywords: keyword one, keyword two, keyword three",
            ),
        ),
    };
    for (head, text, kw) in [first, second] {
        b.style(head, "FrontHeading");
        b.push(styled(text, "Abstract"));
        let (label, words) = kw.split_once(": ").unwrap_or((kw, ""));
        let mut k = styled("", "Abstract");
        add(&mut k, &format!("{label}: "), CharProps { bold: Some(true), ..Default::default() });
        add(&mut k, words, CharProps::default());
        b.push(k);
    }
    b.style(lang.pick("Acknowledgements", "Agradecimientos"), "FrontHeading");
    b.text(lang.pick(
        "Thank the people and the institutions that made the work possible: your supervisor, the people who read drafts, the ones who put up with you.",
        "Da las gracias a las personas y a las instituciones que hicieron posible el trabajo: tu tutor o tutora, quienes leyeron borradores, quienes te aguantaron.",
    ));
    b.make(Make::Toc { title: lang.pick("Contents", "Índice") });
    b.make(Make::Figures { label: "Figure", title: lang.pick("List of figures", "Índice de figuras") });
    b.make(Make::Figures { label: "Table", title: lang.pick("List of tables", "Índice de tablas") });
    b.style(lang.pick("List of abbreviations", "Lista de abreviaturas"), "FrontHeading");
    let abbreviations = match lang {
        Lang::En => [&["Abbreviation", "Meaning"][..], &["API", "Application programming interface"], &["TFG", "Final degree project"]],
        Lang::Es => [&["Abreviatura", "Significado"][..], &["API", "Interfaz de programación de aplicaciones"], &["TFG", "Trabajo de fin de grado"]],
    };
    let mut t = text_table(&abbreviations, 440.0, usize::MAX);
    quiet_table(&mut t, false);
    b.blocks.push(Block::Table(t));
    let front_footer = footer(&mut d);
    b.end_section(SectionProps {
        page_num_format: NumFormat::LowerRoman,
        page_num_start: Some(1),
        ..SectionProps { footers: letra_doc::section::HeaderSet { default: Some(front_footer), ..Default::default() }, ..page(lang) }
    });

    // Chapters.
    chapter(
        &mut b,
        lang,
        "Introduction",
        "Introducción",
        [
            (
                "Context and motivation",
                "Contexto y motivación",
                "Start from the wider field and narrow to your question. Explain why someone who is not in your field should care, and what happens if the question stays unanswered.",
                "Parte del campo más amplio y estréchalo hasta tu pregunta. Explica por qué debería importar a alguien ajeno a tu campo y qué ocurre si la pregunta queda sin respuesta.",
            ),
            (
                "Objectives",
                "Objetivos",
                "State one general objective and a few specific ones that can be checked at the end. Each should be something you can say was done or not done.",
                "Enuncia un objetivo general y unos pocos específicos que puedan comprobarse al final. Cada uno debe poder darse por cumplido o por no cumplido.",
            ),
            (
                "Structure of this document",
                "Estructura del documento",
                "Say what each chapter does in a sentence. A reader who knows the map finds the way back when they get lost.",
                "Di en una frase qué hace cada capítulo. Quien conoce el mapa encuentra el camino de vuelta cuando se pierde.",
            ),
        ],
    );
    chapter(
        &mut b,
        lang,
        "Background",
        "Marco teórico",
        [
            (
                "Related work",
                "Trabajos relacionados",
                "Review what others have done, grouped by idea and not by author. Cite them as you go, for example Knuth's argument that a program is also a text to be read [[cite:knuth1984]]. Systems that keep content apart from layout, such as LaTeX, follow the same idea [[cite:lamport1994]].",
                "Repasa lo que han hecho otros, agrupado por ideas y no por autores. Cítalos sobre la marcha, por ejemplo el argumento de Knuth de que un programa es también un texto que se lee [[cite:knuth1984]]. Los sistemas que separan el contenido de la maquetación, como LaTeX, siguen la misma idea [[cite:lamport1994]].",
            ),
            (
                "Concepts and definitions",
                "Conceptos y definiciones",
                "Define the terms you will use, once, here. Later chapters can then use them without stopping to explain.",
                "Define aquí, una sola vez, los términos que vas a usar. Los capítulos siguientes podrán usarlos sin detenerse a explicarlos.",
            ),
        ],
    );
    // Methodology, with the numbered equation, a figure and a table.
    b.style(lang.pick("Methodology", "Metodología"), "Heading1");
    b.style(lang.pick("Approach", "Enfoque"), "Heading2");
    b.text(lang.pick(
        "Describe what you did in enough detail that someone else could repeat it. Where a model is needed, set it on its own line so that it can be numbered and referred to:",
        "Describe lo que hiciste con el detalle suficiente para que otra persona pueda repetirlo. Cuando haga falta un modelo, ponlo en su propia línea para poder numerarlo y citarlo:",
    ));
    b.make(Make::Equation { latex: r"y = \beta_0 + \beta_1 x + \varepsilon" });
    b.text(lang.pick(
        "Refer to it by number from the text with Insert > Cross-reference, and the number follows the chapter when the chapters move.",
        "Remítete a ella por su número con Insertar > Referencia cruzada, y el número seguirá al capítulo cuando los capítulos cambien de sitio.",
    ));
    b.style(lang.pick("Data and materials", "Datos y materiales"), "Heading2");
    b.text(lang.pick(
        "Say where the data come from, how they were cleaned, and what was left out and why.",
        "Di de dónde salen los datos, cómo se limpiaron y qué se dejó fuera y por qué.",
    ));
    let mut placeholder = styled(lang.pick("Replace this with your figure", "Sustituye esto por tu figura"), "CoverLine");
    placeholder.props.space_before = Some(6.0);
    placeholder.props.space_after = Some(4.0);
    placeholder.props.keep_next = Some(true);
    callout(&mut placeholder);
    placeholder.props.align = Some(Align::Center);
    b.captioned(Block::Para(placeholder), "Figure", lang.pick("Describe what the figure shows", "Describe lo que muestra la figura"), false);

    chapter(
        &mut b,
        lang,
        "Results",
        "Resultados",
        [(
            "Main findings",
            "Resultados principales",
            "Report what you found, without interpreting it yet. Tables carry exact numbers; the text says which to look at.",
            "Presenta lo que encontraste, sin interpretarlo todavía. Las tablas llevan los números exactos; el texto indica cuáles mirar.",
        )],
    );
    let results = match lang {
        Lang::En => [&["Measure", "Before", "After"][..], &["Visits per week", "120", "168"], &["Satisfaction", "71 %", "84 %"]],
        Lang::Es => [&["Indicador", "Antes", "Después"][..], &["Visitas por semana", "120", "168"], &["Satisfacción", "71 %", "84 %"]],
    };
    let mut table = text_table(&results, 440.0, 1);
    quiet_table(&mut table, false);
    b.captioned(Block::Table(table), "Table", lang.pick("Describe what the table shows", "Describe lo que muestra la tabla"), true);
    b.text("");

    chapter(
        &mut b,
        lang,
        "Discussion",
        "Discusión",
        [
            (
                "Interpretation",
                "Interpretación",
                "Say what the findings mean, how they sit with the work in the background chapter, and where they are weaker than they look.",
                "Explica qué significan los resultados, cómo encajan con el marco teórico y dónde son más débiles de lo que parecen.",
            ),
            (
                "Limitations",
                "Limitaciones",
                "Name the limits yourself. A reader trusts an author who has already found them.",
                "Nombra tú las limitaciones. Quien lee se fía de quien ya las ha encontrado.",
            ),
        ],
    );
    chapter(
        &mut b,
        lang,
        "Conclusions and future work",
        "Conclusiones y trabajo futuro",
        [
            (
                "Conclusions",
                "Conclusiones",
                "Answer the question from the introduction, plainly, and say which objectives were met.",
                "Responde con claridad a la pregunta de la introducción y di qué objetivos se cumplieron.",
            ),
            (
                "Future work",
                "Trabajo futuro",
                "List what you would do next, in order of how much it would add.",
                "Enumera lo que harías a continuación, por orden de lo que aportaría.",
            ),
        ],
    );

    b.make(Make::Bibliography { title: lang.pick("Bibliography", "Bibliografía") });
    b.style(lang.pick("Appendix A. Additional material", "Anexo A. Material adicional"), "AppendixHeading");
    b.text(lang.pick(
        "Put here what supports the work but would interrupt it: long tables, questionnaires, listings, proofs.",
        "Pon aquí lo que respalda el trabajo pero lo interrumpiría: tablas largas, cuestionarios, listados, demostraciones.",
    ));

    let body_footer = footer(&mut d);
    let header = chapter_header(&mut d);
    d.last_section = SectionProps {
        page_num_start: Some(1),
        headers: letra_doc::section::HeaderSet { default: Some(header), ..Default::default() },
        footers: letra_doc::section::HeaderSet { default: Some(body_footer), ..Default::default() },
        ..page(lang)
    };
    finish(d, b, lang)
}

/// A numbered chapter: a Heading 1 and, for each `(title, text)`, a Heading 2 with a paragraph.
fn chapter<const N: usize>(
    b: &mut Draft,
    lang: Lang,
    en: &'static str,
    es: &'static str,
    sections: [(&'static str, &'static str, &'static str, &'static str); N],
) {
    b.style(lang.pick(en, es), "Heading1");
    for (t_en, t_es, p_en, p_es) in sections {
        b.style(lang.pick(t_en, t_es), "Heading2");
        b.text(lang.pick(p_en, p_es));
    }
}

fn source(tag: &str, kind: &str, author: &str, title: &str, year: &str, publisher: &str) -> Source {
    Source {
        tag: tag.into(),
        kind: kind.into(),
        author: author.into(),
        title: title.into(),
        year: year.into(),
        publisher: publisher.into(),
        ..Source::default()
    }
}

/// Run the scratch session over the draft: number the headings, then make everything the engine
/// makes, last to first so that the positions still to come stay where they were.
fn finish(mut d: Document, b: Draft, lang: Lang) -> Document {
    let Draft { mut blocks, makes } = b;
    for block in &mut blocks {
        if let Block::Para(p) = block
            && let Some(cited) = with_citation(p)
        {
            *p = cited;
        }
    }
    body(&mut d, blocks);
    let mut s = Session::new(d);
    // Number the chapters (the headings the caret is on keeps its style; the others follow).
    if let Some(first) = s.doc.body.iter().position(|b| b.as_para().is_some_and(|p| p.props.style.as_deref() == Some("Heading1"))) {
        s.sel = crate::Selection::caret(letra_doc::Pos::body(first, 0));
        let _ = s.run("list.multilevelPreset", &json!({"preset": "headings"}));
    }
    for (at, make) in makes.iter().rev() {
        s.sel = crate::Selection::caret(letra_doc::Pos::body(*at, 0));
        let (command, params): (&str, Value) = match make {
            Make::Toc { title } => ("references.toc", json!({"levels": 2, "title": title})),
            Make::Figures { label, title } => {
                // The list's own title is a heading of the front matter, above it.
                let heading = styled(title, "FrontHeading");
                let _ = s.doc.insert_block(letra_doc::StoryRef::Body, &letra_doc::Path::top(*at), Block::Para(heading));
                s.sel = crate::Selection::caret(letra_doc::Pos::body(at + 1, 0));
                ("references.tableOfFigures", json!({"label": localised_label(label, lang)}))
            }
            Make::Bibliography { title } => {
                let heading = styled(title, "FrontHeading");
                let _ = s.doc.insert_block(letra_doc::StoryRef::Body, &letra_doc::Path::top(*at), Block::Para(heading));
                s.sel = crate::Selection::caret(letra_doc::Pos::body(at + 1, 0));
                ("references.bibliography", json!({"title": ""}))
            }
            Make::Caption { label, text, above } => (
                "references.caption",
                json!({"label": localised_label(label, lang), "text": text, "position": if *above { "above" } else { "below" }, "chapter": 1, "separator": "."}),
            ),
            Make::Equation { latex } => {
                let _ = s.run("insert.equation", &json!({"latex": latex, "display": false, "edit": false}));
                ("references.numberEquation", json!({"chapter": 1, "separator": ".", "label": localised_label("Equation", lang)}))
            }
        };
        let _ = s.run(command, &params);
    }
    drop_blank_before_headings(&mut s.doc);
    // Every field fresh: numbers, citations, the contents with its page numbers.
    let _ = s.run("references.updateFields", &json!({}));
    // The contents counts its own pages, so a second pass settles what the first one moved.
    for _ in 0..2 {
        let _ = s.run("references.updateToc", &json!({}));
    }
    let mut doc = s.doc.clone();
    doc.core.creator = "Letra".into();
    doc
}

/// Remove the empty paragraph a generated list leaves before the next heading: with the page
/// break the heading brings, it would be a page of its own whenever the list fills its page.
fn drop_blank_before_headings(d: &mut Document) {
    let starts_page = |b: &Block| {
        b.as_para().is_some_and(|p| matches!(p.props.style.as_deref(), Some("FrontHeading" | "Heading1" | "AppendixHeading" | "TOCHeading")))
    };
    let mut keep = Vec::with_capacity(d.body.len());
    for (i, b) in d.body.iter().enumerate() {
        let blank = b.as_para().is_some_and(|p| p.is_empty() && p.objects.is_empty() && p.section.is_none());
        keep.push(!(blank && d.body.get(i + 1).is_some_and(|n| starts_page(n))));
    }
    let mut keep = keep.into_iter();
    d.body.retain(|_| keep.next().unwrap_or(true));
}

/// The caption label in `lang`: the word the caption shows, which is also the sequence's name.
fn localised_label(label: &str, lang: Lang) -> &'static str {
    match (label, lang) {
        ("Figure", Lang::Es) => "Figura",
        ("Table", Lang::Es) => "Tabla",
        ("Equation", Lang::Es) => "Ecuación",
        ("Figure", _) => "Figure",
        ("Table", _) => "Table",
        _ => "Equation",
    }
}

/// The paragraph with each `[[cite:tag]]` mark made a citation field, if it has any.
fn with_citation(p: &Paragraph) -> Option<Paragraph> {
    let text = p.plain_text();
    text.find("[[cite:")?;
    let mut out = Paragraph::with_text("", CharProps::default());
    out.props = p.props.clone();
    let mut rest = text.as_str();
    while let Some(start) = rest.find("[[cite:") {
        let close = start + rest.get(start..)?.find("]]")?;
        let at = out.len();
        out.insert_text(at, rest.get(..start)?, &CharProps::default()).ok()?;
        let at = out.len();
        out.insert_object(at, field(&format!("CITATION {}", rest.get(start + 7..close)?), ""), &CharProps::default()).ok()?;
        rest = rest.get(close + 2..)?;
    }
    let at = out.len();
    out.insert_text(at, rest, &CharProps::default()).ok()?;
    Some(out)
}

/// Whether template `id` is one of the long templates, which carry their own design.
pub(crate) fn is_long(id: &str) -> bool {
    matches!(id, "thesis")
}

#[cfg(test)]
mod tests {
    use super::*;
    use letra_doc::StoryRef;

    fn text(d: &Document) -> String {
        d.plain_text(StoryRef::Body)
    }

    /// The thesis has a cover, a front matter in roman numerals and a body that starts again at 1.
    #[test]
    fn the_thesis_has_three_sections_numbered_apart() {
        let d = thesis(Lang::En);
        let ended: Vec<&SectionProps> = d.body.iter().filter_map(|b| b.as_para()).filter_map(|p| p.section.as_deref()).collect();
        assert_eq!(ended.len(), 2, "cover and front matter end their sections; the body is the last");
        assert_eq!(ended[0].valign, PageVAlign::Center, "the cover sits in the middle of its page");
        assert_eq!(ended[1].page_num_format, NumFormat::LowerRoman);
        assert_eq!(d.last_section.page_num_format, NumFormat::Decimal);
        assert_eq!(d.last_section.page_num_start, Some(1));
        assert!(d.last_section.headers.default.is_some(), "the body has a running header");
    }

    /// The contents, the lists, the captions, the equation and the bibliography are made, and the
    /// chapters are numbered.
    #[test]
    fn the_thesis_arrives_complete() {
        for (lang, figure, table) in [(Lang::En, "Figure 3.1:", "Table 4.1:"), (Lang::Es, "Figura 3.1:", "Tabla 4.1:")] {
            let t = text(&thesis(lang));
            assert!(t.contains("1\tIntroduc"), "{lang:?}: numbered chapters in the contents");
            assert!(t.contains(&format!("{figure} ")) && t.contains(&format!("{table} ")), "{lang:?}: captions and their lists");
            assert!(t.contains("(3.1)"), "{lang:?}: the equation is numbered by chapter");
            assert!(t.contains("Knuth, D. (1984)"), "{lang:?}: the bibliography lists the cited sources");
            assert!(t.contains("(Lamport, 1994)"), "{lang:?}: the citations in the text are filled in");
        }
    }

    /// The contents agree with the pages: the first chapter is on page 1 of the body, and the front
    /// matter's last entries sit on the roman pages they are on (no blank page in between).
    #[test]
    fn the_thesis_contents_match_its_pages() {
        let d = thesis(Lang::En);
        let t = text(&d);
        assert!(t.contains("Introduction\t1\n"), "{t}");
        assert!(t.contains("List of abbreviations\tviii"), "{t}");
        let l = letra_layout::layout(&d, &mut letra_layout::LayoutCache::new(), &Default::default());
        assert_eq!(l.pages.len(), 17);
    }

    /// The sections, numbering and styles survive a save to .docx and back.
    #[test]
    fn the_thesis_survives_docx() {
        let d = thesis(Lang::En);
        let back = letra_docx::read(&letra_docx::write(&d).unwrap()).unwrap();
        assert_eq!(back.last_section.page_num_start, Some(1));
        assert!(back.styles.get("FrontHeading").is_some());
        assert_eq!(text(&back), text(&d));
    }
}
