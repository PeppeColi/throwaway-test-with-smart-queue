//! Tests of the `ricette list` subcommands.
//!
//! They run the compiled `ricette` binary as a subprocess, with a temporary
//! directory as its working directory, and assert stdout, stderr, the exit
//! status and the parsed contents of `shopping-list.json`.

use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

const FILE_NAME: &str = "shopping-list.json";
const EMPTY_MESSAGE: &str = "Shopping list is empty.\n";

/// A temporary working directory, removed when the test ends.
struct WorkDir {
    path: PathBuf,
}

impl WorkDir {
    fn new(test_name: &str) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "ricette-{}-{}-{}",
            test_name,
            std::process::id(),
            unique
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temporary working directory");
        WorkDir { path }
    }

    fn file(&self) -> PathBuf {
        self.path.join(FILE_NAME)
    }

    fn write_file(&self, contents: &str) {
        fs::write(self.file(), contents).expect("write shopping-list.json");
    }

    fn file_text(&self) -> String {
        fs::read_to_string(self.file()).expect("read shopping-list.json")
    }

    fn read_json(&self) -> Value {
        serde_json::from_str(&self.file_text()).expect("shopping-list.json is valid JSON")
    }

    fn run(&self, args: &[&str]) -> Output {
        run_in(&self.path, args)
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ricette"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run the ricette binary")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

/// The file contents the new format writes for plain items.
fn items(names: &[&str]) -> Value {
    Value::Array(names.iter().map(|name| json!({ "item": name })).collect())
}

#[test]
fn add_creates_the_file_and_prints_nothing() {
    let dir = WorkDir::new("add_creates");

    let output = dir.run(&["list", "add", "milk"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(dir.read_json(), items(&["milk"]));
}

#[test]
fn show_prints_items_in_insertion_order() {
    let dir = WorkDir::new("show_order");
    dir.run(&["list", "add", "milk"]);
    dir.run(&["list", "add", "eggs"]);

    let output = dir.run(&["list", "show"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "milk\neggs\n");
}

#[test]
fn show_without_a_file_prints_empty_message_and_creates_no_file() {
    let dir = WorkDir::new("show_missing");

    let output = dir.run(&["list", "show"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), EMPTY_MESSAGE);
    assert!(!dir.file().exists(), "show must not create the file");
}

#[test]
fn clear_without_a_file_creates_an_empty_array() {
    let dir = WorkDir::new("clear_missing");

    let output = dir.run(&["list", "clear"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(dir.read_json(), items(&[]));
}

#[test]
fn clear_empties_the_list() {
    let dir = WorkDir::new("clear_entries");
    dir.run(&["list", "add", "milk"]);
    dir.run(&["list", "add", "eggs"]);

    let output = dir.run(&["list", "clear"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(dir.read_json(), items(&[]));

    let shown = dir.run(&["list", "show"]);
    assert!(shown.status.success(), "stderr: {}", stderr(&shown));
    assert_eq!(stdout(&shown), EMPTY_MESSAGE);
}

#[test]
fn item_with_inner_space_is_kept_whole() {
    let dir = WorkDir::new("inner_space");

    let output = dir.run(&["list", "add", "olive oil"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    assert_eq!(dir.read_json(), items(&["olive oil"]));
    let shown = dir.run(&["list", "show"]);
    assert_eq!(stdout(&shown), "olive oil\n");
}

#[test]
fn item_is_trimmed_before_it_is_stored() {
    let dir = WorkDir::new("trim");

    let output = dir.run(&["list", "add", " milk "]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    assert_eq!(dir.read_json(), items(&["milk"]));
    let shown = dir.run(&["list", "show"]);
    assert_eq!(stdout(&shown), "milk\n");
}

#[test]
fn adding_the_same_item_twice_keeps_one_line() {
    let dir = WorkDir::new("duplicate");
    dir.run(&["list", "add", "milk"]);

    let output = dir.run(&["list", "add", "milk"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");

    let shown = dir.run(&["list", "show"]);
    assert!(shown.status.success(), "stderr: {}", stderr(&shown));
    assert_eq!(stdout(&shown), "milk\n");
    assert_eq!(dir.read_json(), items(&["milk"]));
}

#[test]
fn add_with_quantity_and_unit_is_shown_with_both() {
    let dir = WorkDir::new("measured_add");

    let output = dir.run(&["list", "add", "milk", "--qty", "2", "--unit", "l"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        dir.read_json(),
        json!([{ "item": "milk", "qty": 2, "unit": "l" }])
    );

    let shown = dir.run(&["list", "show"]);
    assert!(shown.status.success(), "stderr: {}", stderr(&shown));
    assert_eq!(stdout(&shown), "milk 2 l\n");
}

#[test]
fn flags_may_come_in_either_order() {
    let dir = WorkDir::new("flag_order");

    let output = dir.run(&["list", "add", "milk", "--unit", "l", "--qty", "2"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    let shown = dir.run(&["list", "show"]);
    assert_eq!(stdout(&shown), "milk 2 l\n");
}

#[test]
fn same_unit_adds_quantities_and_keeps_the_first_position() {
    let dir = WorkDir::new("same_unit");
    dir.run(&["list", "add", "milk", "--qty", "2", "--unit", "l"]);
    dir.run(&["list", "add", "eggs"]);

    let output = dir.run(&["list", "add", "milk", "--qty", "1", "--unit", "l"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    let shown = dir.run(&["list", "show"]);
    assert_eq!(stdout(&shown), "milk 3 l\neggs\n");
    assert_eq!(
        dir.read_json(),
        json!([{ "item": "milk", "qty": 3, "unit": "l" }, { "item": "eggs" }])
    );
}

#[test]
fn different_unit_stays_a_separate_line_in_insertion_order() {
    let dir = WorkDir::new("other_unit");
    dir.run(&["list", "add", "milk", "--qty", "2", "--unit", "l"]);

    let output = dir.run(&["list", "add", "milk", "--qty", "1", "--unit", "kg"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    let shown = dir.run(&["list", "show"]);
    assert_eq!(stdout(&shown), "milk 2 l\nmilk 1 kg\n");
}

#[test]
fn plain_item_stays_a_separate_line_from_a_measured_one() {
    let dir = WorkDir::new("plain_and_measured");
    dir.run(&["list", "add", "milk", "--qty", "2", "--unit", "l"]);

    let output = dir.run(&["list", "add", "milk"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    let shown = dir.run(&["list", "show"]);
    assert_eq!(stdout(&shown), "milk 2 l\nmilk\n");
}

#[test]
fn unit_is_trimmed_and_compared_case_sensitively() {
    let dir = WorkDir::new("unit_rules");
    dir.run(&["list", "add", "milk", "--qty", "1", "--unit", " l "]);
    dir.run(&["list", "add", "milk", "--qty", "1", "--unit", "L"]);
    dir.run(&["list", "add", "milk", "--qty", "4", "--unit", "l"]);

    let shown = dir.run(&["list", "show"]);

    assert!(shown.status.success(), "stderr: {}", stderr(&shown));
    assert_eq!(stdout(&shown), "milk 5 l\nmilk 1 L\n");
    assert_eq!(
        dir.read_json(),
        json!([
            { "item": "milk", "qty": 5, "unit": "l" },
            { "item": "milk", "qty": 1, "unit": "L" }
        ])
    );
}

#[test]
fn invalid_quantity_or_unit_is_rejected_and_the_list_is_unchanged() {
    let argument_lists: [&[&str]; 8] = [
        &["list", "add", "milk", "--qty", "2"],
        &["list", "add", "milk", "--unit", "l"],
        &["list", "add", "milk", "--qty", "0", "--unit", "l"],
        &["list", "add", "milk", "--qty", "-1", "--unit", "l"],
        &["list", "add", "milk", "--qty", "1.5", "--unit", "l"],
        &["list", "add", "milk", "--qty", "two", "--unit", "l"],
        &["list", "add", "milk", "--qty", "2", "--unit", ""],
        &["list", "add", "milk", "--qty", "2", "--unit", "   "],
    ];

    for args in argument_lists {
        let dir = WorkDir::new("invalid_measure");
        dir.run(&["list", "add", "eggs"]);
        let before = dir.file_text();

        let output = dir.run(args);

        assert!(!output.status.success(), "{args:?} must exit non-zero");
        assert!(
            !stderr(&output).is_empty(),
            "{args:?} must write an error to stderr"
        );
        assert_eq!(
            dir.file_text(),
            before,
            "{args:?} must leave the list unchanged"
        );
    }
}

#[test]
fn empty_item_with_quantity_and_unit_is_rejected_and_the_list_is_unchanged() {
    for item in ["", "   "] {
        let dir = WorkDir::new("empty_measured_item");
        dir.run(&["list", "add", "milk"]);

        let output = dir.run(&["list", "add", item, "--qty", "1", "--unit", "l"]);

        assert!(
            !output.status.success(),
            "adding {item:?} must exit non-zero"
        );
        assert!(
            !stderr(&output).is_empty(),
            "adding {item:?} must write an error to stderr"
        );
        assert_eq!(dir.read_json(), items(&["milk"]));
    }
}

#[test]
fn legacy_file_is_read_as_plain_entries_and_show_does_not_rewrite_it() {
    let dir = WorkDir::new("legacy_show");
    let legacy = r#"["milk","milk","eggs"]"#;
    dir.write_file(legacy);

    let output = dir.run(&["list", "show"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "milk\neggs\n");
    assert_eq!(dir.file_text(), legacy, "show must not write the file");
}

#[test]
fn add_rewrites_a_legacy_file_as_objects_and_keeps_its_entries() {
    let dir = WorkDir::new("legacy_add");
    dir.write_file(r#"["milk","eggs"]"#);

    let output = dir.run(&["list", "add", "milk", "--qty", "2", "--unit", "l"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        dir.read_json(),
        json!([
            { "item": "milk" },
            { "item": "eggs" },
            { "item": "milk", "qty": 2, "unit": "l" }
        ])
    );

    let shown = dir.run(&["list", "show"]);
    assert!(shown.status.success(), "stderr: {}", stderr(&shown));
    assert_eq!(stdout(&shown), "milk\neggs\nmilk 2 l\n");
}

#[test]
fn entries_with_the_same_item_and_unit_are_combined_on_read() {
    let dir = WorkDir::new("combine_on_read");
    dir.write_file(
        r#"[{"item":"milk","qty":2,"unit":"l"},{"item":"eggs"},{"item":"milk","qty":3,"unit":"l"},"eggs"]"#,
    );

    let output = dir.run(&["list", "show"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "milk 5 l\neggs\n");
}

#[test]
fn invalid_file_makes_every_subcommand_fail_and_leaves_the_file_unchanged() {
    let invalid_contents = [
        "this is not json",
        r#"{"milk": true}"#,
        r#"[1, 2]"#,
        r#"["milk", 3]"#,
        r#"["milk", null]"#,
        r#"[{"item":"milk","qty":2}]"#,
        r#"[{"item":"milk","unit":"l"}]"#,
        r#"[{"item":"milk","qty":0,"unit":"l"}]"#,
        r#"[{"item":"milk","qty":-1,"unit":"l"}]"#,
        r#"[{"item":"milk","qty":1.5,"unit":"l"}]"#,
        r#"[{"item":"milk","qty":2,"unit":""}]"#,
        r#"[{"qty":2,"unit":"l"}]"#,
        r#"[{"item":3}]"#,
    ];
    let subcommands: [&[&str]; 3] = [
        &["list", "show"],
        &["list", "add", "eggs"],
        &["list", "clear"],
    ];

    for contents in invalid_contents {
        for args in subcommands {
            let dir = WorkDir::new("invalid_file");
            dir.write_file(contents);

            let output = dir.run(args);

            assert!(
                !output.status.success(),
                "{args:?} on {contents:?} must exit non-zero"
            );
            assert!(
                !stderr(&output).is_empty(),
                "{args:?} on {contents:?} must write an error to stderr"
            );
            assert_eq!(
                dir.file_text(),
                contents,
                "{args:?} on {contents:?} must leave the file unchanged"
            );
        }
    }
}

#[test]
fn empty_or_whitespace_item_is_rejected_and_the_list_is_unchanged() {
    for item in ["", "   ", "\t"] {
        let dir = WorkDir::new("empty_item");
        dir.run(&["list", "add", "milk"]);

        let output = dir.run(&["list", "add", item]);

        assert!(
            !output.status.success(),
            "adding {item:?} must exit non-zero"
        );
        assert!(
            !stderr(&output).is_empty(),
            "adding {item:?} must write an error to stderr"
        );
        assert_eq!(dir.read_json(), items(&["milk"]));
    }
}
