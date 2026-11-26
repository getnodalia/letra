//! The AI assistant: Letra's side of the Office suite's shared chat (`nodalia-assistant`, from
//! nodalia-design `packages/assistant`).
//!
//! The component draws the button, the panel and the Preferences section the same way in Celdas,
//! Lámina and here, keeps the person's provider, model and key, and runs the loop with the model.
//! This file is everything the component cannot know: what a Letra document is, which tools the
//! model gets to work on it, and how a turn is undone. The tools sit on the same command registry
//! that scripts and MCP use, so the assistant can do what a person can do from the ribbon and
//! nothing else: it cannot open, save, close or print files.
//!
//! The person's own messages are the only instructions. The document is data, and every change
//! waits for Apply unless they chose to let the assistant work as it goes (Preferences ›
//! Agents). A whole turn undoes in one click.

use std::sync::Arc;

use egui::Ui;
use nodalia_assistant::{Assistant, Host, Lang, ToolSpec};
use nodalia_theme::office::Look;
use serde_json::{Value, json};

use crate::WordApp;

/// Characters of a block's text the snapshot keeps once the document is long.
const LONG_BLOCK: usize = 200;
/// Characters of snapshot JSON before the blocks are shortened.
const SNAPSHOT_BUDGET: usize = 40_000;
/// Most commands `list_commands` returns in one answer.
const MAX_COMMANDS: usize = 60;
/// Most paragraphs one `read_blocks` answer carries.
const MAX_BLOCKS: usize = 200;
/// Longest text the approval card shows for one argument.
const PREVIEW: usize = 80;

/// Command families the assistant never runs: files and windows are the person's to open, save,
/// close and print, and `ui.*` drives the window rather than the document.
const FORBIDDEN: [&str; 6] = ["file.", "ui.", "view.switchWindows", "print", "app.", "window."];

/// The look the shared component draws with: Letra's palette and its semibold family.
pub fn look(dark: bool) -> Look {
    let mode = if dark { nodalia_theme::Mode::Dark } else { nodalia_theme::Mode::Light };
    Look::new(crate::theme::palette(mode)).with_bold(crate::theme::semibold(1.0).family)
}

/// The language the component speaks: Spanish when the interface is Spanish, English otherwise.
pub fn lang() -> Lang {
    Lang::from_code(crate::i18n::current().code())
}

impl WordApp {
    /// The assistant, made the first time it is asked for. It reads the saved choices and starts
    /// asking the keychain, which nothing should do before a window is on screen.
    pub(crate) fn assistant_mut(&mut self) -> Option<&mut Assistant> {
        if self.assistant.is_none() {
            let ctx = self.ctx.clone()?;
            self.assistant = Some(Assistant::new(Arc::new(move || ctx.request_repaint())));
        }
        self.assistant.as_mut()
    }

    /// Whether the title bar offers the assistant: the person did not turn it off, and a
    /// document is open for it to work on.
    pub(crate) fn assistant_offered(&mut self) -> bool {
        self.has_document() && self.assistant_mut().is_some_and(|a| a.settings.enabled)
    }

    /// Runs `f` with the assistant and a host over this window. The assistant lives in the app
    /// and the host borrows the app, so it is taken out for the duration.
    pub(crate) fn with_assistant<R>(&mut self, f: impl FnOnce(&mut Assistant, &mut LetraHost<'_>) -> R) -> Option<R> {
        self.assistant_mut()?;
        let mut assistant = self.assistant.take()?;
        assistant.lang = lang();
        let result = f(&mut assistant, &mut LetraHost { app: self, lang: lang() });
        self.assistant = Some(assistant);
        Some(result)
    }

    /// Runs the model's pending tool calls and moves its answer along. Called once a frame.
    pub(crate) fn poll_assistant(&mut self) {
        if self.assistant.is_none() {
            return;
        }
        if self.with_assistant(|a, host| a.poll(host)).unwrap_or(false)
            && let Some(ctx) = self.ctx.clone()
        {
            ctx.request_repaint();
        }
    }
}

/// The title bar's button: the chosen provider's mark, which opens and closes the chat.
pub fn button(app: &mut WordApp, ui: &mut Ui) {
    let look = look(crate::theme::Tokens::get(ui.ctx()).dark);
    let Some(a) = app.assistant_mut() else { return };
    a.lang = lang();
    let label = a.lang.t("Assistant");
    let r = nodalia_assistant::ui::button(ui, &look, a);
    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
}

/// The chat panel, on the right edge, drawn before the task panes so it keeps the edge.
pub fn panel(app: &mut WordApp, ui: &mut Ui) {
    if !app.assistant.as_ref().is_some_and(|a| a.open) {
        return;
    }
    let t = crate::theme::Tokens::get(ui.ctx());
    let look = look(t.dark);
    let width = nodalia_assistant::ui::PANEL_WIDTH;
    let mut open_preferences = false;
    egui::Panel::right("assistant_panel")
        .exact_size(width + crate::theme::ISLAND_GAP)
        .show_separator_line(false)
        .frame(egui::Frame::NONE.fill(t.canvas).inner_margin(egui::Margin {
            left: 0,
            right: crate::theme::ISLAND_GAP as i8,
            top: 0,
            bottom: crate::theme::ISLAND_GAP as i8,
        }))
        .show(ui, |ui| {
            if let Some(outcome) = app.with_assistant(|a, host| nodalia_assistant::ui::panel(ui, &look, a, host)) {
                open_preferences = outcome.open_preferences;
            }
        });
    if open_preferences {
        let _ = app.run("file.options", json!({"pane": "agents"}));
    }
}

/// The Preferences › Agents section.
pub fn preferences(app: &mut WordApp, ui: &mut Ui) {
    let look = look(crate::theme::Tokens::get(ui.ctx()).dark);
    if let Some(a) = app.assistant_mut() {
        nodalia_assistant::ui::preferences(ui, &look, a);
    }
}

/// What the assistant works through: this window's document, by the commands the registry has.
pub(crate) struct LetraHost<'a> {
    app: &'a mut WordApp,
    lang: Lang,
}

fn tool(name: &str, description: &str, schema: Value, read_only: bool) -> ToolSpec {
    ToolSpec { name: name.into(), description: description.into(), schema, read_only }
}

fn object(properties: Value, required: &[&str]) -> Value {
    json!({"type": "object", "properties": properties, "required": required})
}

fn text(description: &str) -> Value {
    json!({"type": "string", "description": description})
}

fn forbidden(command: &str) -> bool {
    FORBIDDEN.iter().any(|f| command.starts_with(f))
}

/// The first `n` characters of `s`, with an ellipsis when it was longer.
fn clip(s: &str, n: usize) -> String {
    let mut out: String = s.chars().take(n).collect();
    if s.chars().nth(n).is_some() {
        out.push('…');
    }
    out
}

fn str_arg<'v>(input: &'v Value, key: &str) -> Result<&'v str, String> {
    input.get(key).and_then(Value::as_str).ok_or_else(|| format!("`{key}` is missing; it must be a string"))
}

impl LetraHost<'_> {
    fn run(&mut self, command: &str, params: Value) -> Result<Value, String> {
        self.app.execute(command, params)
    }

    /// Every edit starts a step of its own: typing the person did just before must not swallow it.
    fn edit(&mut self, command: &str, params: Value) -> Result<Value, String> {
        self.app.session.close_typing();
        self.run(command, params)
    }

    fn list_commands(&mut self, query: &str) -> Value {
        let q = query.to_lowercase();
        let registry = self.app.session.registry.clone();
        let found: Vec<Value> = registry
            .all()
            .iter()
            .filter(|c| !forbidden(c.id))
            .filter(|c| q.is_empty() || [c.id, c.label, c.location, c.params].iter().any(|f| f.to_lowercase().contains(&q)))
            .take(MAX_COMMANDS)
            .map(|c| json!({"id": c.id, "label": c.label, "where": c.location, "params": c.params}))
            .collect();
        json!({"commands": found, "limit": MAX_COMMANDS})
    }

    fn read_blocks(&mut self, input: &Value) -> Result<Value, String> {
        let from = input.get("from").and_then(Value::as_u64).unwrap_or(0) as usize;
        let count = input.get("count").and_then(Value::as_u64).map_or(50, |n| n as usize).clamp(1, MAX_BLOCKS);
        let inspected = self.run("document.inspect", json!({"text": true}))?;
        let blocks = inspected.get("blocks").and_then(Value::as_array).cloned().unwrap_or_default();
        let total = blocks.len();
        let slice: Vec<Value> = blocks.iter().skip(from).take(count).map(slim).collect();
        Ok(json!({"from": from, "total": total, "blocks": slice}))
    }
}

/// A block as the model reads it: its place, kind and text, without the run-by-run formatting.
fn slim(block: &Value) -> Value {
    let mut out = serde_json::Map::new();
    for key in ["index", "type", "style", "text", "list", "rows", "cols", "cells", "sectionBreak"] {
        if let Some(v) = block.get(key) {
            out.insert(key.into(), v.clone());
        }
    }
    Value::Object(out)
}

/// Shortens every block's text, tables included, to at most `n` characters.
fn shorten(block: &mut Value, n: usize) {
    if let Some(t) = block.get("text").and_then(Value::as_str) {
        let t = clip(t, n);
        block["text"] = json!(t);
    }
    if let Some(rows) = block.get_mut("cells").and_then(Value::as_array_mut) {
        for row in rows {
            if let Some(cells) = row.as_array_mut() {
                for c in cells {
                    if let Some(t) = c.as_str() {
                        *c = json!(clip(t, 60));
                    }
                }
            }
        }
    }
}

/// The snapshot the model starts a turn with: the document's blocks (shortened when it is long,
/// so the model reads the rest with `read_blocks`), the selection and the document's state.
fn snapshot(inspected: &Value) -> Value {
    let blocks: Vec<Value> = inspected.get("blocks").and_then(Value::as_array).map(|b| b.iter().map(slim).collect()).unwrap_or_default();
    let total = blocks.len();
    let mut shown = blocks;
    let mut note = Value::Null;
    if serde_json::to_string(&shown).map_or(0, |s| s.chars().count()) > SNAPSHOT_BUDGET {
        for b in &mut shown {
            shorten(b, LONG_BLOCK);
        }
        note = json!("Long document: each block's text is shortened. Read a range in full with read_blocks.");
    }
    // Still too long: the first and last parts and every heading say what the document is.
    if serde_json::to_string(&shown).map_or(0, |s| s.chars().count()) > SNAPSHOT_BUDGET {
        let keep = |i: usize, b: &Value| {
            i < 60 || i + 20 >= total || b.get("style").and_then(Value::as_str).is_some_and(|s| s.starts_with("Heading") || s == "Title")
        };
        shown = shown.into_iter().enumerate().filter(|(i, b)| keep(*i, b)).map(|(_, b)| b).collect();
        note = json!("Very long document: only the start, the end and the headings are listed. Read the rest with read_blocks.");
    }
    let mut out = json!({
        "blocks": shown,
        "blockCount": total,
        "selection": inspected.get("selection").cloned().unwrap_or(Value::Null),
        "pages": inspected.get("pages"),
        "words": inspected.get("words"),
        "trackChanges": inspected.get("trackChanges"),
        "comments": inspected.get("comments"),
        "canUndo": inspected.get("canUndo"),
    });
    if !note.is_null() {
        out["note"] = note;
    }
    out
}

const GUIDE: &str = "Letra is a word processor. The body of a document is a list of blocks, each a paragraph (with a style such as Normal, Title or Heading 1) or a table; headers, footers and footnotes are separate parts.\n\
\n\
The snapshot lists the blocks with their index and text, plus the selection. Where the person is working is the selection: with nothing selected it is the caret. type_text puts text at the caret and replaces a selection.\n\
\n\
To change something that already exists, select it first with select_text (exact text, and which occurrence), then type_text to replace it, or run_command for formatting such as format.bold. To add paragraphs use type_text with paragraphs=true, one line per paragraph. Use insert_table for tables, filled in one call.\n\
\n\
Letra has hundreds of commands, one for each button on the ribbon. Look one up with list_commands (search by what it does, like \"bold\", \"margin\", \"footnote\") and read its params before running it with run_command. Prefer small edits to replacing the whole document with set_document_text, which loses all formatting.\n\
\n\
If the document tracks changes, your edits are recorded as the person's own, so they can review them. You cannot open, save, close or print files.";

impl Host for LetraHost<'_> {
    fn app_name(&self) -> &str {
        "Letra"
    }

    fn guide(&self) -> String {
        GUIDE.into()
    }

    fn context(&mut self) -> Result<Value, String> {
        let inspected = self.run("document.inspect", json!({"text": true}))?;
        Ok(snapshot(&inspected))
    }

    fn tools(&self) -> Vec<ToolSpec> {
        vec![
            tool(
                "read_blocks",
                "Read blocks of the body in full: paragraphs with their style and text, tables with their cells. Use it when the snapshot was shortened or to read a part again.",
                object(
                    json!({
                        "from": json!({"type": "integer", "minimum": 0, "description": "Index of the first block (default 0)"}),
                        "count": json!({"type": "integer", "minimum": 1, "description": format!("How many blocks, at most {MAX_BLOCKS} (default 50)")}),
                    }),
                    &[],
                ),
                true,
            ),
            tool(
                "get_selection",
                "The selected text and the formatting at the caret: font, size, bold, italic, style, alignment.",
                object(json!({}), &[]),
                true,
            ),
            tool(
                "get_paragraph",
                "Everything about one paragraph of the body: its properties and its resolved formatting.",
                object(json!({"index": json!({"type": "integer", "minimum": 0, "description": "The block's index"})}), &["index"]),
                true,
            ),
            tool(
                "list_commands",
                "Find Letra commands by what they do. Searches each command's id, label, ribbon location and params. Returns up to 60.",
                object(json!({"query": text("Words to look for, e.g. \"line spacing\", \"table\", \"footnote\"")}), &[]),
                true,
            ),
            tool(
                "select_text",
                "Select the n-th occurrence of some exact text in the body. Does nothing to the document; later edits act on it.",
                object(
                    json!({
                        "text": text("The exact text to select"),
                        "occurrence": json!({"type": "integer", "minimum": 1, "description": "Which occurrence, counting from 1 (default 1)"}),
                    }),
                    &["text"],
                ),
                true,
            ),
            tool(
                "type_text",
                "Type text at the caret, replacing the selection. With paragraphs=true each line becomes a paragraph.",
                object(
                    json!({
                        "text": text("The text"),
                        "paragraphs": json!({"type": "boolean", "description": "Split the lines into paragraphs"}),
                    }),
                    &["text"],
                ),
                false,
            ),
            tool(
                "insert_table",
                "Insert a table at the caret, filled in one call: rows of cells, all the same length.",
                object(
                    json!({
                        "values": json!({"type": "array", "description": "Rows of cells", "items": {"type": "array", "items": {"type": ["string", "number", "boolean", "null"]}}}),
                        "header": json!({"type": "boolean", "description": "The first row is a header row"}),
                        "style": text("A table style name"),
                    }),
                    &["values"],
                ),
                false,
            ),
            tool(
                "run_command",
                "Run a Letra command by id with params, as the ribbon button would on the selection, e.g. format.bold or insert.pageBreak. Find ids with list_commands. File, window and print commands are not available.",
                object(
                    json!({
                        "command": text("The command id"),
                        "params": json!({"type": "object", "description": "The command's params"}),
                    }),
                    &["command"],
                ),
                false,
            ),
            tool(
                "set_document_text",
                "Replace the whole body with plain text. Loses every style and format: only for starting a document from scratch.",
                object(json!({"text": text("The new text")}), &["text"]),
                false,
            ),
        ]
    }

    fn call(&mut self, tool: &str, input: &Value) -> Result<Value, String> {
        match tool {
            "read_blocks" => self.read_blocks(input),
            "get_selection" => self.run("document.selection", json!({})),
            "get_paragraph" => {
                let index = input.get("index").and_then(Value::as_u64).ok_or("`index` is missing; it must be a block index")?;
                self.run("document.paragraph", json!({"path": [index]}))
            }
            "list_commands" => Ok(self.list_commands(input.get("query").and_then(Value::as_str).unwrap_or(""))),
            "select_text" => {
                let occurrence = input.get("occurrence").and_then(Value::as_u64).unwrap_or(1);
                self.run("select.text", json!({"text": str_arg(input, "text")?, "occurrence": occurrence}))
            }
            "type_text" => {
                let paragraphs = input.get("paragraphs").and_then(Value::as_bool).unwrap_or(false);
                let mut params = json!({"text": str_arg(input, "text")?, "raw": true});
                if paragraphs {
                    params["paragraphs"] = json!(true);
                }
                self.edit("text.insert", params)
            }
            "insert_table" => {
                let mut params = json!({"values": input.get("values").cloned().ok_or("`values` is missing; give an array of rows")?});
                for key in ["header", "style"] {
                    if let Some(v) = input.get(key) {
                        params[key] = v.clone();
                    }
                }
                self.edit("insert.spreadsheet", params)
            }
            "run_command" => {
                let command = str_arg(input, "command")?;
                if forbidden(command) {
                    return Err(format!("`{command}` is not available to the assistant: files, windows and printing are the person's to handle"));
                }
                let params = input.get("params").cloned().unwrap_or_else(|| json!({}));
                self.edit(command, params)
            }
            "set_document_text" => self.edit("document.setText", json!({"text": str_arg(input, "text")?})),
            other => Err(format!("there is no tool called `{other}`")),
        }
    }

    fn describe(&self, tool: &str, input: &Value) -> Option<String> {
        let es = self.lang == Lang::Spanish;
        let arg = |k: &str| input.get(k).and_then(Value::as_str).map(|s| clip(s, PREVIEW));
        Some(match tool {
            "type_text" => {
                let t = arg("text")?;
                if es { format!("Escribir «{t}»") } else { format!("Type “{t}”") }
            }
            "select_text" => {
                let t = arg("text")?;
                if es { format!("Seleccionar «{t}»") } else { format!("Select “{t}”") }
            }
            "insert_table" => {
                let rows = input.get("values").and_then(Value::as_array)?;
                let cols = rows.first().and_then(Value::as_array).map_or(0, Vec::len);
                if es { format!("Insertar una tabla de {} × {}", rows.len(), cols) } else { format!("Insert a table of {} × {}", rows.len(), cols) }
            }
            "run_command" => {
                let c = arg("command")?;
                let params = input.get("params").filter(|p| p.as_object().is_some_and(|o| !o.is_empty()));
                let params = params.map(|p| clip(&p.to_string(), PREVIEW)).unwrap_or_default();
                let label = self.app.session.registry.get(&c).map(|s| s.label.to_string());
                let name = label.map_or_else(|| c.clone(), |l| format!("{l} ({c})"));
                match (es, params.is_empty()) {
                    (true, true) => format!("Ejecutar {name}"),
                    (true, false) => format!("Ejecutar {name} {params}"),
                    (false, true) => format!("Run {name}"),
                    (false, false) => format!("Run {name} {params}"),
                }
            }
            "set_document_text" => {
                if es {
                    "Reemplazar todo el texto del documento".into()
                } else {
                    "Replace all the text of the document".into()
                }
            }
            _ => return None,
        })
    }

    fn suggestions(&self, lang: Lang) -> Vec<String> {
        let s: [&str; 4] = if lang == Lang::Spanish {
            [
                "Resume este documento en cinco puntos",
                "Corrige la ortografía y la gramática",
                "Reescribe el texto seleccionado con un tono más formal",
                "Dale formato de informe con títulos y una tabla de contenido",
            ]
        } else {
            [
                "Summarise this document in five points",
                "Fix the spelling and grammar",
                "Rewrite the selected text in a more formal tone",
                "Format this as a report with headings and a table of contents",
            ]
        };
        s.iter().map(|s| (*s).to_string()).collect()
    }

    fn undo_depth(&mut self) -> usize {
        self.app.session.undo_depth()
    }

    fn undo(&mut self, steps: usize) -> Result<(), String> {
        let done = self.app.session.undo_steps(steps);
        if done == 0 && steps > 0 { Err("nothing to undo".into()) } else { Ok(()) }
    }
}

#[cfg(test)]
mod tests;
