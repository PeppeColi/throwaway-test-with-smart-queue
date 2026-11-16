//! Tests of `ricette list add-recipe <nome>`.
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

/// A temporary working directory, removed when the test ends.
struct WorkDir {
    path: PathBuf,
}

impl WorkDir {
    fn new(test_name: &str) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "ricette-add-recipe-{}-{}-{}",
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

/// The file contents the current format writes for measured entries.
fn measured(entries: &[(&str, u64, &str)]) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|(item, qty, unit)| json!({ "item": item, "qty": qty, "unit": unit }))
            .collect(),
    )
}

#[test]
fn add_recipe_to_an_empty_list_writes_its_ingredients_in_recipe_order() {
    let dir = WorkDir::new("empty_spaghetti");

    let output = dir.run(&["list", "add-recipe", "Spaghetti al pomodoro"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "");
    assert_eq!(
        dir.read_json(),
        measured(&[
            ("spaghetti", 320, "g"),
            ("tomato", 400, "g"),
            ("olive oil", 2, "tbsp"),
            ("garlic", 2, "pcs"),
        ])
    );
}

#[test]
fn list_show_prints_each_ingredient_of_the_recipe_on_its_own_line() {
    let dir = WorkDir::new("show_omelette");
    dir.run(&["list", "add-recipe", "Omelette"]);

    let output = dir.run(&["list", "show"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "eggs 3 pcs\nmilk 50 ml\nsalt 1 pinch\nbutter 10 g\n"
    );
}

#[test]
fn same_ingredient_and_unit_adds_the_quantity_and_keeps_the_first_position() {
    let dir = WorkDir::new("sum_tomato");
    dir.run(&["list", "add", "tomato", "--qty", "100", "--unit", "g"]);

    let output = dir.run(&["list", "add-recipe", "Spaghetti al pomodoro"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        dir.read_json(),
        measured(&[
            ("tomato", 500, "g"),
            ("spaghetti", 320, "g"),
            ("olive oil", 2, "tbsp"),
            ("garlic", 2, "pcs"),
        ])
    );
}

#[test]
fn same_ingredient_with_another_unit_stays_a_separate_entry() {
    let dir = WorkDir::new("other_unit_milk");
    dir.run(&["list", "add", "milk", "--qty", "1", "--unit", "l"]);

    let output = dir.run(&["list", "add-recipe", "Omelette"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        dir.read_json(),
        measured(&[
            ("milk", 1, "l"),
            ("eggs", 3, "pcs"),
            ("milk", 50, "ml"),
            ("salt", 1, "pinch"),
            ("butter", 10, "g"),
        ])
    );
}

#[test]
fn adding_the_same_recipe_twice_adds_up_its_quantities() {
    let dir = WorkDir::new("twice_omelette");

    dir.run(&["list", "add-recipe", "Omelette"]);
    let output = dir.run(&["list", "add-recipe", "Omelette"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        dir.read_json(),
        measured(&[
            ("eggs", 6, "pcs"),
            ("milk", 100, "ml"),
            ("salt", 2, "pinch"),
            ("butter", 20, "g"),
        ])
    );
}

#[test]
fn recipe_name_is_matched_ignoring_case_and_surrounding_spaces() {
    let exact = WorkDir::new("name_exact");
    exact.run(&["list", "add-recipe", "Spaghetti al pomodoro"]);
    let lowercase = WorkDir::new("name_lowercase");
    let output = lowercase.run(&["list", "add-recipe", "spaghetti al pomodoro"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let padded = WorkDir::new("name_padded");
    let output = padded.run(&["list", "add-recipe", "  SPAGHETTI AL POMODORO  "]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    assert_eq!(lowercase.read_json(), exact.read_json());
    assert_eq!(padded.read_json(), exact.read_json());
}

#[test]
fn unknown_recipe_exits_non_zero_and_creates_no_file() {
    let dir = WorkDir::new("unknown_no_file");

    let output = dir.run(&["list", "add-recipe", "Chocolate cake"]);

    assert_ne!(output.status.code(), Some(0), "exit status");
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "No recipe named \"Chocolate cake\".\n");
    assert!(
        !dir.file().exists(),
        "an unknown recipe must not create the file"
    );
}

#[test]
fn unknown_recipe_leaves_an_existing_list_unchanged() {
    let dir = WorkDir::new("unknown_keeps_list");
    dir.run(&["list", "add", "milk", "--qty", "1", "--unit", "l"]);
    let before = dir.file_text();

    let output = dir.run(&["list", "add-recipe", "Chocolate cake"]);

    assert_ne!(output.status.code(), Some(0), "exit status");
    assert_eq!(stderr(&output), "No recipe named \"Chocolate cake\".\n");
    assert_eq!(dir.file_text(), before);
}

#[test]
fn invalid_list_file_exits_non_zero_and_is_left_unchanged() {
    let dir = WorkDir::new("invalid_file");
    let invalid = "{\"not\": \"a list\"}";
    dir.write_file(invalid);

    let output = dir.run(&["list", "add-recipe", "Omelette"]);

    assert_ne!(output.status.code(), Some(0), "exit status");
    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).contains("cannot use shopping-list.json"),
        "stderr should explain the invalid file, got: {}",
        stderr(&output)
    );
    assert_eq!(dir.file_text(), invalid);
}

#[test]
fn recipes_list_still_prints_only_the_recipe_names() {
    let dir = WorkDir::new("recipes_list_unchanged");

    let output = dir.run(&["recipes", "list"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "Spaghetti al pomodoro\nOmelette\nPancakes\nInsalata caprese\nMinestrone\n"
    );
}

#[test]
fn recipes_search_still_prints_only_the_recipe_names() {
    let dir = WorkDir::new("recipes_search_unchanged");

    let output = dir.run(&["recipes", "search", "tomato"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "Spaghetti al pomodoro\nInsalata caprese\nMinestrone\n"
    );
}
