//! My Library: the person's own list of sources, kept in the settings folder and shared by every
//! document. A document keeps a copy of the sources it cites (as Word does), so a file stays
//! self-contained; the library is where the same forty references wait between chapters and papers.
//!
//! Also the one place that decides when two records are the same work, so importing the same
//! export twice, or adding a source the document already has, never makes a duplicate.

use super::Source;
use super::render::{authors, bare_doi, corporate};
use super::sources::auto_tag_base;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Most sources the library keeps.
pub const MAX_ITEMS: usize = 20_000;

/// The largest library file read.
#[cfg(not(target_arch = "wasm32"))]
const MAX_FILE: u64 = 64 << 20;

/// The sources a person keeps across documents.
#[derive(Debug, Default)]
pub struct Library {
    items: Vec<Source>,
    path: Option<PathBuf>,
}

/// What [`merge`] did with a list of incoming sources.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Merged {
    /// Tags of the sources added, in order.
    pub added: Vec<String>,
    /// How many were already there (the same work) and were left alone.
    pub duplicates: usize,
    /// How many did not fit (the list is full).
    pub dropped: usize,
}

impl Library {
    pub fn items(&self) -> &[Source] {
        &self.items
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Keep the library in the file at `path` (the host passes it at start-up), reading what it
    /// holds. A missing or damaged file starts an empty library; the damaged one is left on disk
    /// until the next change replaces it.
    pub fn set_path(&mut self, path: Option<PathBuf>) {
        self.items = path.as_deref().map(read_file).unwrap_or_default();
        self.path = path;
    }

    /// Add `src` unless the same work is already here. Returns the tag of the source that is now
    /// in the library, and whether it is new.
    pub fn add(&mut self, src: Source) -> Option<(String, bool)> {
        if let Some(have) = self.items.iter().find(|x| same_work(x, &src)) {
            return Some((have.tag.clone(), false));
        }
        let merged = merge(&mut self.items, vec![src], MAX_ITEMS);
        merged.added.into_iter().next().map(|t| (t, true))
    }

    /// Add every source of `incoming` that is not here yet.
    pub fn add_all(&mut self, incoming: Vec<Source>) -> Merged {
        merge(&mut self.items, incoming, MAX_ITEMS)
    }

    /// Replace the source tagged `tag`, keeping its place. Returns false when there is none.
    pub fn replace(&mut self, tag: &str, src: Source) -> bool {
        match self.items.iter_mut().find(|x| x.tag == tag) {
            Some(x) => {
                *x = Source { tag: tag.to_string(), ..src };
                true
            }
            None => false,
        }
    }

    pub fn remove(&mut self, tag: &str) -> bool {
        let before = self.items.len();
        self.items.retain(|x| x.tag != tag);
        self.items.len() != before
    }

    pub fn get(&self, tag: &str) -> Option<&Source> {
        self.items.iter().find(|x| x.tag == tag)
    }

    /// Write the library to its file. Nothing happens when the host gave none (the web).
    pub fn save(&self) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = &self.path {
            let text = serde_json::to_string_pretty(&serde_json::json!({"format": "letra-library", "version": 1, "sources": self.items}))
                .map_err(|e| e.to_string())?;
            crate::io::write_own_file(path, text.as_bytes())?;
        }
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_file(path: &Path) -> Vec<Source> {
    let Ok(bytes) = crate::io::read_own_file(path, MAX_FILE) else { return Vec::new() };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) else { return Vec::new() };
    let list = v.get("sources").cloned().unwrap_or(v);
    let mut items: Vec<Source> = serde_json::from_value(list).unwrap_or_default();
    items.truncate(MAX_ITEMS);
    items
}

#[cfg(target_arch = "wasm32")]
fn read_file(_path: &Path) -> Vec<Source> {
    Vec::new()
}

/// `s` with letters and digits only, lower case: titles and surnames compare without punctuation,
/// case or spacing.
fn plain(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// The first author's surname (or the corporate author), plain.
fn first_author(src: &Source) -> String {
    match corporate(src) {
        Some(c) => plain(c),
        None => authors(&src.author).first().map(|a| plain(&a.0)).unwrap_or_default(),
    }
}

/// Whether `a` and `b` are the same work: the same DOI, or the same title, first author and year
/// (a field one of them leaves empty does not tell them apart).
pub fn same_work(a: &Source, b: &Source) -> bool {
    let (da, db) = (bare_doi(&a.doi).to_ascii_lowercase(), bare_doi(&b.doi).to_ascii_lowercase());
    if !da.is_empty() && !db.is_empty() {
        return da == db;
    }
    let (ta, tb) = (plain(&a.title), plain(&b.title));
    if ta.is_empty() || ta != tb {
        return false;
    }
    let agree = |x: String, y: String| x.is_empty() || y.is_empty() || x == y;
    agree(first_author(a), first_author(b)) && agree(plain(&a.year), plain(&b.year))
}

/// Add each of `incoming` to `list` unless the same work is already there, giving it a tag that
/// no source of the list has (its own when free, otherwise the author-and-year stem with `1`, `2`…
/// added). `cap` bounds the list.
pub fn merge(list: &mut Vec<Source>, incoming: Vec<Source>, cap: usize) -> Merged {
    let mut out = Merged::default();
    let mut tags: HashSet<String> = list.iter().map(|x| x.tag.clone()).collect();
    for mut src in incoming {
        if list.iter().any(|x| same_work(x, &src)) {
            out.duplicates += 1;
            continue;
        }
        if list.len() >= cap {
            out.dropped += 1;
            continue;
        }
        if src.tag.trim().is_empty() || tags.contains(&src.tag) {
            let base = if src.tag.trim().is_empty() { auto_tag_base(&src) } else { src.tag.clone() };
            src.tag = if tags.contains(&base) { (1u32..).map(|n| format!("{base}{n}")).find(|t| !tags.contains(t)).unwrap_or(base) } else { base };
        }
        tags.insert(src.tag.clone());
        out.added.push(src.tag.clone());
        list.push(src);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src(tag: &str, author: &str, title: &str, year: &str, doi: &str) -> Source {
        Source {
            tag: tag.into(),
            kind: "article".into(),
            author: author.into(),
            title: title.into(),
            year: year.into(),
            doi: doi.into(),
            ..Default::default()
        }
    }

    #[test]
    fn same_doi_or_same_title_author_and_year_is_one_work() {
        let a = src("A", "Smith, Jane", "Deep Learning", "2020", "10.1/x");
        assert!(same_work(&a, &src("B", "Other, Person", "Another title", "1999", "https://doi.org/10.1/X")));
        let b = src("C", "SMITH, Jane", "Deep  learning!", "2020", "");
        assert!(same_work(&src("D", "Smith, J.", "deep learning", "2020", ""), &b));
        assert!(!same_work(&a, &src("E", "Smith, Jane", "Deep Learning", "2020", "10.2/y")), "different DOIs are different works");
        assert!(!same_work(&src("F", "Smith, Jane", "Deep Learning", "2020", ""), &src("G", "Smith, Jane", "Deep Learning", "2021", "")));
        assert!(!same_work(&src("H", "", "", "2020", ""), &src("I", "", "", "2020", "")), "no title, no identity");
    }

    #[test]
    fn merge_skips_duplicates_and_keeps_tags_unique() {
        let mut list = vec![src("Smi20", "Smith, Jane", "One", "2020", "")];
        let m = merge(
            &mut list,
            vec![
                src("Smi20", "Smith, Jane", "One", "2020", ""),
                src("Smi20", "Smith, John", "Two", "2020", ""),
                src("", "Smith, Ann", "Three", "2020", ""),
            ],
            100,
        );
        assert_eq!(m.duplicates, 1);
        assert_eq!(m.added, vec!["Smi201".to_string(), "Smi202".to_string()]);
        let tags: HashSet<_> = list.iter().map(|x| x.tag.as_str()).collect();
        assert_eq!(tags.len(), list.len());
    }

    #[test]
    fn merge_respects_the_cap() {
        let mut list = Vec::new();
        let m = merge(&mut list, (0..5).map(|i| src("", "A, B", &format!("T{i}"), "2000", "")).collect(), 3);
        assert_eq!((list.len(), m.dropped), (3, 2));
    }

    #[test]
    fn the_library_round_trips_through_its_file() {
        let dir = std::env::temp_dir().join(format!("letra-lib-{}", std::process::id()));
        let path = dir.join("library.json");
        let mut lib = Library::default();
        lib.set_path(Some(path.clone()));
        assert!(lib.items().is_empty());
        assert_eq!(lib.add(src("", "Knuth, Donald", "The Art of Computer Programming", "1968", "")), Some(("Knu68".into(), true)));
        assert_eq!(lib.add(src("", "Knuth, Donald", "the art of computer programming", "1968", "")), Some(("Knu68".into(), false)));
        lib.save().unwrap();
        let mut again = Library::default();
        again.set_path(Some(path.clone()));
        assert_eq!(again.items(), lib.items());
        assert!(again.remove("Knu68") && !again.remove("Knu68"));
        std::fs::write(&path, b"{ not json").unwrap();
        again.set_path(Some(path));
        assert!(again.items().is_empty(), "a damaged file starts an empty library");
        let _ = std::fs::remove_dir_all(dir);
    }
}
