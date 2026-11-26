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
struct Look {
    /// Body text, points, and the line spacing as a multiple.
    body: f32,
    spacing: f32,
    /// Heading 1: size, space before and after, a page of its own, capitals, alignment.
    h1: f32,
    h1_space: (f32, f32),
    h1_break: bool,
    h1_caps: bool,
    h1_align: Align,
    h2: f32,
    h3: f32,
    /// The title style's size.
    title: f32,
}

/// A thesis: a roomy page, each chapter on a page of its own.
const THESIS_LOOK: Look = Look {
    body: 11.5,
    spacing: 1.3,
    h1: 22.0,
    h1_space: (0.0, 22.0),
    h1_break: true,
    h1_caps: false,
    h1_align: Align::Left,
    h2: 15.0,
    h3: 12.5,
    title: 34.0,
};

fn add_styles(d: &mut Document, lang: Lang, look: &Look) {
    // Justified long text reads better hyphenated.
    d.settings.auto_hyphenation = true;
    d.settings.consecutive_hyphen_limit = 3;
    let styles = Arc::make_mut(&mut d.styles);
    let serif = Some("Cambria".to_string());
    let sans = Some("Calibri".to_string());
    // Body text: a serif at 11.5 pt with room between lines, justified, the way a long text reads.
    styles.default_chr.font = serif.clone();
    styles.default_chr.size = Some(look.body);
    styles.default_chr.lang = Some(lang.pick("en-US", "es-ES").into());
    styles.default_para.line_spacing = Some(LineSpacing::Multiple(look.spacing));
    styles.default_para.space_after = Some(look.body * 0.6);
    styles.default_para.align = Some(Align::Justify);
    let heading = |id: &str, size: f32, before: f32, after: f32, break_before: bool, align: Align, caps: bool| {
        let st = styles_heading(id);
        Style {
            para: ParaProps {
                space_before: Some(before),
                space_after: Some(after),
                keep_next: Some(true),
                page_break_before: break_before.then_some(true),
                align: Some(align),
                ..st.para
            },
            chr: CharProps { font: sans.clone(), size: Some(size), bold: Some(true), caps: caps.then_some(true), color: colour(ACCENT), ..st.chr },
            ..st
        }
    };
    let (before, after) = look.h1_space;
    styles.upsert(heading("Heading1", look.h1, before, after, look.h1_break, look.h1_align, look.h1_caps));
    styles.upsert(heading("Heading2", look.h2, look.h2 * 1.1, look.h2 * 0.35, false, Align::Left, false));
    styles.upsert(heading("Heading3", look.h3, look.h3 * 0.9, look.h3 * 0.3, false, Align::Left, false));
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
        t.chr.size = Some(look.title);
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
        levels: u8,
    },
    Index {
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
    /// Words to mark as index entries wherever they occur.
    entries: Vec<&'static str>,
}

impl Draft {
    fn new() -> Draft {
        Draft { blocks: Vec::new(), makes: Vec::new(), entries: Vec::new() }
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
    /// The last paragraph (an empty one if the last block is not a paragraph) ends a section with
    /// `props`, so that no empty line is left on the page.
    fn end_with(&mut self, props: SectionProps) {
        match self.blocks.last_mut() {
            Some(Block::Para(p)) if p.section.is_none() => p.section = Some(Box::new(props)),
            _ => self.end_section(props),
        }
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
    add_styles(&mut d, lang, &THESIS_LOOK);
    d.sources = thesis_sources();
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
    b.make(Make::Toc { title: lang.pick("Contents", "Índice"), levels: 2 });
    b.make(Make::Figures { label: "Figure", title: lang.pick("List of figures", "Índice de figuras") });
    b.make(Make::Figures { label: "Table", title: lang.pick("List of tables", "Índice de tablas") });
    b.style(lang.pick("List of abbreviations", "Lista de abreviaturas"), "FrontHeading");
    let abbreviations = match lang {
        Lang::En => [&["Abbreviation", "Meaning"][..], &["API", "Application programming interface"], &["TFG", "Final degree project"]],
        Lang::Es => [&["Abreviatura", "Significado"][..], &["API", "Interfaz de programación de aplicaciones"], &["TFG", "Trabajo de fin de grado"]],
    };
    b.blocks.push(Block::Table(numeric_table(&abbreviations, 440.0, usize::MAX)));
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
    b.captioned(
        Block::Table(numeric_table(&results, 440.0, 1)),
        "Table",
        lang.pick("Describe what the table shows", "Describe lo que muestra la tabla"),
        true,
    );
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

    d.last_section = chapter_body_section(&mut d, page(lang));
    finish(d, b, lang, &Numbering { preset: "headings", chapters: true })
}

/// The last section of a document with numbered chapters: page numbers from 1, the chapter's title
/// in the header and the page number in the footer.
fn chapter_body_section(d: &mut Document, page: SectionProps) -> SectionProps {
    let footer = footer(d);
    let header = chapter_header(d);
    SectionProps {
        page_num_start: Some(1),
        headers: letra_doc::section::HeaderSet { default: Some(header), ..Default::default() },
        footers: letra_doc::section::HeaderSet { default: Some(footer), ..Default::default() },
        ..page
    }
}

fn thesis_sources() -> Vec<Source> {
    vec![
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
    ]
}

/// A scientific paper: title, authors, abstract and keywords, numbered sections, a figure, a table
/// and a numbered equation, citations and the references they make. `two_columns` is the
/// IEEE-like layout (title block across the page, the body in two columns, sections I., II.);
/// otherwise a general journal layout in one column (sections 1, 1.1).
pub fn paper(lang: Lang, two_columns: bool) -> Document {
    let look = if two_columns {
        Look {
            body: 9.5,
            spacing: 1.1,
            h1: 10.5,
            h1_space: (12.0, 4.0),
            h1_break: false,
            h1_caps: true,
            h1_align: Align::Center,
            h2: 10.0,
            h3: 9.5,
            title: 22.0,
        }
    } else {
        Look {
            body: 11.0,
            spacing: 1.2,
            h1: 14.0,
            h1_space: (16.0, 6.0),
            h1_break: false,
            h1_caps: false,
            h1_align: Align::Left,
            h2: 12.0,
            h3: 11.0,
            title: 24.0,
        }
    };
    let mut d = Document::new();
    d.core.title = lang.pick("Title of the paper", "Título del artículo").into();
    d.core.creator = "Letra".into();
    add_styles(&mut d, lang, &look);
    d.sources = thesis_sources();
    let margin = if two_columns { 54.0 } else { 72.0 };
    let page = SectionProps { margin_left: margin, margin_right: margin, margin_top: 64.0, margin_bottom: 64.0, ..page(lang) };
    let mut b = Draft::new();

    b.style(lang.pick("Title of the paper: what it found, in one line", "Título del artículo: lo que encontró, en una línea"), "Title");
    b.push(styled(lang.pick("First Author¹, Second Author², Third Author¹", "Primer Autor¹, Segunda Autora², Tercer Autor¹"), "CoverLine"));
    let mut affiliations = styled(
        lang.pick(
            "¹ Department, University, City · ² Institute, City · Corresponding author: name@example.org",
            "¹ Departamento, Universidad, Ciudad · ² Instituto, Ciudad · Autor de correspondencia: nombre@example.org",
        ),
        "CoverLine",
    );
    format_small(&mut affiliations);
    b.push(affiliations);
    let (head, text, kw) = match lang {
        Lang::En => (
            "Abstract",
            "In about 150 words: the question, the method, the main result with its number, and what follows from it. Write it last and make it stand alone, since it is what most people will read.",
            "Keywords: keyword one, keyword two, keyword three",
        ),
        Lang::Es => (
            "Resumen",
            "En unas 150 palabras: la pregunta, el método, el resultado principal con su cifra y lo que se sigue de él. Escríbelo al final y hazlo autosuficiente, porque es lo que casi todos leerán.",
            "Palabras clave: palabra clave uno, palabra clave dos, palabra clave tres",
        ),
    };
    let mut abstract_para = styled("", "Abstract");
    add(&mut abstract_para, &format!("{head}. "), CharProps { bold: Some(true), ..Default::default() });
    add(&mut abstract_para, text, CharProps::default());
    callout(&mut abstract_para);
    abstract_para.props.indent_left = Some(0.0);
    abstract_para.props.indent_right = Some(0.0);
    b.push(abstract_para);
    let (label, words) = kw.split_once(": ").unwrap_or((kw, ""));
    let mut keywords = styled("", "Abstract");
    add(&mut keywords, &format!("{label}: "), CharProps { bold: Some(true), ..Default::default() });
    add(&mut keywords, words, CharProps::default());
    keywords.props.indent_left = Some(0.0);
    keywords.props.indent_right = Some(0.0);
    b.push(keywords);
    let footer_id = footer(&mut d);
    let footers = letra_doc::section::HeaderSet { default: Some(footer_id), ..Default::default() };
    if two_columns {
        b.end_section(SectionProps { footers, ..page.clone() });
    }

    chapter(
        &mut b,
        lang,
        "Introduction",
        "Introducción",
        [(
            "",
            "",
            "Open with the problem and why it matters, say what is known and what is not, and finish with what this paper does about it. In the text, cite as you go [[cite:knuth1984]]; Letra builds the list of references from what you cite.",
            "Abre con el problema y por qué importa, di qué se sabe y qué no, y termina con lo que hace este artículo al respecto. En el texto, cita sobre la marcha [[cite:knuth1984]]; Letra construye la lista de referencias con lo que citas.",
        )],
    );
    chapter(
        &mut b,
        lang,
        "Methods",
        "Métodos",
        [(
            "",
            "",
            "Describe the design, the data and the analysis so that another group could repeat them. A model goes on its own line, numbered so the text can point to it:",
            "Describe el diseño, los datos y el análisis para que otro grupo pueda repetirlos. Un modelo va en su propia línea, numerado para que el texto pueda remitirse a él:",
        )],
    );
    b.make(Make::Equation { latex: r"y = \beta_0 + \beta_1 x + \varepsilon" });
    let mut placeholder = styled(lang.pick("Replace this with your figure", "Sustituye esto por tu figura"), "CoverLine");
    callout(&mut placeholder);
    placeholder.props.align = Some(Align::Center);
    placeholder.props.keep_next = Some(true);
    b.captioned(
        Block::Para(placeholder),
        "Figure",
        lang.pick("Describe what the figure shows, so that it can be read alone", "Describe lo que muestra la figura, de modo que se entienda sola"),
        false,
    );
    chapter(
        &mut b,
        lang,
        "Results",
        "Resultados",
        [(
            "",
            "",
            "Report what was found before saying what it means. Put the exact numbers in tables and tell the reader which ones to look at.",
            "Presenta lo encontrado antes de decir qué significa. Pon las cifras exactas en tablas e indica al lector cuáles mirar.",
        )],
    );
    let rows = match lang {
        Lang::En => [&["Group", "Mean", "SD"][..], &["Control", "4.2", "0.9"], &["Treatment", "5.1", "1.1"]],
        Lang::Es => [&["Grupo", "Media", "DE"][..], &["Control", "4,2", "0,9"], &["Tratamiento", "5,1", "1,1"]],
    };
    b.captioned(
        Block::Table(numeric_table(&rows, if two_columns { 230.0 } else { 440.0 }, 1)),
        "Table",
        lang.pick("Describe what the table shows", "Describe lo que muestra la tabla"),
        true,
    );
    b.text("");
    chapter(
        &mut b,
        lang,
        "Discussion",
        "Discusión",
        [(
            "",
            "",
            "Say what the results mean, how they sit with earlier work, and where they are weaker than they look. Name the limits yourself, and point to a systems view where it helps [[cite:lamport1994]].",
            "Explica qué significan los resultados, cómo encajan con trabajos anteriores y dónde son más débiles de lo que parecen. Nombra tú las limitaciones y apunta a una visión de conjunto cuando ayude [[cite:lamport1994]].",
        )],
    );
    chapter(
        &mut b,
        lang,
        "Conclusions",
        "Conclusiones",
        [(
            "",
            "",
            "Answer the question from the introduction in two or three sentences, and say what should be done next.",
            "Responde a la pregunta de la introducción en dos o tres frases y di qué debería hacerse a continuación.",
        )],
    );
    b.style(lang.pick("Acknowledgements", "Agradecimientos"), "FrontHeading");
    b.text(lang.pick(
        "Thank those who helped, and name the funders with the grant numbers they ask for.",
        "Da las gracias a quienes ayudaron y nombra a quienes financiaron, con los números de proyecto que pidan.",
    ));
    b.make(Make::Bibliography { title: lang.pick("References", "Referencias") });

    d.last_section = SectionProps {
        start: if two_columns { letra_doc::section::SectionStart::Continuous } else { letra_doc::section::SectionStart::NextPage },
        columns: if two_columns { letra_doc::section::Columns { count: 2, space: 18.0, ..Default::default() } } else { Default::default() },
        footers,
        ..page
    };
    let numbering = Numbering { preset: if two_columns { "outlineHeadings" } else { "headings" }, chapters: false };
    finish(d, b, lang, &numbering)
}

/// A book: half-title, title, copyright and dedication pages, a contents and a preface in roman
/// numerals, two parts of two chapters each, every chapter opening on a right-hand page under
/// a sunk title, running heads that change from page to page, mirrored margins, and the back
/// matter (acknowledgements, an index, a colophon).
pub fn book(lang: Lang) -> Document {
    let look = Look {
        body: 10.5,
        spacing: 1.25,
        h1: 28.0,
        h1_space: (96.0, 30.0),
        h1_break: false,
        h1_caps: false,
        h1_align: Align::Left,
        h2: 14.0,
        h3: 11.5,
        title: 38.0,
    };
    let mut d = Document::new();
    d.core.title = lang.pick("Title of the book", "Título del libro").into();
    d.core.creator = "Letra".into();
    d.settings.mirror_margins = true;
    d.settings.even_odd_headers = true;
    add_styles(&mut d, lang, &look);
    {
        let styles = Arc::make_mut(&mut d.styles);
        styles.upsert(Style {
            id: "FirstPara".into(),
            name: "First paragraph".into(),
            kind: StyleKind::Paragraph,
            based_on: Some("Normal".into()),
            para: ParaProps { indent_first: Some(0.0), space_after: Some(0.0), ..ParaProps::default() },
            priority: Some(13),
            quick: true,
            ..Style::default()
        });
        for (id, name, size, caps) in [("PartLabel", "Part label", 12.0, true), ("PartTitle", "Part title", 30.0, false)] {
            styles.upsert(Style {
                id: id.into(),
                name: name.into(),
                kind: StyleKind::Paragraph,
                based_on: Some("Normal".into()),
                next: Some("Normal".into()),
                para: ParaProps {
                    align: Some(Align::Center),
                    space_after: Some(10.0),
                    keep_next: Some(true),
                    indent_first: Some(0.0),
                    outline_level: (id == "PartTitle").then_some(0),
                    ..ParaProps::default()
                },
                chr: CharProps {
                    font: Some("Calibri".into()),
                    size: Some(size),
                    bold: Some(true),
                    caps: caps.then_some(true),
                    color: colour(ACCENT),
                    ..CharProps::default()
                },
                priority: Some(14),
                quick: true,
                ..Style::default()
            });
        }
        // Where a heading of the front or back matter starts: a page of its own.
        for id in ["FrontHeading", "AppendixHeading"] {
            if let Some(st) = styles.get_mut(id) {
                st.para.page_break_before = Some(true);
                st.para.space_before = Some(96.0);
            }
        }
    }
    d.sources = thesis_sources();
    let (w, h) = match lang {
        Lang::En => (432.0, 648.0),
        Lang::Es => (419.5, 595.3),
    };
    let page = SectionProps {
        page_w: w,
        page_h: h,
        margin_top: 60.0,
        margin_bottom: 62.0,
        margin_left: 62.0,
        margin_right: 48.0,
        header: 30.0,
        footer: 30.0,
        ..SectionProps::default()
    };
    // Blank bands for the pages that carry none (parts, half-title) and the running ones.
    let blank_header = d.add_part(PartKind::Header, vec![para_block(Paragraph::new().styled("Header"))]);
    let blank_footer = d.add_part(PartKind::Footer, vec![para_block(Paragraph::new().styled("Footer"))]);
    let dim = CharProps { color: colour(DIM), size: Some(9.0), ..Default::default() };
    let aligned = |align: Align, instr: Option<&str>, text: &str, italic: bool| {
        let mut p = Paragraph::new();
        p.props.align = Some(align);
        let props = CharProps { italic: italic.then_some(true), ..dim.clone() };
        if let Some(i) = instr {
            let _ = p.insert_object(0, field(i, ""), &props);
        } else {
            let _ = p.insert_text(0, text, &props);
        }
        p
    };
    let head_odd = d.add_part(PartKind::Header, vec![para_block(aligned(Align::Right, Some("STYLEREF 1"), "", true).styled("Header"))]);
    let head_even = d.add_part(
        PartKind::Header,
        vec![para_block(aligned(Align::Left, None, lang.pick("Title of the book", "Título del libro"), true).styled("Header"))],
    );
    let foot_odd = d.add_part(PartKind::Footer, vec![para_block(aligned(Align::Right, Some("PAGE"), "", false).styled("Footer"))]);
    let foot_even = d.add_part(PartKind::Footer, vec![para_block(aligned(Align::Left, Some("PAGE"), "", false).styled("Footer"))]);
    let set = |default: u32, even: u32| letra_doc::section::HeaderSet { default: Some(default), even: Some(even), ..Default::default() };
    let none = SectionProps { headers: set(blank_header, blank_header), footers: set(blank_footer, blank_footer), ..page.clone() };
    let running = SectionProps {
        start: letra_doc::section::SectionStart::OddPage,
        headers: set(head_odd, head_even),
        footers: set(foot_odd, foot_even),
        ..page.clone()
    };
    let author = lang.pick("Author Name", "Nombre del Autor");

    let mut b = Draft::new();
    // Half-title, title page, copyright page and dedication, each on a page of its own.
    let mut half = styled(lang.pick("Title of the book", "Título del libro"), "CoverLine");
    half.props.space_before = Some(150.0);
    b.push(half);
    let mut title = styled(lang.pick("Title of the book", "Título del libro"), "Title");
    title.props.page_break_before = Some(true);
    title.props.space_before = Some(110.0);
    b.push(title);
    b.style(lang.pick("A subtitle that says what kind of book it is", "Un subtítulo que diga qué tipo de libro es"), "Subtitle");
    let mut by = styled(author, "CoverLine");
    by.props.space_before = Some(90.0);
    b.push(by);
    b.push(styled(lang.pick("Publisher · City", "Editorial · Ciudad"), "CoverLine"));
    let mut copyright = styled(&format!("© {} {author}", lang.long_date().rsplit(' ').next().unwrap_or("")), "CoverLine");
    copyright.props.page_break_before = Some(true);
    copyright.props.space_before = Some(330.0);
    copyright.props.align = Some(Align::Left);
    b.push(copyright);
    for line in [
        lang.pick(
            "All rights reserved. No part of this book may be reproduced without the publisher's permission.",
            "Reservados todos los derechos. Ninguna parte de este libro puede reproducirse sin permiso de la editorial.",
        ),
        lang.pick("First edition · ISBN 000-0-00-000000-0", "Primera edición · ISBN 000-0-00-000000-0"),
        lang.pick("Typeset in Letra.", "Maquetado con Letra."),
    ] {
        let mut p = styled(line, "CoverLine");
        p.props.align = Some(Align::Left);
        format_small(&mut p);
        b.push(p);
    }
    let mut dedication = styled(lang.pick("For everyone who read the drafts.", "Para quienes leyeron los borradores."), "CoverLine");
    dedication.props.page_break_before = Some(true);
    dedication.props.space_before = Some(170.0);
    let n = dedication.len();
    let _ = dedication.format(0, n, &|c| c.italic = Some(true));
    b.push(dedication);
    b.end_with(SectionProps { start: letra_doc::section::SectionStart::NextPage, ..none.clone() });

    // Contents and preface, in roman numerals.
    b.make(Make::Toc { title: lang.pick("Contents", "Índice"), levels: 1 });
    b.end_with(SectionProps { page_num_format: NumFormat::LowerRoman, page_num_start: Some(1), ..running.clone() });
    b.style(lang.pick("Preface", "Prólogo"), "FrontHeading");
    for text in book_text(lang, 0) {
        b.push(book_paragraph(text, b.blocks.last().is_some_and(is_heading)));
    }
    b.end_with(SectionProps { page_num_format: NumFormat::LowerRoman, ..running.clone() });

    // The parts and their chapters.
    let parts = [("I", "The question", "La pregunta"), ("II", "The answer", "La respuesta")];
    let chapters: [[(&str, &str); 2]; 2] = [
        [("Where it begins", "Dónde empieza"), ("The first obstacle", "El primer obstáculo")],
        [("What was found", "Lo que se encontró"), ("What it means", "Lo que significa")],
    ];
    let mut first_body = true;
    for (pi, (num, title_en, title_es)) in parts.iter().enumerate() {
        let mut label = styled(&format!("{} {num}", lang.pick("Part", "Parte")), "PartLabel");
        label.props.space_before = Some(150.0);
        b.push(label);
        b.style(lang.pick(title_en, title_es), "PartTitle");
        b.end_with(SectionProps { valign: PageVAlign::Center, page_num_start: first_body.then_some(1), ..running.clone() });
        first_body = false;
        for (ci, (c_en, c_es)) in chapters[pi].iter().enumerate() {
            b.style(lang.pick(c_en, c_es), "Heading1");
            if pi == 0 && ci == 0 {
                let mut epigraph = styled(
                    lang.pick(
                        "\u{201C}A book is a long conversation.\u{201D} (Anonymous)",
                        "\u{ab}Un libro es una larga conversación.\u{bb} (Anónimo)",
                    ),
                    "Abstract",
                );
                epigraph.props.align = Some(Align::Right);
                epigraph.props.space_after = Some(20.0);
                let n = epigraph.len();
                let _ = epigraph.format(0, n, &|c| c.italic = Some(true));
                b.push(epigraph);
            }
            let texts = book_text(lang, 1 + pi * 2 + ci);
            for (k, text) in texts.iter().enumerate() {
                let mut p = book_paragraph(text, k == 0);
                if k == 0 {
                    p.props.drop_cap = Some(2);
                }
                b.push(p);
            }
            if ci == 0 {
                b.style(lang.pick("A section inside the chapter", "Un apartado dentro del capítulo"), "Heading2");
                let extra = book_text(lang, 5);
                b.push(book_paragraph(extra[0], true));
                b.push(book_paragraph(extra[1], false));
            }
            b.end_with(running.clone());
        }
    }

    // Back matter.
    b.style(lang.pick("Acknowledgements", "Agradecimientos"), "FrontHeading");
    b.push(book_paragraph(
        lang.pick(
            "Thank the people who made the book: editors, readers, the ones who waited.",
            "Da las gracias a quienes hicieron el libro: editores, lectores, quienes esperaron.",
        ),
        true,
    ));
    b.entries = match lang {
        Lang::En => vec!["chapter", "evidence", "heading", "preface"],
        Lang::Es => vec!["capítulo", "prueba", "título", "prólogo"],
    };
    b.make(Make::Index { title: lang.pick("Index", "Índice analítico") });
    b.style(lang.pick("Colophon", "Colofón"), "FrontHeading");
    b.push(book_paragraph(
        lang.pick(
            "This book was set in Cambria and Calibri and typeset in Letra on A5 pages.",
            "Este libro se compuso en Cambria y Calibri y se maquetó con Letra en páginas de formato A5.",
        ),
        true,
    ));
    d.last_section = running.clone();
    finish(d, b, lang, &Numbering { preset: "headings", chapters: false })
}

fn is_heading(b: &Block) -> bool {
    b.as_para().is_some_and(|p| matches!(p.props.style.as_deref(), Some("FrontHeading" | "Heading1" | "Heading2" | "PartTitle")))
}

/// A book paragraph: justified, with the first-line indent a book uses, or none right after a
/// heading.
fn book_paragraph(text: &str, first: bool) -> Paragraph {
    let mut p = para(text);
    p.props.space_after = Some(0.0);
    p.props.indent_first = Some(if first { 0.0 } else { 14.0 });
    p
}

/// The text of the `n`th passage of the book: guidance, written as the prose it would become.
fn book_text(lang: Lang, n: usize) -> [&'static str; 3] {
    let en: [[&str; 3]; 6] = [
        [
            "A preface says why the book exists and who it is for, in the author's own voice. Keep it short; readers who like the preface will read the book anyway.",
            "It can also say what the book is not, which saves both sides some disappointment.",
            "Sign it with a place and a date.",
        ],
        [
            "Open the chapter with something particular: a person, a place, a question someone asked. The general point can come in the second paragraph, once the reader has a reason to want it.",
            "A chapter is a unit of attention. Each should do one job, and the title should say which.",
            "End where the next chapter can begin, and not before.",
        ],
        [
            "Obstacles are where a story shows what it is made of. Say what stood in the way, and what it cost to go round it, in the order it happened.",
            "Readers forgive a lot in a long book, but not confusion about where they are. Remind them, lightly.",
            "When a detail does not change what the reader does or feels, leave it for the notes.",
        ],
        [
            "Report what was found as plainly as you can. A claim followed by its evidence is easier to follow than the reverse.",
            "Where the evidence is thin, say so in the same sentence. Trust builds from that.",
            "Keep tables and figures near the sentence that needs them.",
        ],
        [
            "Say what the findings mean for the person who has read this far, and what they might do differently on Monday.",
            "The strongest ending is not a summary but a consequence.",
            "Leave the last line for something the reader can carry out of the room.",
        ],
        [
            "A section heading inside a chapter marks a change of subject within a single idea. Use them sparingly, and use the same size for the same kind of change.",
            "Running heads on the page show the chapter's title on the right-hand page and the book's on the left.",
            "Paragraphs after a heading start flush left; the others are indented, as in a printed book.",
        ],
    ];
    let es: [[&str; 3]; 6] = [
        [
            "Un prólogo dice por qué existe el libro y para quién es, con la voz del autor. Que sea breve; quien disfruta el prólogo leerá el libro de todos modos.",
            "También puede decir lo que el libro no es, y así se ahorra alguna decepción.",
            "Fírmalo con un lugar y una fecha.",
        ],
        [
            "Abre el capítulo con algo concreto: una persona, un lugar, una pregunta que alguien hizo. La idea general puede llegar en el segundo párrafo, cuando el lector ya tenga un motivo para quererla.",
            "Un capítulo es una unidad de atención. Cada uno debe hacer un trabajo, y el título debe decir cuál.",
            "Termina donde pueda empezar el siguiente capítulo, y no antes.",
        ],
        [
            "Los obstáculos son donde una historia muestra de qué está hecha. Di qué se interpuso y lo que costó rodearlo, en el orden en que ocurrió.",
            "Los lectores perdonan mucho en un libro largo, pero no la confusión sobre dónde están. Recuérdaselo, con ligereza.",
            "Cuando un detalle no cambia lo que el lector hace o siente, déjalo para las notas.",
        ],
        [
            "Presenta lo que se encontró con la mayor claridad posible. Una afirmación seguida de su prueba se sigue mejor que al revés.",
            "Donde la prueba sea escasa, dilo en la misma frase. La confianza se construye así.",
            "Mantén las tablas y las figuras cerca de la frase que las necesita.",
        ],
        [
            "Di qué significan los hallazgos para quien ha llegado hasta aquí y qué podría hacer de otra manera el lunes.",
            "El mejor final no es un resumen sino una consecuencia.",
            "Deja la última línea para algo que el lector pueda llevarse de la sala.",
        ],
        [
            "Un apartado dentro del capítulo marca un cambio de tema dentro de una misma idea. Úsalos con mesura y con el mismo tamaño para el mismo tipo de cambio.",
            "Los titulillos de la página muestran el título del capítulo en la página derecha y el del libro en la izquierda.",
            "Los párrafos después de un título empiezan sin sangría; los demás llevan sangría, como en un libro impreso.",
        ],
    ];
    match lang {
        Lang::En => en[n.min(5)],
        Lang::Es => es[n.min(5)],
    }
}

fn format_small(p: &mut Paragraph) {
    let n = p.len();
    if n > 0 {
        let _ = p.format(0, n, &|c| c.size = Some(9.5));
    }
}

/// A table of text cells whose header row is aligned like the numbers under it.
fn numeric_table(rows: &[&[&str]], width: f32, right_from: usize) -> letra_doc::Table {
    let mut t = text_table(rows, width, right_from);
    quiet_table(&mut t, false);
    if let Some(head) = t.rows.first_mut() {
        for cell in head.cells.iter_mut().skip(right_from) {
            for block in &mut cell.blocks {
                if let Block::Para(p) = Arc::make_mut(block) {
                    p.props.align = Some(Align::Right);
                }
            }
        }
    }
    t
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
        if !t_en.is_empty() {
            b.style(lang.pick(t_en, t_es), "Heading2");
        }
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

/// How a template numbers its headings, and whether captions and equations restart per chapter.
struct Numbering {
    /// A Multilevel List preset: `headings` (1, 1.1) or `outlineHeadings` (I., A., 1.).
    preset: &'static str,
    chapters: bool,
}

/// Run the scratch session over the draft: number the headings, then make everything the engine
/// makes, last to first so that the positions still to come stay where they were.
fn finish(mut d: Document, b: Draft, lang: Lang, numbering: &Numbering) -> Document {
    let Draft { mut blocks, makes, entries } = b;
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
        let _ = s.run("list.multilevelPreset", &json!({"preset": numbering.preset}));
    }
    for entry in &entries {
        let _ = s.run("references.markEntry", &json!({"entry": entry, "text": entry, "all": true}));
    }
    for (at, make) in makes.iter().rev() {
        s.sel = crate::Selection::caret(letra_doc::Pos::body(*at, 0));
        let (command, params): (&str, Value) = match make {
            Make::Toc { title, levels } => ("references.toc", json!({"levels": levels, "title": title})),
            Make::Figures { label, title } => {
                heading_above(&mut s, *at, title);
                ("references.tableOfFigures", json!({"label": localised_label(label, lang)}))
            }
            Make::Bibliography { title } => {
                heading_above(&mut s, *at, title);
                ("references.bibliography", json!({"title": ""}))
            }
            Make::Index { title } => {
                heading_above(&mut s, *at, title);
                ("references.index", json!({"columns": 2, "headings": true, "title": ""}))
            }
            Make::Caption { label, text, above } => {
                let params = chapter_numbered(
                    json!({"label": localised_label(label, lang), "text": text, "position": if *above { "above" } else { "below" }}),
                    numbering,
                );
                ("references.caption", params)
            }
            Make::Equation { latex } => {
                let _ = s.run("insert.equation", &json!({"latex": latex, "display": false, "edit": false}));
                ("references.numberEquation", chapter_numbered(json!({"label": localised_label("Equation", lang)}), numbering))
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

/// A heading of the front or back matter at block `at`, with the caret left on the block after it
/// where a generated list goes.
fn heading_above(s: &mut Session, at: usize, title: &str) {
    let _ = s.doc.insert_block(letra_doc::StoryRef::Body, &letra_doc::Path::top(at), Block::Para(styled(title, "FrontHeading")));
    s.sel = crate::Selection::caret(letra_doc::Pos::body(at + 1, 0));
}

/// `params` of a caption or equation number, counting from 1 in each chapter when the document
/// numbers its chapters.
fn chapter_numbered(mut params: Value, numbering: &Numbering) -> Value {
    if numbering.chapters {
        params["chapter"] = json!(1);
        params["separator"] = json!(".");
    }
    params
}

/// Remove the empty paragraph a generated list leaves before the next heading: with the page
/// break the heading brings, it would be a page of its own whenever the list fills its page.
fn drop_blank_before_headings(d: &mut Document) {
    let starts_page = |b: &Block| {
        b.as_para()
            .is_some_and(|p| matches!(p.props.style.as_deref(), Some("FrontHeading" | "Heading1" | "AppendixHeading" | "TOCHeading" | "PartLabel")))
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

/// A school textbook: units that open with what the student will learn, lessons with worked
/// examples, "remember" boxes, exercises, a summary, then a glossary and the answers.
pub fn textbook(lang: Lang) -> Document {
    let mut d = Document::new();
    d.core.title = lang.pick("Mathematics 6", "Matemáticas 6").into();
    d.core.creator = "Letra".into();
    add_styles(&mut d, lang, &TEXTBOOK_LOOK);
    d.settings.mirror_margins = true;
    // The heading above a set of exercises is a small heading with no number.
    Arc::make_mut(&mut d.styles).upsert(Style {
        id: "ExerciseHeading".into(),
        name: "Exercise heading".into(),
        kind: StyleKind::Paragraph,
        based_on: Some("Heading3".into()),
        next: Some("Normal".into()),
        para: ParaProps { numbering: Some(NumRef { num: 0, level: 0 }), ..ParaProps::default() },
        priority: Some(13),
        quick: true,
        ..Style::default()
    });
    let page = SectionProps { margin_left: 70.0, margin_right: 62.0, ..page(lang) };
    let mut num = (*d.numbering).clone();
    let exercise_lists: Vec<u32> = (0..4).map(|_| num.add_list(letra_doc::ListKind::Numbered)).collect();
    d.numbering = Arc::new(num);
    let mut b = Draft::new();

    // Cover.
    b.push(cover_line(lang.pick("Student book", "Libro del alumnado")));
    b.push(styled("", "CoverLine"));
    b.style(lang.pick("Mathematics 6", "Matemáticas 6"), "Title");
    b.style(lang.pick("Fractions, ratios and the habit of checking", "Fracciones, razones y el hábito de comprobar"), "Subtitle");
    b.push(styled("", "CoverLine"));
    b.push(cover_line(lang.pick("Grade 6 · Authors' names", "Sexto curso · Nombres de las autoras y los autores")));
    b.push(cover_line(lang.pick("Publisher · City", "Editorial · Ciudad")));
    b.end_with(SectionProps { valign: PageVAlign::Center, ..page.clone() });

    // Front matter.
    b.style(lang.pick("How to use this book", "Cómo usar este libro"), "FrontHeading");
    b.text(lang.pick(
        "Every unit starts with what you will be able to do by the end. Read it first, and come back to it when you finish.",
        "Cada unidad empieza con lo que sabrás hacer al terminarla. Léelo primero y vuelve a ello cuando acabes.",
    ));
    for (label, tone, text) in [
        (
            lang.pick("In this unit", "En esta unidad"),
            Tone::Goals,
            lang.pick("a short list of what you will learn.", "una lista breve de lo que vas a aprender."),
        ),
        (
            lang.pick("Worked example", "Ejemplo resuelto"),
            Tone::Example,
            lang.pick(
                "a problem solved step by step. Cover the answer and try it first.",
                "un problema resuelto paso a paso. Tapa la solución e inténtalo antes.",
            ),
        ),
        (
            lang.pick("Remember", "Recuerda"),
            Tone::Remember,
            lang.pick("the idea to keep, in one sentence.", "la idea que debes conservar, en una frase."),
        ),
    ] {
        b.push(boxed(label, text, tone));
    }
    b.make(Make::Toc { title: lang.pick("Contents", "Índice"), levels: 2 });
    let front_footer = footer(&mut d);
    b.end_section(SectionProps {
        page_num_format: NumFormat::LowerRoman,
        page_num_start: Some(1),
        ..SectionProps { footers: letra_doc::section::HeaderSet { default: Some(front_footer), ..Default::default() }, ..page.clone() }
    });

    // Unit 1.
    b.style(lang.pick("Fractions", "Fracciones"), "Heading1");
    b.push(boxed(
        lang.pick("In this unit", "En esta unidad"),
        lang.pick(
            "you will read a fraction as a part of a whole, compare two fractions, and add fractions with the same and with different denominators.",
            "leerás una fracción como parte de un todo, compararás dos fracciones y sumarás fracciones con el mismo y con distinto denominador.",
        ),
        Tone::Goals,
    ));
    b.style(lang.pick("Parts of a whole", "Partes de un todo"), "Heading2");
    b.text(lang.pick(
        "A fraction tells how many equal parts of a whole you take. In 3/4, the whole is cut into four equal parts and you take three. The number below the line is the denominator; the number above it is the numerator.",
        "Una fracción dice cuántas partes iguales de un todo tomas. En 3/4, el todo se corta en cuatro partes iguales y tomas tres. El número de abajo es el denominador; el de arriba, el numerador.",
    ));
    b.push(boxed(
        lang.pick("Remember", "Recuerda"),
        lang.pick(
            "the parts must be equal. Three unequal pieces of a pizza are not three quarters.",
            "las partes deben ser iguales. Tres trozos desiguales de una pizza no son tres cuartos.",
        ),
        Tone::Remember,
    ));
    b.style(lang.pick("Adding fractions", "Sumar fracciones"), "Heading2");
    b.text(lang.pick(
        "When the denominators are the same, the parts are the same size, so you only count them. Add the numerators and keep the denominator:",
        "Cuando los denominadores son iguales, las partes son del mismo tamaño y solo hay que contarlas. Suma los numeradores y conserva el denominador:",
    ));
    b.make(Make::Equation { latex: r"\frac{a}{c} + \frac{b}{c} = \frac{a+b}{c}" });
    b.push(boxed(
        lang.pick("Worked example", "Ejemplo resuelto"),
        lang.pick(
            "Lucía eats 2/8 of a tray of lasagne and Mateo eats 3/8. Together they eat 2/8 + 3/8 = 5/8 of the tray, so 3/8 is left.",
            "Lucía se come 2/8 de una bandeja de lasaña y Mateo, 3/8. Entre los dos se comen 2/8 + 3/8 = 5/8 de la bandeja, así que quedan 3/8.",
        ),
        Tone::Example,
    ));
    let fractions = match lang {
        Lang::En => [&["Fraction", "Decimal", "Percentage"][..], &["1/2", "0.5", "50 %"], &["1/4", "0.25", "25 %"]],
        Lang::Es => [&["Fracción", "Decimal", "Porcentaje"][..], &["1/2", "0,5", "50 %"], &["1/4", "0,25", "25 %"]],
    };
    b.captioned(
        Block::Table(numeric_table(&fractions, 300.0, 1)),
        "Table",
        lang.pick("The same quantity written three ways", "La misma cantidad escrita de tres maneras"),
        true,
    );
    b.style(lang.pick("Exercises", "Ejercicios"), "ExerciseHeading");
    for t in [
        lang.pick("Colour 5/6 of a strip cut into six equal parts.", "Colorea 5/6 de una tira dividida en seis partes iguales."),
        lang.pick("Which is larger, 3/5 or 4/5? Explain how you know.", "¿Cuál es mayor, 3/5 o 4/5? Explica cómo lo sabes."),
        lang.pick("Calculate 2/9 + 4/9 and 1/7 + 5/7.", "Calcula 2/9 + 4/9 y 1/7 + 5/7."),
    ] {
        b.push(numbered(t, exercise_lists[0]));
    }
    b.push(boxed(
        lang.pick("Summary", "Resumen"),
        lang.pick(
            "A fraction is parts of a whole. To add fractions with the same denominator, add the numerators and keep the denominator.",
            "Una fracción son partes de un todo. Para sumar fracciones con el mismo denominador, se suman los numeradores y se conserva el denominador.",
        ),
        Tone::Goals,
    ));

    // Unit 2.
    b.style(lang.pick("Ratios and proportions", "Razones y proporciones"), "Heading1");
    b.push(boxed(
        lang.pick("In this unit", "En esta unidad"),
        lang.pick(
            "you will compare two quantities with a ratio, find equivalent ratios and use them to scale a recipe or a map.",
            "compararás dos cantidades con una razón, encontrarás razones equivalentes y las usarás para ampliar una receta o un mapa.",
        ),
        Tone::Goals,
    ));
    b.style(lang.pick("Comparing quantities", "Comparar cantidades"), "Heading2");
    b.text(lang.pick(
        "A ratio compares two quantities of the same kind. If a drink uses 2 cups of juice for every 3 cups of water, the ratio of juice to water is 2 to 3, written 2 : 3.",
        "Una razón compara dos cantidades de la misma clase. Si una bebida lleva 2 vasos de zumo por cada 3 de agua, la razón de zumo a agua es de 2 a 3, y se escribe 2 : 3.",
    ));
    b.push(boxed(
        lang.pick("Worked example", "Ejemplo resuelto"),
        lang.pick(
            "To make 10 cups of the same drink, keep the ratio: 2 : 3 = 4 : 6 = 6 : 9. Five groups of (2 + 3) cups make 10 cups, so you need 4 cups of juice and 6 of water.",
            "Para hacer 10 vasos de la misma bebida, mantén la razón: 2 : 3 = 4 : 6 = 6 : 9. Dos grupos de (2 + 3) vasos hacen 10 vasos, así que necesitas 4 de zumo y 6 de agua.",
        ),
        Tone::Example,
    ));
    b.style(lang.pick("Scaling", "Ampliar y reducir"), "Heading2");
    b.text(lang.pick(
        "A map drawn at 1 : 50 000 shows 1 cm for every 50 000 cm on the ground, which is 500 m. Multiply both parts of a ratio by the same number and it still says the same thing.",
        "Un mapa dibujado a escala 1 : 50 000 muestra 1 cm por cada 50 000 cm del terreno, es decir, 500 m. Si multiplicas las dos partes de una razón por el mismo número, sigue diciendo lo mismo.",
    ));
    b.push(boxed(
        lang.pick("Remember", "Recuerda"),
        lang.pick(
            "equal ratios come from multiplying or dividing both parts by the same number.",
            "las razones iguales salen de multiplicar o dividir las dos partes por el mismo número.",
        ),
        Tone::Remember,
    ));
    b.style(lang.pick("Exercises", "Ejercicios"), "ExerciseHeading");
    for t in [
        lang.pick(
            "Write the ratio of boys to girls in a class of 12 boys and 18 girls, in its simplest form.",
            "Escribe la razón de chicos a chicas en una clase de 12 chicos y 18 chicas, en su forma más simple.",
        ),
        lang.pick(
            "A recipe for 4 people uses 300 g of rice. How much rice for 10 people?",
            "Una receta para 4 personas lleva 300 g de arroz. ¿Cuánto arroz hace falta para 10 personas?",
        ),
        lang.pick(
            "Two cities are 4 cm apart on a 1 : 250 000 map. How far apart are they in kilometres?",
            "Dos ciudades están a 4 cm en un mapa 1 : 250 000. ¿A cuántos kilómetros están?",
        ),
    ] {
        b.push(numbered(t, exercise_lists[1]));
    }
    b.push(boxed(
        lang.pick("Summary", "Resumen"),
        lang.pick(
            "A ratio compares two quantities. Equal ratios keep the same relation when both parts are scaled together.",
            "Una razón compara dos cantidades. Las razones iguales mantienen la misma relación cuando las dos partes se escalan a la vez.",
        ),
        Tone::Goals,
    ));

    // Back matter.
    b.style(lang.pick("Glossary", "Glosario"), "FrontHeading");
    for (term, meaning) in [
        (
            lang.pick("Denominator", "Denominador"),
            lang.pick(
                "the number below the line: how many equal parts the whole has.",
                "el número de abajo: en cuántas partes iguales se divide el todo.",
            ),
        ),
        (
            lang.pick("Numerator", "Numerador"),
            lang.pick("the number above the line: how many parts are taken.", "el número de arriba: cuántas partes se toman."),
        ),
        (
            lang.pick("Ratio", "Razón"),
            lang.pick("a comparison of two quantities of the same kind.", "una comparación entre dos cantidades de la misma clase."),
        ),
        (
            lang.pick("Scale", "Escala"),
            lang.pick("the ratio between a length on a drawing and the real one.", "la razón entre una longitud en un dibujo y la real."),
        ),
    ] {
        let mut p = para("");
        p.props.align = Some(Align::Left);
        add(&mut p, term, CharProps { bold: Some(true), color: colour(ACCENT), ..Default::default() });
        add(&mut p, &format!(": {meaning}"), CharProps::default());
        b.push(p);
    }
    b.style(lang.pick("Answers to the exercises", "Soluciones de los ejercicios"), "AppendixHeading");
    b.text(lang.pick("Unit 1.", "Unidad 1."));
    for t in [
        lang.pick("Five of the six parts coloured.", "Cinco de las seis partes coloreadas."),
        lang.pick("4/5, because it has more equal parts.", "4/5, porque tiene más partes iguales."),
        lang.pick("6/9 and 6/7.", "6/9 y 6/7."),
    ] {
        b.push(numbered(t, exercise_lists[2]));
    }
    b.text(lang.pick("Unit 2.", "Unidad 2."));
    for t in [lang.pick("2 : 3.", "2 : 3."), lang.pick("750 g.", "750 g."), lang.pick("10 km.", "10 km.")] {
        b.push(numbered(t, exercise_lists[3]));
    }

    d.last_section = chapter_body_section(&mut d, page);
    finish(d, b, lang, &Numbering { preset: "headings", chapters: true })
}

/// A textbook: a larger, more open page than a thesis, with big unit titles.
const TEXTBOOK_LOOK: Look = Look {
    body: 12.0,
    spacing: 1.3,
    h1: 28.0,
    h1_space: (0.0, 24.0),
    h1_break: true,
    h1_caps: false,
    h1_align: Align::Left,
    h2: 17.0,
    h3: 13.0,
    title: 38.0,
};

/// What a box in a textbook is for, which sets its colour.
#[derive(Clone, Copy)]
enum Tone {
    /// What a unit teaches and what it came to.
    Goals,
    /// A solved problem.
    Example,
    /// The idea to keep.
    Remember,
}

/// A tinted box with a bold label and a text after it.
fn boxed(label: &str, text: &str, tone: Tone) -> Paragraph {
    let (shade, bar) = match tone {
        Tone::Goals => (Rgb(0xEC, 0xF3, 0xF4), ACCENT),
        Tone::Example => (Rgb(0xFB, 0xF3, 0xE2), Rgb(0xC9, 0x7F, 0x1A)),
        Tone::Remember => (Rgb(0xE8, 0xF4, 0xEC), Rgb(0x2B, 0x8A, 0x57)),
    };
    let mut p = para("");
    add(&mut p, label, CharProps { bold: Some(true), color: colour(bar), ..Default::default() });
    add(&mut p, &format!(" {text}"), CharProps::default());
    callout(&mut p);
    p.props.shading = Some(shade);
    p.props.align = Some(Align::Left);
    p.props.keep_lines = Some(true);
    p.props.borders = Some(Borders {
        left: Some(line(3.0, bar, 8.0)),
        top: Some(line(0.0, shade, 6.0)),
        bottom: Some(line(0.0, shade, 6.0)),
        ..Default::default()
    });
    p
}

/// An item of the numbered list `list`.
fn numbered(text: &str, list: u32) -> Paragraph {
    let mut p = para(text).styled("ListParagraph");
    p.props.numbering = Some(NumRef { num: list, level: 0 });
    p.props.align = Some(Align::Left);
    p
}

/// Whether template `id` is one of the long templates, which carry their own design.
pub(crate) fn is_long(id: &str) -> bool {
    matches!(id, "thesis" | "paper" | "paper-2col" | "book" | "textbook")
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

    /// The paper's two-column variant sets its title across the page and its body in two columns;
    /// the single-column one has no section break.
    #[test]
    fn the_papers_are_laid_out_in_one_or_two_columns() {
        let one = paper(Lang::En, false);
        assert!(one.body.iter().filter_map(|b| b.as_para()).all(|p| p.section.is_none()));
        assert_eq!(one.last_section.columns.count, 1);
        let two = paper(Lang::En, true);
        let title_block = two.body.iter().filter_map(|b| b.as_para()).find_map(|p| p.section.as_deref()).unwrap();
        assert_eq!(title_block.columns.count, 1, "the title and abstract span the page");
        assert_eq!(two.last_section.columns.count, 2);
        assert_eq!(two.last_section.start, letra_doc::section::SectionStart::Continuous);
    }

    /// Sections are numbered in the style of each variant, captions and the equation count
    /// through the paper (no chapters), and the references list what is cited.
    #[test]
    fn the_papers_arrive_complete() {
        for (lang, two, figure, table) in [(Lang::En, false, "Figure 1:", "Table 1:"), (Lang::Es, true, "Figura 1:", "Tabla 1:")] {
            let t = text(&paper(lang, two));
            assert!(t.contains(figure) && t.contains(table), "{lang:?} {two}: {t}");
            assert!(t.contains("(1)"), "{lang:?}: the equation is numbered through the paper");
            assert!(t.contains("Knuth, D. (1984)") && t.contains("Lamport, L. (1994)"), "{lang:?}: references");
            assert!(!t.contains("[[cite"), "{lang:?}: every citation mark became a field");
        }
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

    /// The book mirrors its margins, starts parts and chapters on right-hand pages, numbers its
    /// front matter in roman numerals, and has different running heads on left and right pages.
    #[test]
    fn the_book_is_set_for_print() {
        let d = book(Lang::En);
        assert!(d.settings.mirror_margins && d.settings.even_odd_headers);
        let ended: Vec<&SectionProps> = d.body.iter().filter_map(|b| b.as_para()).filter_map(|p| p.section.as_deref()).collect();
        assert!(ended.iter().any(|s| s.page_num_format == NumFormat::LowerRoman), "roman front matter");
        assert!(ended.iter().any(|s| s.valign == PageVAlign::Center), "part pages sit in the middle of the page");
        assert!(ended.iter().any(|s| s.start == letra_doc::section::SectionStart::OddPage), "chapters open on right-hand pages");
        assert!(d.last_section.headers.even.is_some() && d.last_section.headers.default.is_some());
        let l = letra_layout::layout(&d, &mut letra_layout::LayoutCache::new(), &Default::default());
        assert!(l.pages.len() >= 20, "{}", l.pages.len());
    }

    /// The contents, the numbered chapters and the index are made, in both languages.
    #[test]
    fn the_book_arrives_complete() {
        for (lang, heading, word) in [(Lang::En, "1\tWhere it begins", "evidence, "), (Lang::Es, "1\tDónde empieza", "prueba, ")] {
            let t = text(&book(lang));
            assert!(t.contains(heading), "{lang:?}: {t}");
            assert!(t.contains(word), "{lang:?}: the index lists marked words with their pages");
            assert_eq!(t.matches("\nIndex\n").count() + t.matches("\nÍndice analítico\n").count(), 1, "{lang:?}: one index heading");
        }
    }

    /// The textbook opens complete: numbered units, the contents, a numbered equation and table,
    /// the boxes, and exercises whose lists restart for each unit.
    #[test]
    fn the_textbook_arrives_complete() {
        for (lang, unit, example, table) in
            [(Lang::En, "1\tFractions", "Worked example", "Table 1.1:"), (Lang::Es, "1\tFracciones", "Ejemplo resuelto", "Tabla 1.1:")]
        {
            let d = textbook(lang);
            let t = text(&d);
            assert!(t.contains(unit), "{lang:?}: numbered units in the contents");
            assert!(t.contains(example) && t.contains(table) && t.contains("(1.1)"), "{lang:?}: {t}");
            let l = letra_layout::layout(&d, &mut letra_layout::LayoutCache::new(), &Default::default());
            assert_eq!(l.pages.len(), 7, "{lang:?}: cover, front matter, two units, glossary, answers");
            let lists: std::collections::BTreeSet<u32> = d
                .body
                .iter()
                .filter_map(|b| b.as_para())
                .filter(|p| p.props.style.as_deref() == Some("ListParagraph"))
                .filter_map(|p| p.props.numbering.map(|n| n.num))
                .collect();
            assert_eq!(lists.len(), 4, "{lang:?}: each set of exercises and answers numbers from 1");
        }
    }
}
