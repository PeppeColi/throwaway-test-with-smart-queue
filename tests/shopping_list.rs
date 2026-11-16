//! Tests of the `ricette list` subcommands.
//!
//! They run the compiled `ricette` binary as a subprocess, with a temporary
//! directory as its working directory, and assert stdout, stderr, the exit
//! status and the parsed contents of `shopping-list.json`.

use serde_json::Value;
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

    fn read_json(&self) -> Value {
        let text = fs::read_to_string(self.file()).expect("read shopping-list.json");
        serde_json::from_str(&text).expect("shopping-list.json is valid JSON")
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

fn strings(values: &[&str]) -> Value {
    Value::Array(
        values
            .iter()
            .map(|v| Value::String(v.to_string()))
            .collect(),
    )
}

#[test]
fn add_creates_the_file_and_prints_nothing() {
    let dir = WorkDir::new("add_creates");

    let output = dir.run(&["list", "add", "milk"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(dir.read_json(), strings(&["milk"]));
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
    assert_eq!(dir.read_json(), strings(&[]));
}

#[test]
fn clear_empties_the_list() {
    let dir = WorkDir::new("clear_entries");
    dir.run(&["list", "add", "milk"]);
    dir.run(&["list", "add", "eggs"]);

    let output = dir.run(&["list", "clear"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(dir.read_json(), strings(&[]));

    let shown = dir.run(&["list", "show"]);
    assert!(shown.status.success(), "stderr: {}", stderr(&shown));
    assert_eq!(stdout(&shown), EMPTY_MESSAGE);
}

#[test]
fn item_with_inner_space_is_kept_whole() {
    let dir = WorkDir::new("inner_space");

    let output = dir.run(&["list", "add", "olive oil"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    assert_eq!(dir.read_json(), strings(&["olive oil"]));
    let shown = dir.run(&["list", "show"]);
    assert_eq!(stdout(&shown), "olive oil\n");
}

#[test]
fn item_is_trimmed_before_it_is_stored() {
    let dir = WorkDir::new("trim");

    let output = dir.run(&["list", "add", " milk "]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    assert_eq!(dir.read_json(), strings(&["milk"]));
    let shown = dir.run(&["list", "show"]);
    assert_eq!(stdout(&shown), "milk\n");
}

#[test]
fn adding_the_same_item_twice_keeps_both() {
    let dir = WorkDir::new("duplicate");
    dir.run(&["list", "add", "milk"]);
    dir.run(&["list", "add", "milk"]);

    let shown = dir.run(&["list", "show"]);

    assert!(shown.status.success(), "stderr: {}", stderr(&shown));
    assert_eq!(stdout(&shown), "milk\nmilk\n");
}

#[test]
fn invalid_file_makes_every_subcommand_fail_and_leaves_the_file_unchanged() {
    let invalid_contents = [
        "this is not json",
        r#"{"milk": true}"#,
        r#"[1, 2]"#,
        r#"["milk", 3]"#,
        r#"["milk", null]"#,
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
                fs::read_to_string(dir.file()).expect("read shopping-list.json"),
                contents,
                "{args:?} on {contents:?} must leave the file unchanged"
            );
        }
    }
}

#[test]
fn concurrent_adds_are_all_kept() {
    let dir = WorkDir::new("concurrent");
    let items: Vec<String> = (0..16).map(|n| format!("item{n}")).collect();

    let children: Vec<_> = items
        .iter()
        .map(|item| {
            Command::new(env!("CARGO_BIN_EXE_ricette"))
                .args(["list", "add", item])
                .current_dir(&dir.path)
                .spawn()
                .expect("spawn the ricette binary")
        })
        .collect();
    for mut child in children {
        assert!(child.wait().expect("wait").success());
    }

    let Value::Array(stored) = dir.read_json() else {
        panic!("shopping-list.json is not an array");
    };
    let mut stored: Vec<String> = stored
        .into_iter()
        .map(|v| v.as_str().expect("string").to_string())
        .collect();
    stored.sort();
    let mut expected = items.clone();
    expected.sort();
    assert_eq!(stored, expected);
}

#[cfg(unix)]
#[test]
fn a_failed_write_leaves_the_previous_list_unchanged() {
    use std::os::unix::fs::PermissionsExt;

    let dir = WorkDir::new("failed_write");
    dir.run(&["list", "add", "milk"]);
    // A read-only directory makes it impossible to create a replacement file.
    fs::set_permissions(&dir.path, fs::Permissions::from_mode(0o555)).expect("chmod");
    let writable = fs::File::create(dir.path.join("probe")).is_ok();

    let output = dir.run(&["list", "add", "eggs"]);

    fs::set_permissions(&dir.path, fs::Permissions::from_mode(0o755)).expect("chmod");
    if writable {
        return; // permissions are not enforced (for example when running as root)
    }
    assert!(!output.status.success(), "the write cannot succeed");
    assert_eq!(dir.read_json(), strings(&["milk"]));
}

#[cfg(unix)]
#[test]
fn a_write_keeps_the_permissions_of_the_existing_list() {
    use std::os::unix::fs::PermissionsExt;

    let dir = WorkDir::new("keeps_permissions");
    dir.run(&["list", "add", "milk"]);
    fs::set_permissions(dir.file(), fs::Permissions::from_mode(0o600)).expect("chmod");

    for args in [&["list", "add", "eggs"][..], &["list", "clear"][..]] {
        let output = dir.run(args);
        assert!(output.status.success(), "stderr: {}", stderr(&output));
        let mode = fs::metadata(dir.file())
            .expect("metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "after {args:?}");
    }
}

#[cfg(unix)]
#[test]
fn a_new_list_is_not_readable_by_other_users() {
    use std::os::unix::fs::PermissionsExt;

    let dir = WorkDir::new("new_list_permissions");
    let output = dir.run(&["list", "add", "milk"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let mode = fs::metadata(dir.file())
        .expect("metadata")
        .permissions()
        .mode();
    assert_eq!(mode & 0o077, 0, "mode {:o}", mode & 0o777);
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
        assert_eq!(dir.read_json(), strings(&["milk"]));
    }
}
