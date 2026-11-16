mod catalog;

use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::process::ExitCode;

const SHOPPING_LIST_FILE: &str = "shopping-list.json";
const EMPTY_MESSAGE: &str = "Shopping list is empty.";
const USAGE: &str = "usage: ricette recipes list\n       ricette recipes search <ingredient>\n       ricette list <add <item> | show | clear>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();

    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliError::Usage(message)) => {
            eprintln!("ricette: {message}");
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
        Err(CliError::Failure(message)) => {
            eprintln!("ricette: {message}");
            ExitCode::FAILURE
        }
    }
}

enum CliError {
    Usage(String),
    Failure(String),
}

fn run(args: &[&str]) -> Result<(), CliError> {
    match args {
        ["recipes", "list"] => {
            for recipe in catalog::CATALOG {
                println!("{}", recipe.name);
            }
            Ok(())
        }
        ["recipes", "search", term] => {
            let term = term.trim();
            if term.is_empty() {
                return Err(CliError::Usage(
                    "the ingredient must not be empty".to_string(),
                ));
            }
            let matches = catalog::recipes_with_ingredient(term);
            if matches.is_empty() {
                println!("No recipes match \"{term}\".");
            }
            for recipe in matches {
                println!("{}", recipe.name);
            }
            Ok(())
        }
        ["list", "add", item] => add(Path::new(SHOPPING_LIST_FILE), item),
        ["list", "show"] => show(Path::new(SHOPPING_LIST_FILE)),
        ["list", "clear"] => clear(Path::new(SHOPPING_LIST_FILE)),
        _ => Err(CliError::Usage(format!(
            "unrecognised command: {}",
            args.join(" ")
        ))),
    }
}

fn add(path: &Path, item: &str) -> Result<(), CliError> {
    let item = item.trim();
    if item.is_empty() {
        return Err(CliError::Failure("the item must not be empty".to_string()));
    }
    let mut items = load(path)?;
    items.push(item.to_string());
    save(path, &items)
}

fn show(path: &Path) -> Result<(), CliError> {
    let items = load(path)?;
    if items.is_empty() {
        println!("{EMPTY_MESSAGE}");
    } else {
        for item in &items {
            println!("{item}");
        }
    }
    Ok(())
}

fn clear(path: &Path) -> Result<(), CliError> {
    load(path)?;
    save(path, &[])
}

/// Reads the shopping list. A missing file is an empty list; a file that is
/// not a JSON array of strings is an error and is never rewritten.
fn load(path: &Path) -> Result<Vec<String>, CliError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(failure(path, &error.to_string())),
    };

    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| failure(path, &format!("it is not valid JSON ({error})")))?;

    let serde_json::Value::Array(values) = value else {
        return Err(failure(path, "it is not a JSON array"));
    };

    values
        .into_iter()
        .map(|value| match value {
            serde_json::Value::String(item) => Ok(item),
            _ => Err(failure(path, "it is not an array of strings")),
        })
        .collect()
}

fn save(path: &Path, items: &[String]) -> Result<(), CliError> {
    let text = serde_json::to_string_pretty(items)
        .map_err(|error| CliError::Failure(error.to_string()))?;
    fs::write(path, format!("{text}\n")).map_err(|error| failure(path, &error.to_string()))
}

fn failure(path: &Path, reason: &str) -> CliError {
    CliError::Failure(format!("cannot use {}: {reason}", path.display()))
}
