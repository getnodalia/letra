# Writing plan: beyond a Word clone

> **Status:** active · P1 shipped in 0.4.0 (sources, library, import/export, manager) · **Started:** 2026-11-25 · Tracks the work that makes Letra better than Word
> at long, cited, scientific writing. Parity with Word is tracked in [`gaps.md`](gaps.md); this
> page is the other half: what Letra does that Word does not.

## Who this is for

Someone writing a final-year project (TFG/TFM), a thesis, a paper, a book or a school textbook
who has never typeset anything and still wants the result to look professionally laid out. Word
makes that person fight the tool (a source manager from 2007, equations that need a ribbon,
chapter numbering that breaks). LaTeX gives the result but not the experience. Letra should give
the LaTeX result with the Word experience, and keep `.docx` as the file so supervisors and
publishers can still open it.

## Principles

1. **Everything stays a real `.docx`.** New features write fields, styles, bookmarks and
   sources that Word also reads; nothing needs a Letra-only format. Where Word has no
   equivalent, the feature degrades to plain, correct text.
2. **Everything is a command.** Each feature is a command with a schema, so the ribbon, the
   command line, the control channel and MCP all reach it, and agents can write a thesis too.
3. **Offline first.** Nothing leaves the machine unless the person asks (a DOI lookup is a button,
   never a background request).
4. **Beautiful by default.** Templates and styles ship with good typography (measure, leading,
   hanging indents, italics where the style guide wants them) so the first page already looks
   right. Same Nodalia Design look as the rest of the suite: no gradients, no glows.
5. **Spanish and English from day one.** Every string is translated; every template exists in
   both languages.

## Baseline (0.3.0)

Already there: Word's source manager (per document), 12 citation styles from a fixed table,
`CITATION`/`BIBLIOGRAPHY` fields, Zotero over its desktop protocol, footnotes and endnotes,
captions with `SEQ` fields and cross-references, tables of contents and figures, an equation
editor with Unicode-math and LaTeX input and equation numbers, and 13 templates.

What is missing, and why it hurts a writer:

| Gap | Effect |
| --- | --- |
| Sources live inside one document | The same 40 references are retyped for every chapter or paper |
| No BibTeX, RIS or CSL-JSON import | Anyone with a Zotero, Mendeley, Google Scholar or journal export starts from zero |
| Entries are plain text, no italics, DOI ignored | Journal and book titles are not italic and DOIs are lost, so a bibliography is wrong in APA, Vancouver and IEEE |
| No Vancouver, AMA, Nature, ACS | Health, medicine and science students have no style for their field |
| No theorem/definition/proof blocks | Maths, physics and engineering text is typed by hand and renumbered by hand |
| Equation numbers do not follow chapters, no list of equations | `(2.3)` style numbering and a list of equations need manual work |
| No thesis, paper, book or textbook templates | The first hour goes into styles, page numbers, front matter and chapter openers |

## Increments

Priority order; each ships as its own pull request with tests, screenshots and release notes.

### P1. Sources and citations (0.4.0)

- **Import** BibTeX (`.bib`), RIS, CSL-JSON and Word's `Sources.xml`, from a file or pasted text,
  with accents, braces, months, page ranges and name forms handled; duplicates are matched by DOI
  or by title, author and year.
- **Export** the document's sources as BibTeX, RIS or CSL-JSON, so they move to Zotero or LaTeX.
- **My library**: a personal list of sources kept in the settings folder, shared by every
  document. A document uses a copy of the sources it cites (as Word does), so files stay
  self-contained.
- **Source Manager** redesign: search, two lists (this document, my library), a live preview of
  the formatted entry and the in-text citation in the current style, a style picker, paste
  to import, and one-click add to the other list.
- **Styles**: Vancouver, ACM and Harvard (Cite Them Right) are in (15 styles); DOI, issue, edition and
  editor in every template that uses them; italic titles where the style guide asks for them. AMA,
  Nature and ACS need superscript numbering and come later.
- Later: DOI and ISBN lookup (explicit button), CSL style file import, citation ranges
  (`[1–3]`), superscript numbering, notes-and-bibliography styles, cite-while-you-write search box.

### P2. Scientific writing (0.5.0)

- Numbered equations that follow the chapter (`(2.3)`), with a list of equations.
- Theorem, lemma, proposition, corollary, definition, example, remark and proof blocks: styled,
  numbered by chapter or by document, cross-referenceable, listed on demand. **Done**: References ›
  Theorems › Insert Theorem (`references.theorem`). A block is a Caption-based paragraph, so Insert
  Cross-reference lists it under its label and a table of figures of that label is a list of
  theorems.
- Figure and table captions with chapter-aware numbers, subfigures.
- Units and symbols: a quick list, siunitx-style spacing between number and unit.
- Abbreviation and symbol lists generated from the text.

### P3. Document types and templates (0.5.0)

- **Thesis / TFG**: cover, title page, declaration, abstract in two languages, acknowledgements,
  contents, lists of figures, tables and abbreviations, numbered chapters, bibliography,
  appendices, with roman front-matter numbering and arabic body numbering.
- **Scientific paper**: single and two-column layouts, abstract, keywords, numbered sections,
  references, an IEEE-like and a general journal variant.
- **Book**: half-title, title page, copyright page, dedication, parts and chapters opening
  on a right-hand page, running heads, index, colophon; trim sizes and mirrored margins.
- **School textbook**: units with objectives, margin notes, worked examples, exercises,
  summary boxes, a glossary, answer key.

### P4. Typography and export quality

- Hyphenation, justification and ligature checks; widow and orphan control on by default in the
  long-document templates; optical margin hints.
- PDF export with outline, tagged structure, embedded fonts and correct page labels for front
  matter.

## How each increment is checked

- Unit tests next to the code, a failing test before each fix, command-level tests that go
  through the same entry points the ribbon uses.
- A DOCX save and reopen test for everything written to the file.
- Screenshots from the running app, light and dark, attached to the pull request.
- Spanish strings added in the same pull request (the interface audit fails otherwise).
