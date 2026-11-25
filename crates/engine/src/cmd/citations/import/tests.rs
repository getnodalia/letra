use super::*;

const BIB: &str = r#"
% a comment line
@comment{ignored @article{nope, title={Nope}} }
@string{ieee = "IEEE Trans. Pattern Anal."}
@Article{garcia2020,
  author  = {Garc{\'\i}a-L{\'o}pez, Mar{\'\i}a and Jos{\'e} Mart{\'i}nez and {\"U}mit \c{S}ahin},
  title   = {Aprendizaje {profundo} para \emph{im\'agenes} m{\'e}dicas -- una revisi{\'o}n},
  journal = ieee,
  year    = 2020,
  month   = mar,
  volume  = {42},
  number  = "3",
  pages   = {101--115},
  doi     = {https://doi.org/10.1109/TPAMI.2020.1},
}
@book{beethoven,
  author = {Ludwig van Beethoven and Jones, Jr., Ann},
  title  = {Symphonies} # { and } # {Sonatas},
  publisher = {Harbor Books},
  address = {Portland},
  edition = {2nd},
  year = {1824},
}
@techreport{who21,
  author = {{World Health Organization}},
  title = {World report on hearing},
  institution = {WHO Press},
  year = {2021},
  url = {https://www.who.int/publications},
}
@inproceedings{conf,
  author = {Rivera, Alex},
  title = {Kilns at scale},
  booktitle = {Proceedings of the Studio Conference},
  year = {2019},
  pages = {1--9},
}
"#;

#[test]
fn bibtex_entries_become_sources() {
    let v = parse(BIB, Format::BibTeX);
    assert_eq!(v.len(), 4, "{v:#?}");
    let a = &v[0];
    assert_eq!(a.tag, "garcia2020");
    assert_eq!(a.kind, "article");
    assert_eq!(a.author, "García-López, María; Martínez, José; Sahin, Ümit".replace("Sahin, Ümit", "Şahin, Ümit"), "{a:?}");
    assert_eq!(a.title, "Aprendizaje profundo para imágenes médicas – una revisión");
    assert_eq!(a.journal, "IEEE Trans. Pattern Anal.", "a macro is replaced by its text");
    assert_eq!((a.year.as_str(), a.month.as_str(), a.volume.as_str(), a.issue.as_str()), ("2020", "March", "42", "3"));
    assert_eq!(a.pages, "101–115");
    assert_eq!(a.doi, "10.1109/TPAMI.2020.1", "a doi.org link is reduced to the DOI");
}

#[test]
fn bibtex_names_follow_the_von_and_junior_rules() {
    let v = parse(BIB, Format::BibTeX);
    let b = &v[1];
    assert_eq!(b.author, "van Beethoven, Ludwig; Jones, Ann", "{b:?}");
    assert_eq!(b.title, "Symphonies and Sonatas", "# joins the parts");
    assert_eq!((b.city.as_str(), b.publisher.as_str(), b.edition.as_str(), b.kind.as_str()), ("Portland", "Harbor Books", "2nd", "book"));
}

#[test]
fn a_doubly_braced_author_is_a_corporate_author() {
    let v = parse(BIB, Format::BibTeX);
    let r = &v[2];
    assert_eq!(r.corporate, "World Health Organization");
    assert!(r.author.is_empty());
    assert_eq!((r.kind.as_str(), r.publisher.as_str()), ("report", "WHO Press"));
    assert_eq!(v[3].kind, "conference");
    assert_eq!(v[3].journal, "Proceedings of the Studio Conference");
}

#[test]
fn unlatex_reads_accents_and_symbols() {
    assert_eq!(unlatex(r#"Sch{\"o}n \& {\v{C}}apek \ss\ {\o}"#), "Schön & Čapek ß ø");
    assert_eq!(unlatex(r"\'{e}\`a\^o\~n \c c"), "éàôñ ç");
    assert_eq!(unlatex("a---b a--b ``q''"), "a—b a–b “q”");
    assert_eq!(unlatex(r"\textit{Hello}~world"), "Hello world");
}

#[test]
fn hostile_bibtex_never_panics() {
    for t in [
        "@article{",
        "@article{k, title = {unclosed",
        "@article{k, title = \"unclosed",
        "@string{",
        "@comment{",
        "@@@@",
        "@article{k,,,,= = =}",
        "@book(k, title={x}",
        &"{".repeat(100_000),
        &format!("@article{{k, title = {}x{}}}", "{".repeat(50_000), "}".repeat(50_000)),
        "@article{k, author = {a and and and}, year = {}}",
        "@article{k, title = {\\",
    ] {
        let _ = parse(t, Format::BibTeX);
    }
    let many: String = (0..30_000).map(|i| format!("@misc{{k{i}, title={{T{i}}}}}\n")).collect();
    assert_eq!(parse(&many, Format::BibTeX).len(), MAX_RECORDS, "the import is capped");
}

const RIS: &str = "TY  - JOUR\nAU  - Rivera, Alex\nAU  - Chen, Mei\nTI  - Shared spaces and a very long\n   wrapped title\nT2  - Journal of Studios\nPY  - 2021/03/15/\nVL  - 7\nIS  - 2\nSP  - 11\nEP  - 29\nDO  - 10.1000/182\nER  - \n\nTY  - BOOK\nA1  - Okafor, Ngozi\nT1  - Clay\nPB  - Harbor Books\nCY  - Portland\nY1  - 2018\nER  - \n";

#[test]
fn ris_records_become_sources() {
    let v = parse(RIS, Format::Ris);
    assert_eq!(v.len(), 2, "{v:#?}");
    let a = &v[0];
    assert_eq!(a.kind, "article");
    assert_eq!(a.author, "Rivera, Alex; Chen, Mei");
    assert_eq!(a.title, "Shared spaces and a very long wrapped title");
    assert_eq!((a.journal.as_str(), a.year.as_str(), a.month.as_str(), a.day.as_str()), ("Journal of Studios", "2021", "March", "15"));
    assert_eq!((a.volume.as_str(), a.issue.as_str(), a.pages.as_str(), a.doi.as_str()), ("7", "2", "11–29", "10.1000/182"));
    assert_eq!((v[1].kind.as_str(), v[1].publisher.as_str(), v[1].year.as_str()), ("book", "Harbor Books", "2018"));
}

#[test]
fn csl_json_items_become_sources() {
    let t = r#"[{"id":"rivera21","type":"article-journal","title":"Shared spaces","author":[{"family":"Rivera","given":"Alex"},{"family":"Chen","given":"Mei"}],
      "container-title":"Journal of Studios","issued":{"date-parts":[[2021,3,15]]},"volume":"7","issue":"2","page":"11-29","DOI":"10.1000/182"},
     {"id":"who","type":"report","title":"World report","author":[{"literal":"World Health Organization"}],"issued":{"raw":"Published 2021"}},
     "not an item", 12]"#;
    let v = parse(t, Format::CslJson);
    assert_eq!(v.len(), 2, "{v:#?}");
    assert_eq!((v[0].tag.as_str(), v[0].kind.as_str(), v[0].author.as_str()), ("rivera21", "article", "Rivera, Alex; Chen, Mei"));
    assert_eq!((v[0].year.as_str(), v[0].month.as_str(), v[0].day.as_str(), v[0].pages.as_str()), ("2021", "March", "15", "11-29"));
    assert_eq!((v[1].corporate.as_str(), v[1].year.as_str()), ("World Health Organization", "2021"));
    assert!(parse("{not json", Format::CslJson).is_empty());
}

#[test]
fn formats_are_detected() {
    assert_eq!(detect(BIB), Some(Format::BibTeX));
    assert_eq!(detect(RIS), Some(Format::Ris));
    assert_eq!(detect("  [{\"id\":\"a\"}]"), Some(Format::CslJson));
    assert_eq!(detect("Just a sentence with an @ sign."), None);
    assert_eq!(detect(""), None);
    assert_eq!(Format::of_path("library.BIB"), Some(Format::BibTeX));
    assert_eq!(Format::of_path("notes.txt"), None);
}

#[test]
fn every_format_round_trips_what_it_can_hold() {
    let src = Source {
        tag: "rivera21".into(),
        kind: "article".into(),
        author: "Rivera, Alex; Chen, Mei".into(),
        title: "Shared spaces & clay".into(),
        year: "2021".into(),
        journal: "Journal of Studios".into(),
        volume: "7".into(),
        issue: "2".into(),
        pages: "11–29".into(),
        doi: "10.1000/182".into(),
        ..Default::default()
    };
    let corp = Source {
        tag: "who".into(),
        kind: "report".into(),
        corporate: "World Health Organization".into(),
        title: "Hearing".into(),
        year: "2021".into(),
        publisher: "WHO Press".into(),
        ..Default::default()
    };
    for f in [Format::BibTeX, Format::Ris, Format::CslJson] {
        let text = export(&[src.clone(), corp.clone()], f);
        let back = parse(&text, f);
        assert_eq!(back.len(), 2, "{f:?}\n{text}");
        for (a, b) in [(&src, &back[0]), (&corp, &back[1])] {
            // RIS has no corporate author: the organisation comes back as the one author.
            let who = |x: &Source| if x.corporate.is_empty() { x.author.clone() } else { x.corporate.clone() };
            assert_eq!((who(a), &a.title, &a.year), (who(b), &b.title, &b.year), "{f:?}\n{text}");
            if f != Format::Ris {
                assert_eq!((&a.author, &a.corporate), (&b.author, &b.corporate), "{f:?}");
            }
            assert_eq!((&a.journal, &a.volume, &a.issue, &a.doi, &a.publisher), (&b.journal, &b.volume, &b.issue, &b.doi, &b.publisher), "{f:?}");
        }
        assert_eq!(back[0].pages.replace('-', "–"), "11–29", "{f:?}");
    }
}

#[test]
fn bare_dois_come_from_any_form() {
    for t in ["10.1000/182", "doi:10.1000/182", "https://doi.org/10.1000/182", "http://dx.doi.org/10.1000/182", " DOI.org/10.1000/182 "] {
        assert_eq!(bare_doi(t), "10.1000/182", "{t}");
    }
    assert_eq!(bare_doi(""), "");
}
