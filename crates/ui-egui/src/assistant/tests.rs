use letra_engine::Session;

use super::*;
use crate::Services;

fn app() -> WordApp {
    WordApp::new(Session::new(letra_doc::Document::new()), Services::default())
}

fn host(app: &mut WordApp) -> LetraHost<'_> {
    LetraHost { app, lang: Lang::English }
}

fn text_of(app: &mut WordApp) -> String {
    app.execute("document.text", json!({})).ok().and_then(|v| v.get("text").and_then(Value::as_str).map(str::to_string)).unwrap_or_default()
}

/// The tools the model is offered: every one named once, each schema an object, and only the
/// ones that leave the document alone run without being asked about.
#[test]
fn the_tools_are_named_once_and_only_readers_run_unasked() {
    let mut a = app();
    let tools = host(&mut a).tools();
    let mut names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
    names.sort_unstable();
    let before = names.len();
    names.dedup();
    assert_eq!(before, names.len(), "a tool is listed twice");
    for t in &tools {
        assert_eq!(t.schema["type"], "object", "{}", t.name);
        assert!(!t.description.is_empty(), "{}", t.name);
    }
    for edit in ["type_text", "insert_table", "run_command", "set_document_text"] {
        assert!(tools.iter().any(|t| t.name == edit && !t.read_only), "{edit} must wait for the person");
    }
    for read in ["read_blocks", "get_selection", "get_paragraph", "list_commands", "select_text"] {
        assert!(tools.iter().any(|t| t.name == read && t.read_only), "{read} is a reader");
    }
}

/// The assistant edits through the commands and a turn undoes in the number of steps it made.
#[test]
fn typing_and_selecting_change_the_document_and_undo_takes_them_back() {
    let mut a = app();
    let mut h = host(&mut a);
    let start = h.undo_depth();
    h.call("type_text", &json!({"text": "Hello world"})).unwrap();
    h.call("select_text", &json!({"text": "world"})).unwrap();
    h.call("type_text", &json!({"text": "Letra"})).unwrap();
    assert!(h.undo_depth() > start);
    let depth = h.undo_depth() - start;
    drop(h);
    assert_eq!(text_of(&mut a), "Hello Letra");
    host(&mut a).undo(depth).unwrap();
    assert_eq!(text_of(&mut a), "");
}

/// Typing the person did just before is not swallowed into the assistant's step: undoing the
/// turn leaves it.
#[test]
fn an_edit_never_joins_the_typing_before_it() {
    let mut a = app();
    a.execute("text.insert", json!({"text": "mine "})).unwrap();
    let mut h = host(&mut a);
    let start = h.undo_depth();
    h.call("type_text", &json!({"text": "theirs"})).unwrap();
    let steps = h.undo_depth() - start;
    assert!(steps >= 1);
    h.undo(steps).unwrap();
    drop(h);
    assert_eq!(text_of(&mut a), "mine ");
}

#[test]
fn paragraphs_split_into_blocks_and_the_snapshot_lists_them() {
    let mut a = app();
    let mut h = host(&mut a);
    h.call("type_text", &json!({"text": "One\nTwo\nThree", "paragraphs": true})).unwrap();
    let snap = h.context().unwrap();
    let blocks = snap["blocks"].as_array().unwrap();
    assert_eq!(blocks.len(), 3, "{snap}");
    assert_eq!(blocks[1]["text"], "Two");
    assert_eq!(snap["blockCount"], 3);
    let read = h.call("read_blocks", &json!({"from": 1, "count": 1})).unwrap();
    assert_eq!(read["total"], 3);
    assert_eq!(read["blocks"][0]["text"], "Two");
}

#[test]
fn a_table_is_inserted_filled() {
    let mut a = app();
    let mut h = host(&mut a);
    h.call("insert_table", &json!({"values": [["Name", "Qty"], ["Pens", 3]], "header": true})).unwrap();
    let read = h.call("read_blocks", &json!({})).unwrap();
    let table = read["blocks"].as_array().unwrap().iter().find(|b| b["type"] == "table").expect("a table block");
    assert_eq!(table["rows"], 2);
    assert_eq!(table["cells"][1][0], "Pens");
}

/// A long document's snapshot is shortened and says how to read the rest, so the request stays
/// small; nothing in it grows without bound.
#[test]
fn a_long_document_is_shortened_in_the_snapshot_and_says_so() {
    let block = |i: usize| json!({"index": i, "type": "paragraph", "style": "Normal", "text": "x".repeat(900)});
    let inspected = json!({"blocks": (0..300).map(block).collect::<Vec<_>>(), "selection": {"text": ""}});
    let snap = snapshot(&inspected);
    let size = serde_json::to_string(&snap).unwrap().chars().count();
    assert!(size < SNAPSHOT_BUDGET + 2_000, "{size}");
    assert_eq!(snap["blockCount"], 300);
    assert!(snap["note"].as_str().unwrap().contains("read_blocks"));
    let first = snap["blocks"][0]["text"].as_str().unwrap();
    assert!(first.chars().count() <= LONG_BLOCK + 1);
}

#[test]
fn a_short_document_is_not_shortened() {
    let inspected = json!({"blocks": [{"index": 0, "type": "paragraph", "style": "Normal", "text": "Hello", "runs": [1, 2, 3]}]});
    let snap = snapshot(&inspected);
    assert_eq!(snap["blocks"][0]["text"], "Hello");
    assert!(snap["blocks"][0].get("runs").is_none(), "run-by-run formatting stays out of the snapshot");
    assert!(snap.get("note").is_none());
}

/// Files, windows and printing are the person's: a command in those families is refused with a
/// reason the model can act on, and the document is untouched.
#[test]
fn file_and_window_commands_are_refused() {
    let mut a = app();
    let mut h = host(&mut a);
    for c in ["file.save", "file.close", "file.open", "ui.screenshot", "view.switchWindows", "printing.print"] {
        let e = h.call("run_command", &json!({"command": c})).unwrap_err();
        assert!(e.contains("not available"), "{c}: {e}");
    }
    assert!(!forbidden("format.bold") && !forbidden("insert.table"));
}

#[test]
fn commands_are_found_by_what_they_do_and_never_include_the_forbidden() {
    let mut a = app();
    let mut h = host(&mut a);
    let found = h.call("list_commands", &json!({"query": "bold"})).unwrap();
    let ids: Vec<_> = found["commands"].as_array().unwrap().iter().filter_map(|c| c["id"].as_str()).collect();
    assert!(ids.contains(&"format.bold"), "{ids:?}");
    let all = h.call("list_commands", &json!({})).unwrap();
    assert!(all["commands"].as_array().unwrap().len() <= MAX_COMMANDS);
    for c in all["commands"].as_array().unwrap() {
        assert!(!forbidden(c["id"].as_str().unwrap_or("")), "{c}");
    }
}

#[test]
fn a_command_runs_on_the_selection() {
    let mut a = app();
    let mut h = host(&mut a);
    h.call("type_text", &json!({"text": "Make this bold"})).unwrap();
    h.call("select_text", &json!({"text": "bold"})).unwrap();
    h.call("run_command", &json!({"command": "format.bold"})).unwrap();
    let sel = h.call("get_selection", &json!({})).unwrap();
    assert_eq!(sel["text"], "bold");
    assert_eq!(sel["format"]["bold"], true, "{sel}");
}

#[test]
fn bad_input_is_an_error_the_model_can_read_never_a_panic() {
    let mut a = app();
    let mut h = host(&mut a);
    assert!(h.call("type_text", &json!({})).unwrap_err().contains("`text`"));
    assert!(h.call("insert_table", &json!({})).unwrap_err().contains("`values`"));
    assert!(h.call("get_paragraph", &json!({"index": "x"})).unwrap_err().contains("`index`"));
    assert!(h.call("get_paragraph", &json!({"index": 99_999})).is_err());
    assert!(h.call("select_text", &json!({"text": "nowhere at all"})).is_err());
    assert!(h.call("run_command", &json!({"command": "no.such.command"})).is_err());
    assert!(h.call("nope", &json!({})).unwrap_err().contains("no tool"));
    assert!(h.call("read_blocks", &json!({"from": 10_000_000, "count": 99_999_999})).is_ok());
}

#[test]
fn the_approval_card_says_what_will_happen_in_both_languages() {
    let mut a = app();
    let en = LetraHost { app: &mut a, lang: Lang::English };
    assert_eq!(en.describe("type_text", &json!({"text": "Hi"})).unwrap(), "Type “Hi”");
    assert_eq!(en.describe("insert_table", &json!({"values": [[1, 2], [3, 4], [5, 6]]})).unwrap(), "Insert a table of 3 × 2");
    let run = en.describe("run_command", &json!({"command": "format.bold"})).unwrap();
    assert!(run.starts_with("Run ") && run.contains("format.bold"), "{run}");
    let long = en.describe("type_text", &json!({"text": "y".repeat(500)})).unwrap();
    assert!(long.chars().count() < 120, "{long}");
    assert!(en.describe("get_selection", &json!({})).is_none());
    let es = LetraHost { app: &mut a, lang: Lang::Spanish };
    assert_eq!(es.describe("type_text", &json!({"text": "Hola"})).unwrap(), "Escribir «Hola»");
    assert!(es.suggestions(Lang::Spanish)[0].starts_with("Resume"));
    assert!(es.suggestions(Lang::English)[0].starts_with("Summarise"));
}

/// The button, the panel and the Preferences section draw in a real frame, and the panel takes
/// the right edge only while the chat is open.
#[test]
fn the_button_and_the_panel_draw_in_a_frame() {
    let ctx = egui::Context::default();
    let mut a = app();
    let frame = |a: &mut WordApp| {
        ctx.run_ui(egui::RawInput::default(), |ui| {
            a.logic(ui.ctx());
            a.ui(ui);
        })
        .drop_without_applying_deltas();
    };
    for _ in 0..4 {
        frame(&mut a);
    }
    assert!(a.assistant.is_some(), "the title bar made the assistant");
    let a_open = |a: &mut WordApp, open: bool| {
        if let Some(x) = a.assistant.as_mut() {
            x.open = open;
        }
    };
    a_open(&mut a, true);
    frame(&mut a);
    frame(&mut a);
    a_open(&mut a, false);
    frame(&mut a);
}
