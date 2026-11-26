//! Built-in sample documents and templates (original text, generated in code), written in English
//! or in Spanish: a document opened from a Spanish interface arrives in Spanish.

use letra_doc::para::InlineObject;
use letra_doc::props::{Align, Border, BorderStyle, Borders, CharProps, NumRef, Rgb, TabAlign, TabLeader, TabStop, TextColor};
use letra_doc::{Block, Document, ListKind, Paragraph, PartKind, Table, para_block};
use std::sync::Arc;

pub(crate) fn para(text: &str) -> Paragraph {
    Paragraph::with_text(text, CharProps::default())
}

pub(crate) fn styled(text: &str, style: &str) -> Paragraph {
    para(text).styled(style)
}

/// Append text with formatting to a paragraph.
pub(crate) fn add(p: &mut Paragraph, text: &str, props: CharProps) {
    let at = p.len();
    let _ = p.insert_text(at, text, &props);
}

fn bold() -> CharProps {
    CharProps { bold: Some(true), ..Default::default() }
}
fn italic() -> CharProps {
    CharProps { italic: Some(true), ..Default::default() }
}

/// The language a built-in document is written in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    En,
    Es,
}

impl Lang {
    /// `"es"`, `"es-ES"` and `"es_MX.UTF-8"` are Spanish; anything else is English.
    pub fn parse(code: &str) -> Lang {
        let code = code.trim().to_ascii_lowercase();
        if code == "es" || code.starts_with("es-") || code.starts_with("es_") { Lang::Es } else { Lang::En }
    }

    /// `en` or `es`, by language.
    pub fn pick(self, en: &'static str, es: &'static str) -> &'static str {
        match self {
            Lang::En => en,
            Lang::Es => es,
        }
    }

    /// Today's date written out in full: `July 4, 2026` or `4 de julio de 2026`.
    pub fn long_date(self) -> String {
        const MONTHS: [&str; 12] =
            ["enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre"];
        match self {
            Lang::En => crate::cmd::insert::format_date("MMMM d, yyyy"),
            Lang::Es => {
                let (y, m, d, _, _) = crate::cmd::local_now();
                let month = MONTHS.get((m as usize).saturating_sub(1)).copied().unwrap_or("enero");
                format!("{d} de {month} de {y}")
            }
        }
    }
}

/// A built-in template: the id `file.new` takes and its name in each language.
pub struct Template {
    pub id: &'static str,
    pub en: &'static str,
    pub es: &'static str,
}

impl Template {
    /// Its name in `lang`.
    pub fn name(&self, lang: Lang) -> &'static str {
        lang.pick(self.en, self.es)
    }
}

/// The built-in templates in gallery order. The first is the blank document.
pub const TEMPLATES: [Template; 14] = [
    Template { id: "blank", en: "Blank document", es: "Documento en blanco" },
    Template { id: "sample", en: "Studio handbook (sample)", es: "Manual del taller (ejemplo)" },
    Template { id: "letter", en: "Letter", es: "Carta" },
    Template { id: "resume", en: "Résumé", es: "Currículum" },
    Template { id: "cover-letter", en: "Cover letter", es: "Carta de presentación" },
    Template { id: "report", en: "Report", es: "Informe" },
    Template { id: "minutes", en: "Meeting minutes", es: "Acta de reunión" },
    Template { id: "essay", en: "Essay", es: "Ensayo" },
    Template { id: "invoice", en: "Invoice", es: "Factura" },
    Template { id: "newsletter", en: "Newsletter", es: "Boletín" },
    Template { id: "proposal", en: "Project proposal", es: "Propuesta de proyecto" },
    Template { id: "flyer", en: "Event flyer", es: "Cartel de evento" },
    Template { id: "recipe", en: "Recipe", es: "Receta" },
    Template { id: "thesis", en: "Thesis / final project", es: "Tesis / TFG" },
];

/// The document a template id names, in `lang`. `blank` and unknown ids are not built here.
pub fn build(id: &str, lang: Lang) -> Option<Document> {
    let mut d = match id {
        "sample" => sample_document_in(lang),
        "letter" => letter_in(lang),
        "cover-letter" => cover_letter(lang),
        "resume" => resume_in(lang),
        "report" => report_in(lang),
        "minutes" => minutes(lang),
        "essay" => essay(lang),
        "invoice" => invoice(lang),
        "newsletter" => newsletter(lang),
        "proposal" => proposal(lang),
        "flyer" => flyer(lang),
        "recipe" => recipe(lang),
        "thesis" => crate::sample_long::thesis(lang),
        _ => return None,
    };
    // The sample handbook and the long templates have their own, richer design; every other
    // template shares one look.
    if id != "sample" && !crate::sample_long::is_long(id) {
        crate::sample_design::polish(id, &mut d);
    }
    Some(d)
}

/// A table of text cells with a header row; the body cells from column `right_from` on are
/// right-aligned (numbers).
pub(crate) fn text_table(rows: &[&[&str]], widths: f32, right_from: usize) -> Table {
    let cols = rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
    let mut t = Table::new(rows.len().max(1), cols, widths);
    t.props.style = Some("GridTable4AccentBlue".into());
    for (r, row) in rows.iter().enumerate() {
        for (c, txt) in row.iter().enumerate() {
            if let Some(cell) = t.rows.get_mut(r).and_then(|x| x.cells.get_mut(c)) {
                let mut p = para(txt);
                if r > 0 && c >= right_from {
                    p.props.align = Some(Align::Right);
                }
                cell.blocks = vec![para_block(p)];
            }
        }
    }
    if let Some(r) = t.rows.first_mut() {
        r.props.header = true;
    }
    t
}

/// A bulleted paragraph.
fn bullet(text: &str, list: u32) -> Block {
    let mut p = para(text).styled("ListParagraph");
    p.props.numbering = Some(NumRef { num: list, level: 0 });
    Block::Para(p)
}

/// The document body from blocks.
pub(crate) fn body(d: &mut Document, blocks: Vec<Block>) {
    d.body = blocks.into_iter().map(std::sync::Arc::new).collect();
}

/// The showcase document: a short studio handbook with headings, lists, a table, a quote,
/// a footer with page numbers.
pub fn sample_document() -> Document {
    sample_document_in(Lang::En)
}

/// [`sample_document`] in `lang`.
pub fn sample_document_in(lang: Lang) -> Document {
    let mut d = Document::new();
    d.core.title = lang.pick("The Open Studio Handbook", "El manual del taller abierto").into();
    d.core.creator = "Letra".into();
    let mut b: Vec<Block> = Vec::new();
    b.push(Block::Para(styled(lang.pick("The Open Studio Handbook", "El manual del taller abierto"), "Title")));
    b.push(Block::Para(styled(
        lang.pick(
            "How a small community of makers shares space, tools and time",
            "Cómo una pequeña comunidad de creadores comparte espacio, herramientas y tiempo",
        ),
        "Subtitle",
    )));
    let mut intro = para(lang.pick(
        "Every Thursday evening the old print shop on Alder Street fills with people who make things. ",
        "Todos los jueves por la tarde, la vieja imprenta de la calle del Aliso se llena de gente que hace cosas. ",
    ));
    add(&mut intro, lang.pick("Painters", "Los pintores"), bold());
    add(&mut intro, lang.pick(" set up by the north windows, ", " se instalan junto a las ventanas del norte, "), CharProps::default());
    add(&mut intro, lang.pick("letterpress", "los aficionados a la tipografía"), italic());
    add(
        &mut intro,
        lang.pick(
            " regulars claim the back room, and somebody always brings bread. This handbook collects what we have learned about running a shared studio: the routines that keep it pleasant, the tools we pool, and the small agreements that let thirty strangers become a crew.",
            " reclaman la sala del fondo y siempre alguien trae pan. Este manual recoge lo que hemos aprendido sobre cómo llevar un taller compartido: las rutinas que lo mantienen agradable, las herramientas que ponemos en común y los pequeños acuerdos que convierten a treinta desconocidos en un equipo.",
        ),
        CharProps::default(),
    );
    b.push(Block::Para(intro));
    b.push(Block::Para(styled(lang.pick("Why share a studio?", "¿Por qué compartir un taller?"), "Heading1")));
    b.push(Block::Para(para(lang.pick("Rent is the obvious reason, but it is not the best one. A shared room puts your work next to work you would never have seen otherwise. A ceramicist explains glaze chemistry to a comic artist; a photographer borrows a weaver's loom for a still life. The cross-talk is the point.", "El alquiler es la razón evidente, pero no la mejor. Una sala compartida pone tu trabajo junto a trabajos que de otro modo nunca habrías visto. Una ceramista le explica la química de los esmaltes a un dibujante de cómic; una fotógrafa le pide prestado el telar a un tejedor para un bodegón. Ese intercambio es lo que importa."))));
    let mut quote = styled(
        lang.pick("A studio is a place where unfinished things are welcome.", "Un taller es un lugar donde las cosas sin terminar son bienvenidas."),
        "Quote",
    );
    quote.props.space_before = Some(12.0);
    b.push(Block::Para(quote));
    b.push(Block::Para(styled(lang.pick("What we agree on", "En lo que estamos de acuerdo"), "Heading2")));
    let mut num = (*d.numbering).clone();
    let bullets = num.add_list(ListKind::Bullet);
    let steps = num.add_list(ListKind::Numbered);
    d.numbering = Arc::new(num);
    for (lead, rest) in [
        (
            lang.pick("Clean as you go. ", "Limpia sobre la marcha. "),
            lang.pick("Leave a table the way you would like to find it.", "Deja la mesa como te gustaría encontrarla."),
        ),
        (
            lang.pick("Label what is yours. ", "Etiqueta lo tuyo. "),
            lang.pick("Unlabelled supplies on the commons shelf are fair game.", "El material sin etiqueta del estante común es de todos."),
        ),
        (
            lang.pick("Ask before you borrow. ", "Pregunta antes de coger prestado. "),
            lang.pick(
                "Even from the shelf, a quick note helps everyone keep track.",
                "Incluso del estante, una nota rápida ayuda a llevar la cuenta.",
            ),
        ),
        (
            lang.pick("Quiet hours ", "El horario de silencio "),
            lang.pick("run from 9 to 11 on weekday mornings.", "va de 9 a 11 los días laborables por la mañana."),
        ),
    ] {
        let mut p = Paragraph::new().styled("ListParagraph");
        add(&mut p, lead, bold());
        add(&mut p, rest, CharProps::default());
        p.props.numbering = Some(NumRef { num: bullets, level: 0 });
        b.push(Block::Para(p));
    }
    b.push(Block::Para(styled(lang.pick("Shared equipment", "Equipo compartido"), "Heading2")));
    b.push(Block::Para(para(lang.pick(
        "The table below lists what the membership owns together and who keeps each item in working order.",
        "La tabla siguiente enumera lo que los miembros tienen en común y quién mantiene cada pieza en buen estado.",
    ))));
    let rows = match lang {
        Lang::En => [
            ["Equipment", "Location", "Steward", "Booking"],
            ["Etching press", "Back room", "Mara O.", "Sign-up sheet"],
            ["Kiln (cone 10)", "Yard shed", "Theo P.", "48 h notice"],
            ["Large-format printer", "Office", "Ines R.", "Calendar"],
            ["Light table", "North windows", "Anyone", "First come"],
        ],
        Lang::Es => [
            ["Equipo", "Lugar", "Responsable", "Reserva"],
            ["Prensa de grabado", "Sala del fondo", "Mara O.", "Hoja de turnos"],
            ["Horno (cono 10)", "Cobertizo del patio", "Theo P.", "48 h de aviso"],
            ["Impresora de gran formato", "Oficina", "Inés R.", "Calendario"],
            ["Mesa de luz", "Ventanas del norte", "Cualquiera", "Por orden de llegada"],
        ],
    };
    let mut t = Table::new(rows.len(), 4, 468.0);
    t.props.style = Some("GridTable4AccentBlue".into());
    for (r, row) in rows.iter().enumerate() {
        for (c, txt) in row.iter().enumerate() {
            if let Some(cell) = t.rows.get_mut(r).and_then(|x| x.cells.get_mut(c)) {
                cell.blocks = vec![para_block(para(txt))];
            }
        }
    }
    if let Some(r) = t.rows.first_mut() {
        r.props.header = true;
    }
    b.push(Block::Table(t));
    b.push(Block::Para(styled(lang.pick("Opening the studio", "Abrir el taller"), "Heading2")));
    for s in [
        lang.pick("Switch on the main breaker by the door.", "Sube el interruptor general junto a la puerta."),
        lang.pick("Open both skylights unless it is raining.", "Abre los dos tragaluces, salvo que llueva."),
        lang.pick("Start the kettle. This step is not optional.", "Pon el hervidor. Este paso no es opcional."),
        lang.pick("Check the booking sheet for the kiln and press.", "Mira la hoja de reservas del horno y la prensa."),
    ] {
        let mut p = para(s).styled("ListParagraph");
        p.props.numbering = Some(NumRef { num: steps, level: 0 });
        b.push(Block::Para(p));
    }
    b.push(Block::Para(styled(lang.pick("Membership", "Hacerse miembro"), "Heading1")));
    let mut m = para(lang.pick("Membership is ", "La cuota es "));
    add(
        &mut m,
        lang.pick("pay what you can", "la que puedas"),
        CharProps { bold: Some(true), color: Some(TextColor::Rgb(Rgb(0x15, 0x60, 0x82))), ..Default::default() },
    );
    add(
        &mut m,
        lang.pick(
            ", reviewed every season. New members shadow a regular for their first two visits, and everyone takes one cleaning shift a month. If you have never made anything before, you are exactly who we hoped would show up.",
            ", y se revisa cada temporada. Quien se une acompaña a un veterano en sus dos primeras visitas, y todos hacemos un turno de limpieza al mes. Si nunca has hecho nada, eres justo a quien esperábamos.",
        ),
        CharProps::default(),
    );
    b.push(Block::Para(m));
    let mut h = para(lang.pick(
        "Questions go to the front desk, or to the notice board by the sink. ",
        "Las dudas, en el mostrador o en el tablón junto al fregadero. ",
    ));
    add(
        &mut h,
        lang.pick("Visit the community board", "Visita el tablón de la comunidad"),
        CharProps { style: Some("Hyperlink".into()), link: Some("https://getnodalia.com/".into()), ..Default::default() },
    );
    add(&mut h, lang.pick(" for events.", " para ver los eventos."), CharProps::default());
    b.push(Block::Para(h));
    d.body = b.into_iter().map(std::sync::Arc::new).collect();
    // Footer: centred page number.
    let mut f = Paragraph::new().styled("Footer");
    f.props.align = Some(Align::Center);
    let _ = f.insert_object(
        0,
        InlineObject::Field { instr: "PAGE".into(), result: "1".into(), locked: false, dirty: false, code: false, form: String::new() },
        &CharProps::default(),
    );
    let fid = d.add_part(PartKind::Footer, vec![para_block(f)]);
    d.last_section.footers.default = Some(fid);
    let mut hd = styled(lang.pick("The Open Studio Handbook\t\tSpring edition", "El manual del taller abierto\t\tEdición de primavera"), "Header");
    hd.props.borders = Some(Borders {
        bottom: Some(Border { style: BorderStyle::Single, width: 0.5, color: Some(Rgb(0xBF, 0xBF, 0xBF)), space: 4.0 }),
        ..Default::default()
    });
    let hid = d.add_part(PartKind::Header, vec![para_block(hd)]);
    d.last_section.headers.default = Some(hid);
    d
}

/// A block-style letter.
pub fn letter() -> Document {
    letter_in(Lang::En)
}

/// [`letter`] in `lang`.
pub fn letter_in(lang: Lang) -> Document {
    let mut d = Document::new();
    let lines: Vec<Paragraph> = vec![
        styled(lang.pick("Your Name", "Tu nombre"), "Title"),
        para(lang.pick("123 Your Street · City, ST 00000 · you@example.com", "Calle Tu Dirección, 123 · Ciudad, 00000 · tu@ejemplo.com")),
        para(""),
        para(&lang.long_date()),
        para(""),
        para(lang.pick("Recipient Name", "Nombre del destinatario")),
        para(lang.pick("Title, Company", "Cargo, empresa")),
        para(lang.pick("Street Address", "Dirección")),
        para(""),
        para(lang.pick("Dear Recipient,", "Estimado destinatario:")),
        para(lang.pick(
            "Start with a sentence that says why you are writing. Keep paragraphs short and specific, and close with what you would like to happen next.",
            "Empieza con una frase que diga por qué escribes. Mantén los párrafos cortos y concretos, y termina con lo que te gustaría que pasara a continuación.",
        )),
        para(lang.pick("Thank you for your time.", "Gracias por tu tiempo.")),
        para(lang.pick("Sincerely,", "Un cordial saludo,")),
        para(""),
        para(lang.pick("Your Name", "Tu nombre")),
    ];
    d.body = lines.into_iter().map(para_block).collect();
    d
}

/// A cover letter to go with a résumé: who you are, why this job, what you bring.
pub fn cover_letter(lang: Lang) -> Document {
    let mut d = Document::new();
    let mut b = vec![
        Block::Para(styled(lang.pick("Alex Rivera", "Alejandra Rivera"), "Title")),
        Block::Para(para(
            lang.pick("Illustrator · alex@example.com · portfolio.example.com", "Ilustradora · alejandra@ejemplo.com · portafolio.ejemplo.com"),
        )),
        Block::Para(para("")),
        Block::Para(para(&lang.long_date())),
        Block::Para(para("")),
        Block::Para(para(lang.pick("Hiring team, Harbor Books", "Equipo de selección, Editorial Puerto"))),
        Block::Para(para("")),
        Block::Para(para(lang.pick("Dear hiring team,", "Estimado equipo de selección:"))),
    ];
    for text in match lang {
        Lang::En => [
            "I am writing about the art director role. Your last three covers made me want to draw again, and I would like to do that work with you.",
            "For five years I have illustrated books for readers of every age: picture books drawn in gouache, a field guide in ink, and a cover series that sold out its first printing. I like a brief with a clear problem and room to solve it.",
            "I would be glad to talk about how I would approach your spring list. Thank you for reading, and for the care your books are made with.",
        ],
        Lang::Es => [
            "Te escribo por el puesto de dirección de arte. Tus tres últimas portadas me dieron ganas de volver a dibujar, y me gustaría hacer ese trabajo contigo.",
            "Durante cinco años he ilustrado libros para lectores de todas las edades: álbumes ilustrados en gouache, una guía de campo en tinta y una serie de cubiertas que agotó su primera tirada. Me gusta un encargo con un problema claro y espacio para resolverlo.",
            "Me encantaría hablar de cómo abordaría vuestro catálogo de primavera. Gracias por leerme, y por el cuidado con el que hacéis vuestros libros.",
        ],
    } {
        b.push(Block::Para(para(text)));
    }
    b.push(Block::Para(para(lang.pick("Sincerely,", "Un cordial saludo,"))));
    b.push(Block::Para(para("")));
    b.push(Block::Para(para(lang.pick("Alex Rivera", "Alejandra Rivera"))));
    body(&mut d, b);
    d
}

/// A one-page résumé.
pub fn resume() -> Document {
    resume_in(Lang::En)
}

/// [`resume`] in `lang`.
pub fn resume_in(lang: Lang) -> Document {
    let mut d = Document::new();
    let mut b: Vec<Block> = vec![
        Block::Para(styled(lang.pick("Alex Rivera", "Alejandra Rivera"), "Title")),
        Block::Para(para(
            lang.pick("Illustrator · alex@example.com · portfolio.example.com", "Ilustradora · alejandra@ejemplo.com · portafolio.ejemplo.com"),
        )),
    ];
    let sections: [(&str, Vec<(&str, &str)>); 3] = match lang {
        Lang::En => [
            ("Experience", vec![("Lead Illustrator, Harbor Books", "2021 – present"), ("Freelance Illustrator", "2016 – 2021")]),
            ("Education", vec![("BFA Illustration, Coastal College of Art", "2016")]),
            ("Skills", vec![("Ink, gouache, digital painting, book layout, lettering", "")]),
        ],
        Lang::Es => [
            ("Experiencia", vec![("Ilustradora principal, Editorial Puerto", "2021 – actualidad"), ("Ilustradora autónoma", "2016 – 2021")]),
            ("Formación", vec![("Grado en Ilustración, Escuela de Arte de la Costa", "2016")]),
            ("Habilidades", vec![("Tinta, gouache, pintura digital, maquetación de libros, rotulación", "")]),
        ],
    };
    for (head, items) in sections {
        b.push(Block::Para(styled(head, "Heading1")));
        for (what, when) in items {
            let mut p = para(what);
            p.props.tabs = Some(vec![TabStop { pos: 468.0, align: TabAlign::Right, leader: TabLeader::None }]);
            if !when.is_empty() {
                add(&mut p, &format!("\t{when}"), italic());
            }
            b.push(Block::Para(p));
        }
    }
    body(&mut d, b);
    d
}

/// A report with a cover heading and sections.
pub fn report() -> Document {
    report_in(Lang::En)
}

/// [`report`] in `lang`.
pub fn report_in(lang: Lang) -> Document {
    let mut d = Document::new();
    d.core.title = lang.pick("Report Title", "Título del informe").into();
    let b: Vec<Block> =
        vec![
            Block::Para(styled(lang.pick("Report Title", "Título del informe"), "Title")),
            Block::Para(styled(lang.pick("Subtitle or date", "Subtítulo o fecha"), "Subtitle")),
            Block::Para(styled(lang.pick("Summary", "Resumen"), "Heading1")),
            Block::Para(para(lang.pick("Summarise the findings in two or three sentences.", "Resume los hallazgos en dos o tres frases."))),
            Block::Para(styled(lang.pick("Background", "Contexto"), "Heading1")),
            Block::Para(para(lang.pick(
                "Describe the context and the question the report answers.",
                "Describe el contexto y la pregunta a la que responde el informe.",
            ))),
            Block::Para(styled(lang.pick("Findings", "Hallazgos"), "Heading1")),
            Block::Para(styled(lang.pick("First finding", "Primer hallazgo"), "Heading2")),
            Block::Para(para(lang.pick("Explain the evidence.", "Explica las pruebas."))),
            Block::Table(text_table(
                &match lang {
                    Lang::En => [&["Measure", "Before", "After"][..], &["Visits per week", "120", "168"], &["Satisfaction", "71 %", "84 %"]],
                    Lang::Es => [&["Indicador", "Antes", "Después"][..], &["Visitas por semana", "120", "168"], &["Satisfacción", "71 %", "84 %"]],
                },
                468.0,
                1,
            )),
            Block::Para(styled(lang.pick("Recommendations", "Recomendaciones"), "Heading1")),
            Block::Para(para(lang.pick("List what should happen next.", "Enumera lo que debería ocurrir a continuación."))),
        ];
    body(&mut d, b);
    d
}

/// Minutes of a meeting: who was there, what was decided and who does what by when.
pub fn minutes(lang: Lang) -> Document {
    let mut d = Document::new();
    d.core.title = lang.pick("Meeting minutes", "Acta de reunión").into();
    let mut num = (*d.numbering).clone();
    let bullets = num.add_list(ListKind::Bullet);
    d.numbering = Arc::new(num);
    let (attendees, decisions, actions) = match lang {
        Lang::En => (
            ["Present: Mara, Theo, Inés", "Apologies: Samir"],
            ["The spring open day moves to the second Saturday.", "The kiln booking sheet becomes a shared calendar."],
            [
                ["Action", "Owner", "Due"],
                ["Send the new open-day date to members", "Mara", "Friday"],
                ["Set up the shared kiln calendar", "Theo", "Next meeting"],
                ["Price a second light table", "Inés", "Next meeting"],
            ],
        ),
        Lang::Es => (
            ["Presentes: Mara, Theo, Inés", "Excusa: Samir"],
            [
                "La jornada de puertas abiertas de primavera pasa al segundo sábado.",
                "La hoja de reservas del horno se convierte en un calendario compartido.",
            ],
            [
                ["Acción", "Responsable", "Fecha"],
                ["Enviar a los miembros la nueva fecha de la jornada", "Mara", "Viernes"],
                ["Crear el calendario compartido del horno", "Theo", "Próxima reunión"],
                ["Pedir precio de una segunda mesa de luz", "Inés", "Próxima reunión"],
            ],
        ),
    };
    let mut b = vec![
        Block::Para(styled(lang.pick("Meeting minutes", "Acta de reunión"), "Title")),
        Block::Para(styled(&format!("{} · {}", lang.pick("Open Studio committee", "Comité del taller abierto"), lang.long_date()), "Subtitle")),
        Block::Para(styled(lang.pick("Attendance", "Asistencia"), "Heading1")),
    ];
    for line in attendees {
        b.push(bullet(line, bullets));
    }
    b.push(Block::Para(styled(lang.pick("Agenda and notes", "Orden del día y notas"), "Heading1")));
    b.push(Block::Para(styled(lang.pick("1. Spring open day", "1. Jornada de primavera"), "Heading2")));
    b.push(Block::Para(para(lang.pick(
        "Notes on what was said. Keep to what someone would need if they missed the meeting.",
        "Notas de lo que se dijo. Limítate a lo que necesitaría saber quien no pudo venir.",
    ))));
    b.push(Block::Para(styled(lang.pick("Decisions", "Decisiones"), "Heading1")));
    for line in decisions {
        b.push(bullet(line, bullets));
    }
    b.push(Block::Para(styled(lang.pick("Actions", "Acciones"), "Heading1")));
    let rows: Vec<&[&str]> = actions.iter().map(|r| r.as_slice()).collect();
    b.push(Block::Table(text_table(&rows, 468.0, usize::MAX)));
    b.push(Block::Para(para("")));
    b.push(Block::Para(para(
        lang.pick("Next meeting: first Thursday of the month, 7 pm.", "Próxima reunión: el primer jueves del mes, a las 19:00."),
    )));
    body(&mut d, b);
    d
}

/// A school or college essay: title block, a thesis, three sections and a conclusion.
pub fn essay(lang: Lang) -> Document {
    let mut d = Document::new();
    d.core.title = lang.pick("Why the lighthouse stayed lit", "Por qué el faro siguió encendido").into();
    let mut b = vec![
        Block::Para(styled(lang.pick("Why the lighthouse stayed lit", "Por qué el faro siguió encendido"), "Title")),
        Block::Para(styled(lang.pick("Your name · Course · Date", "Tu nombre · Asignatura · Fecha"), "Subtitle")),
    ];
    let parts: [(&str, &str); 5] = match lang {
        Lang::En => [
            (
                "Introduction",
                "Say what the essay argues in a sentence or two. A reader should know by the end of this paragraph what you will try to show, and why it is not obvious.",
            ),
            (
                "First argument",
                "Make one point per paragraph. Begin with the claim, give the evidence, and say in a sentence why it matters to the thesis.",
            ),
            (
                "Second argument",
                "Look at the strongest objection to your claim, and answer it fairly. An essay that never meets a counter-argument has not tested itself.",
            ),
            ("Third argument", "Bring in something from outside the main text: a place, a date, a number. It anchors the argument in the world."),
            ("Conclusion", "Return to the question without repeating yourself. Say what has changed now that the reader has followed the argument."),
        ],
        Lang::Es => [
            (
                "Introducción",
                "Di en una o dos frases lo que sostiene el ensayo. Al terminar este párrafo, quien lea debería saber qué vas a intentar demostrar y por qué no es evidente.",
            ),
            (
                "Primer argumento",
                "Una idea por párrafo. Empieza por la afirmación, da la prueba y explica en una frase por qué importa para la tesis.",
            ),
            (
                "Segundo argumento",
                "Mira la objeción más fuerte a tu afirmación y respóndela con justicia. Un ensayo que nunca se enfrenta a un contraargumento no se ha puesto a prueba.",
            ),
            ("Tercer argumento", "Trae algo de fuera del texto principal: un lugar, una fecha, una cifra. Ancla el argumento en el mundo."),
            ("Conclusión", "Vuelve a la pregunta sin repetirte. Di qué ha cambiado ahora que quien lee ha seguido el argumento."),
        ],
    };
    for (head, text) in parts {
        b.push(Block::Para(styled(head, "Heading1")));
        let mut p = para(text);
        p.props.indent_first = Some(36.0);
        b.push(Block::Para(p));
    }
    body(&mut d, b);
    d
}

/// A simple invoice: from, to, a line-item table with a total, and payment terms.
pub fn invoice(lang: Lang) -> Document {
    let mut d = Document::new();
    d.core.title = lang.pick("Invoice", "Factura").into();
    let rows: [[&str; 4]; 5] = match lang {
        Lang::En => [
            ["Description", "Quantity", "Unit price", "Amount"],
            ["Cover illustration", "1", "400.00", "400.00"],
            ["Six chapter openers", "6", "80.00", "480.00"],
            ["Revisions", "2 h", "45.00", "90.00"],
            ["Total", "", "", "970.00"],
        ],
        Lang::Es => [
            ["Concepto", "Cantidad", "Precio unitario", "Importe"],
            ["Ilustración de portada", "1", "400,00", "400,00"],
            ["Seis capitulares", "6", "80,00", "480,00"],
            ["Revisiones", "2 h", "45,00", "90,00"],
            ["Total", "", "", "970,00"],
        ],
    };
    let slices: Vec<&[&str]> = rows.iter().map(|r| r.as_slice()).collect();
    let table = text_table(&slices, 468.0, 1);
    let b = vec![
        Block::Para(styled(lang.pick("Invoice", "Factura"), "Title")),
        Block::Para(styled(&format!("{} 2026-014 · {}", lang.pick("No.", "N.º"), lang.long_date()), "Subtitle")),
        Block::Para(styled(lang.pick("From", "De"), "Heading2")),
        Block::Para(para(lang.pick("Alex Rivera · 123 Your Street · City 00000", "Alejandra Rivera · Calle Tu Dirección, 123 · Ciudad 00000"))),
        Block::Para(styled(lang.pick("To", "Para"), "Heading2")),
        Block::Para(para(lang.pick("Harbor Books · 8 Quay Road · Port Town 00000", "Editorial Puerto · Calle del Muelle, 8 · Villamar 00000"))),
        Block::Para(para("")),
        Block::Table(table),
        Block::Para(para("")),
        Block::Para(styled(lang.pick("Payment", "Pago"), "Heading2")),
        Block::Para(para(lang.pick(
            "Payable within 30 days by bank transfer. Please quote the invoice number.",
            "A pagar en 30 días por transferencia bancaria. Indica el número de factura en el concepto.",
        ))),
        Block::Para(para(lang.pick("Thank you for your business.", "Gracias por confiar en mi trabajo."))),
    ];
    body(&mut d, b);
    d
}

/// A one-page newsletter: a masthead, a lead story and two short items.
pub fn newsletter(lang: Lang) -> Document {
    let mut d = Document::new();
    d.core.title = lang.pick("The Alder Street Bulletin", "El Boletín de la calle del Aliso").into();
    let mut num = (*d.numbering).clone();
    let bullets = num.add_list(ListKind::Bullet);
    d.numbering = Arc::new(num);
    let mut b = vec![
        Block::Para(styled(lang.pick("The Alder Street Bulletin", "El Boletín de la calle del Aliso"), "Title")),
        Block::Para(styled(&format!("{} · {}", lang.pick("Spring edition", "Edición de primavera"), lang.long_date()), "Subtitle")),
    ];
    let (lead_head, lead, news_head, news, close_head, close) = match lang {
        Lang::En => (
            "The press is back",
            "After three months in repair, the etching press is running again. Mara spent the winter finding a roller that fit, and the first proof off it is pinned by the door. Book your slot on the shared calendar.",
            "Also this month",
            [
                "Open day on the second Saturday, 11 to 4.",
                "New members shadow a regular for two visits.",
                "The kiln needs a steward from May: ask Theo.",
            ],
            "From the notice board",
            "Someone left a very good umbrella. It is by the sink, and it is not ours.",
        ),
        Lang::Es => (
            "La prensa ha vuelto",
            "Tras tres meses en reparación, la prensa de grabado vuelve a funcionar. Mara pasó el invierno buscando un rodillo que encajara, y la primera prueba está clavada junto a la puerta. Reserva tu turno en el calendario compartido.",
            "También este mes",
            [
                "Jornada de puertas abiertas el segundo sábado, de 11 a 16.",
                "Quien se une acompaña a un veterano dos visitas.",
                "El horno necesita responsable desde mayo: pregunta a Theo.",
            ],
            "Del tablón de anuncios",
            "Alguien dejó un paraguas muy bueno. Está junto al fregadero y no es nuestro.",
        ),
    };
    b.push(Block::Para(styled(lead_head, "Heading1")));
    b.push(Block::Para(para(lead)));
    b.push(Block::Para(styled(news_head, "Heading1")));
    for item in news {
        b.push(bullet(item, bullets));
    }
    b.push(Block::Para(styled(close_head, "Heading2")));
    b.push(Block::Para(para(close)));
    body(&mut d, b);
    d
}

/// A project proposal: the problem, the plan, a timeline table and the budget.
pub fn proposal(lang: Lang) -> Document {
    let mut d = Document::new();
    d.core.title = lang.pick("Project proposal", "Propuesta de proyecto").into();
    let mut num = (*d.numbering).clone();
    let bullets = num.add_list(ListKind::Bullet);
    d.numbering = Arc::new(num);
    let (summary, problem, goals, plan, budget_table, budget_text, next) = match lang {
        Lang::En => (
            "A neighbourhood workshop that lends tools, teaches repairs and keeps good things out of the bin.",
            "Most households own a drill they use twice a year, and most broken things are thrown away because nobody nearby knows how to mend them. Skills and tools sit idle a few streets apart.",
            [
                "Open a shared workshop two evenings a week.",
                "Teach one repair skill every month.",
                "Keep a tool library of fifty items by the end of the year.",
            ],
            [
                ["Phase", "What happens", "When"],
                ["Prepare", "Find the room, gather first tools", "Months 1–2"],
                ["Open", "Start the two weekly evenings", "Month 3"],
                ["Grow", "Monthly workshops, tool library", "Months 4–12"],
            ],
            [
                ["Item", "Cost"],
                ["Room and insurance", "2,400"],
                ["Tools and shelving", "1,800"],
                ["Materials and printing", "600"],
                ["Total", "4,800"],
            ],
            "We ask for a one-year grant to cover the first year. Everything left over at the end goes back to the tool library.",
            "Thank you for reading. We would be glad to show you the room and talk it through.",
        ),
        Lang::Es => (
            "Un taller de barrio que presta herramientas, enseña a reparar y evita que cosas buenas acaben en la basura.",
            "La mayoría de los hogares tienen un taladro que usan dos veces al año, y casi todo lo que se rompe se tira porque nadie cerca sabe arreglarlo. Las herramientas y los saberes están parados a pocas calles de distancia.",
            [
                "Abrir un taller compartido dos tardes por semana.",
                "Enseñar una habilidad de reparación cada mes.",
                "Tener una biblioteca de cincuenta herramientas a final de año.",
            ],
            [
                ["Fase", "Qué ocurre", "Cuándo"],
                ["Preparar", "Buscar el local y reunir las primeras herramientas", "Meses 1–2"],
                ["Abrir", "Empezar las dos tardes semanales", "Mes 3"],
                ["Crecer", "Talleres mensuales y biblioteca de herramientas", "Meses 4–12"],
            ],
            [
                ["Concepto", "Coste"],
                ["Local y seguro", "2.400"],
                ["Herramientas y estanterías", "1.800"],
                ["Materiales e impresión", "600"],
                ["Total", "4.800"],
            ],
            "Pedimos una subvención de un año para cubrir el primer ejercicio. Lo que sobre al final volverá a la biblioteca de herramientas.",
            "Gracias por la lectura. Nos encantaría enseñaros el local y hablarlo con calma.",
        ),
    };
    let plan_rows: Vec<&[&str]> = plan.iter().map(|r| r.as_slice()).collect();
    let budget_rows: Vec<&[&str]> = budget_table.iter().map(|r| r.as_slice()).collect();
    let mut b = vec![
        Block::Para(styled(lang.pick("The Repair Commons", "El Común de las Reparaciones"), "Title")),
        Block::Para(styled(&format!("{} · {}", lang.pick("Project proposal", "Propuesta de proyecto"), lang.long_date()), "Subtitle")),
        Block::Para(styled(lang.pick("Summary", "Resumen"), "Heading1")),
        Block::Para(para(summary)),
        Block::Para(styled(lang.pick("The problem", "El problema"), "Heading1")),
        Block::Para(para(problem)),
        Block::Para(styled(lang.pick("Goals", "Objetivos"), "Heading1")),
    ];
    for g in goals {
        b.push(bullet(g, bullets));
    }
    b.push(Block::Para(styled(lang.pick("Plan and timeline", "Plan y calendario"), "Heading1")));
    b.push(Block::Table(text_table(&plan_rows, 468.0, usize::MAX)));
    b.push(Block::Para(styled(lang.pick("Budget", "Presupuesto"), "Heading1")));
    b.push(Block::Table(text_table(&budget_rows, 468.0, 1)));
    b.push(Block::Para(para("")));
    b.push(Block::Para(para(budget_text)));
    b.push(Block::Para(styled(lang.pick("Next steps", "Próximos pasos"), "Heading1")));
    b.push(Block::Para(para(next)));
    body(&mut d, b);
    d
}

/// A one-page event flyer, centred: what, when, where and how to join.
pub fn flyer(lang: Lang) -> Document {
    let mut d = Document::new();
    d.core.title = lang.pick("Open day at the workshop", "Jornada de puertas abiertas en el taller").into();
    let (title, sub, when, when_v, wher, wher_v, what, what_v, note) = match lang {
        Lang::En => (
            "Open day at the workshop",
            "Come and make something",
            "When",
            "Saturday, 11 am to 4 pm",
            "Where",
            "The old print shop, Alder Street 14",
            "What to expect",
            "Printing, ceramics and a repair table. Tea and bread all day.",
            "Free for everyone. Bring a friend, and something that needs mending.",
        ),
        Lang::Es => (
            "Jornada de puertas abiertas en el taller",
            "Ven a hacer algo con las manos",
            "Cuándo",
            "Sábado, de 11:00 a 16:00",
            "Dónde",
            "La vieja imprenta, calle del Aliso 14",
            "Qué encontrarás",
            "Grabado, cerámica y una mesa de reparaciones. Té y pan todo el día.",
            "Entrada libre. Trae a alguien, y algo que necesite arreglo.",
        ),
    };
    let b = vec![
        Block::Para(styled(title, "Title")),
        Block::Para(styled(sub, "Subtitle")),
        Block::Para(styled(when, "Heading1")),
        Block::Para(para(when_v)),
        Block::Para(styled(wher, "Heading1")),
        Block::Para(para(wher_v)),
        Block::Para(styled(what, "Heading1")),
        Block::Para(para(what_v)),
        Block::Para(para("")),
        Block::Para(para(note)),
    ];
    body(&mut d, b);
    d
}

/// A recipe: a title block, the ingredients as a list, the method as numbered steps and a tip.
pub fn recipe(lang: Lang) -> Document {
    let mut d = Document::new();
    d.core.title = lang.pick("Lemon and olive oil cake", "Bizcocho de limón y aceite de oliva").into();
    let mut num = (*d.numbering).clone();
    let bullets = num.add_list(ListKind::Bullet);
    let steps = num.add_list(ListKind::Numbered);
    d.numbering = Arc::new(num);
    let (title, sub, ing_head, ingredients, method_head, method, tip_head, tip) = match lang {
        Lang::En => (
            "Lemon and olive oil cake",
            "Serves 8 · 15 minutes to prepare · 40 minutes to bake",
            "Ingredients",
            ["3 eggs", "150 g sugar", "120 ml olive oil", "The zest and juice of 2 lemons", "200 g flour and 2 tsp baking powder", "A pinch of salt"],
            "Method",
            [
                "Heat the oven to 180 °C and oil a 22 cm tin.",
                "Whisk the eggs and sugar until pale, then stream in the oil.",
                "Fold in the zest, the juice, then the flour, baking powder and salt.",
                "Bake for 35 to 40 minutes, until a skewer comes out clean.",
            ],
            "A good tip",
            "It keeps for three days in a tin, and is better on the second.",
        ),
        Lang::Es => (
            "Bizcocho de limón y aceite de oliva",
            "Para 8 personas · 15 minutos de preparación · 40 minutos de horno",
            "Ingredientes",
            [
                "3 huevos",
                "150 g de azúcar",
                "120 ml de aceite de oliva",
                "La ralladura y el zumo de 2 limones",
                "200 g de harina y 2 cucharaditas de levadura",
                "Una pizca de sal",
            ],
            "Preparación",
            [
                "Calienta el horno a 180 °C y engrasa un molde de 22 cm.",
                "Bate los huevos con el azúcar hasta que blanqueen y añade el aceite en hilo.",
                "Incorpora la ralladura, el zumo, y después la harina, la levadura y la sal.",
                "Hornea de 35 a 40 minutos, hasta que un palillo salga limpio.",
            ],
            "Un buen consejo",
            "Aguanta tres días en una lata, y está mejor al segundo.",
        ),
    };
    let mut b = vec![Block::Para(styled(title, "Title")), Block::Para(styled(sub, "Subtitle")), Block::Para(styled(ing_head, "Heading1"))];
    for i in ingredients {
        b.push(bullet(i, bullets));
    }
    b.push(Block::Para(styled(method_head, "Heading1")));
    for m in method {
        let mut p = para(m).styled("ListParagraph");
        p.props.numbering = Some(NumRef { num: steps, level: 0 });
        b.push(Block::Para(p));
    }
    b.push(Block::Para(styled(tip_head, "Heading2")));
    b.push(Block::Para(para(tip)));
    body(&mut d, b);
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_are_valid() {
        assert_eq!(sample_document().word_count(), 361);
        for d in [sample_document(), letter(), resume(), report()] {
            assert!(d.word_count() > 5);
            let l = letra_layout::layout(&d, &mut letra_layout::LayoutCache::new(), &Default::default());
            assert!(!l.pages.is_empty());
        }
    }

    #[test]
    fn every_template_builds_in_both_languages_and_lays_out() {
        for t in &TEMPLATES {
            if t.id == "blank" {
                assert!(build(t.id, Lang::En).is_none());
                continue;
            }
            for lang in [Lang::En, Lang::Es] {
                let d = build(t.id, lang).unwrap_or_else(|| panic!("{} has no builder", t.id));
                assert!(d.word_count() > 8, "{} {lang:?}", t.id);
                let l = letra_layout::layout(&d, &mut letra_layout::LayoutCache::new(), &Default::default());
                assert!(!l.pages.is_empty(), "{} {lang:?}", t.id);
            }
            assert_ne!(t.en, t.es, "{}: no Spanish name", t.id);
        }
    }

    #[test]
    fn the_spanish_documents_are_spanish() {
        assert_eq!(Lang::parse("es-MX"), Lang::Es);
        assert_eq!(Lang::parse("en-GB"), Lang::En);
        for id in [
            "sample",
            "letter",
            "cover-letter",
            "resume",
            "report",
            "minutes",
            "essay",
            "invoice",
            "newsletter",
            "proposal",
            "flyer",
            "recipe",
            "thesis",
        ] {
            let en = build(id, Lang::En).unwrap();
            let es = build(id, Lang::Es).unwrap();
            assert_eq!(en.body.len(), es.body.len(), "{id}: same shape");
            assert_ne!(en.plain_text(letra_doc::StoryRef::Body), es.plain_text(letra_doc::StoryRef::Body), "{id}: the Spanish copy is translated");
        }
    }
}
