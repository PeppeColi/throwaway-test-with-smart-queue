//! Command-line tests of the recipe catalog: they run the compiled `ricette`
//! binary and assert stdout, stderr and the exit status.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

const ALL_RECIPES: &[&str] = &[
    "Spaghetti al pomodoro",
    "Omelette",
    "Pancakes",
    "Insalata caprese",
    "Minestrone",
];

/// An empty, freshly created directory: the commands must work in it without
/// any catalog file.
fn empty_working_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ricette-tests-{}-{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create the empty working directory");
    dir
}

fn run(test_name: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ricette"))
        .args(args)
        .current_dir(empty_working_dir(test_name))
        .output()
        .expect("run the ricette binary")
}

fn lines(names: &[&str]) -> String {
    names.iter().map(|name| format!("{name}\n")).collect()
}

fn assert_success_with_stdout(output: &Output, expected_stdout: &str) {
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        expected_stdout,
        "stdout"
    );
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(0), "exit status");
}

fn assert_usage_error(output: &Output) {
    assert!(
        output.stdout.is_empty(),
        "stdout should be empty, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        !output.stderr.is_empty(),
        "stderr should explain the usage error"
    );
    assert_ne!(output.status.code(), Some(0), "exit status");
}

#[test]
fn list_prints_all_recipes_in_catalog_order() {
    let output = run("list", &["recipes", "list"]);

    assert_success_with_stdout(&output, &lines(ALL_RECIPES));
}

#[test]
fn search_tomato_prints_the_three_recipes_that_use_it_in_catalog_order() {
    let output = run("tomato", &["recipes", "search", "tomato"]);

    assert_success_with_stdout(
        &output,
        &lines(&["Spaghetti al pomodoro", "Insalata caprese", "Minestrone"]),
    );
}

#[test]
fn search_eggs_prints_omelette_and_pancakes() {
    let output = run("eggs", &["recipes", "search", "eggs"]);

    assert_success_with_stdout(&output, &lines(&["Omelette", "Pancakes"]));
}

#[test]
fn search_milk_prints_omelette_and_pancakes() {
    let output = run("milk", &["recipes", "search", "milk"]);

    assert_success_with_stdout(&output, &lines(&["Omelette", "Pancakes"]));
}

#[test]
fn search_ignores_case() {
    let output = run("case", &["recipes", "search", "TOMATO"]);

    assert_success_with_stdout(
        &output,
        &lines(&["Spaghetti al pomodoro", "Insalata caprese", "Minestrone"]),
    );
}

#[test]
fn search_trims_the_search_term() {
    let output = run("trim", &["recipes", "search", " tomato "]);

    assert_success_with_stdout(
        &output,
        &lines(&["Spaghetti al pomodoro", "Insalata caprese", "Minestrone"]),
    );
}

#[test]
fn search_matches_multi_word_ingredient_names() {
    let output = run("olive-oil", &["recipes", "search", "olive oil"]);

    assert_success_with_stdout(
        &output,
        &lines(&["Spaghetti al pomodoro", "Insalata caprese"]),
    );
}

#[test]
fn search_matches_whole_ingredient_names_not_substrings() {
    let output = run("substring", &["recipes", "search", "tom"]);

    assert_success_with_stdout(&output, "No recipes match \"tom\".\n");
}

#[test]
fn search_without_a_match_prints_the_term_and_exits_zero() {
    let output = run("no-match", &["recipes", "search", "chocolate"]);

    assert_success_with_stdout(&output, "No recipes match \"chocolate\".\n");
}

#[test]
fn no_match_message_shows_the_trimmed_term_as_given_before_case_folding() {
    let output = run("no-match-case", &["recipes", "search", "  Chocolate  "]);

    assert_success_with_stdout(&output, "No recipes match \"Chocolate\".\n");
}

#[test]
fn search_without_a_term_is_a_usage_error() {
    let output = run("search-no-term", &["recipes", "search"]);

    assert_usage_error(&output);
}

#[test]
fn search_with_a_blank_term_is_a_usage_error() {
    let output = run("search-blank", &["recipes", "search", "   "]);

    assert_usage_error(&output);
}

#[test]
fn unknown_subcommand_is_a_usage_error() {
    let output = run("unknown", &["recipes", "delete", "tomato"]);

    assert_usage_error(&output);
}

#[test]
fn missing_command_is_a_usage_error() {
    let output = run("no-args", &[]);

    assert_usage_error(&output);
}
