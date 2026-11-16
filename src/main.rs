mod catalog;

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::process::ExitCode;

const SHOPPING_LIST_FILE: &str = "shopping-list.json";
const EMPTY_MESSAGE: &str = "Shopping list is empty.";
const USAGE: &str = "usage: ricette recipes list\n       ricette recipes search <ingredient>\n       ricette list <add <item> [--qty <n> --unit <u>] | show | clear>";

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

/// One line of the shopping list: an item, with a quantity and unit when it is measured.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    item: String,
    measure: Option<Measure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Measure {
    qty: u64,
    unit: String,
}

impl Entry {
    fn unit(&self) -> Option<&str> {
        self.measure.as_ref().map(|measure| measure.unit.as_str())
    }

    /// Two entries are the same when their items and units are equal; the quantity is not part of it.
    fn is_same_as(&self, other: &Entry) -> bool {
        self.item == other.item && self.unit() == other.unit()
    }
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
        ["list", "add", item, options @ ..] => add(Path::new(SHOPPING_LIST_FILE), item, options),
        ["list", "show"] => show(Path::new(SHOPPING_LIST_FILE)),
        ["list", "clear"] => clear(Path::new(SHOPPING_LIST_FILE)),
        _ => Err(CliError::Usage(format!(
            "unrecognised command: {}",
            args.join(" ")
        ))),
    }
}

fn add(path: &Path, item: &str, options: &[&str]) -> Result<(), CliError> {
    let item = item.trim();
    if item.is_empty() {
        return Err(CliError::Failure("the item must not be empty".to_string()));
    }
    let measure = parse_measure(options)?;
    let _lock = lock(path)?;
    let mut entries = load(path)?;
    combine(
        &mut entries,
        Entry {
            item: item.to_string(),
            measure,
        },
        path,
    )?;
    save(path, &entries)
}

/// Reads the `--qty <n>` and `--unit <u>` options of `add`. They are optional,
/// but they must be given together.
fn parse_measure(options: &[&str]) -> Result<Option<Measure>, CliError> {
    let mut qty: Option<&str> = None;
    let mut unit: Option<&str> = None;
    let mut remaining = options.iter();
    while let Some(&flag) = remaining.next() {
        let Some(&value) = remaining.next() else {
            return Err(CliError::Usage(format!("{flag} needs a value")));
        };
        let slot = match flag {
            "--qty" => &mut qty,
            "--unit" => &mut unit,
            _ => return Err(CliError::Usage(format!("unrecognised option: {flag}"))),
        };
        if slot.is_some() {
            return Err(CliError::Usage(format!("{flag} is given more than once")));
        }
        *slot = Some(value);
    }

    match (qty, unit) {
        (None, None) => Ok(None),
        (Some(qty), Some(unit)) => Ok(Some(Measure {
            qty: parse_qty(qty)?,
            unit: parse_unit(unit)?,
        })),
        (Some(_), None) => Err(CliError::Usage("--qty needs --unit".to_string())),
        (None, Some(_)) => Err(CliError::Usage("--unit needs --qty".to_string())),
    }
}

fn parse_qty(value: &str) -> Result<u64, CliError> {
    let digits = !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit());
    match value.parse::<u64>() {
        Ok(qty) if digits && qty >= 1 => Ok(qty),
        _ => Err(CliError::Failure(format!(
            "the quantity must be a positive integer, not {value:?}"
        ))),
    }
}

fn parse_unit(value: &str) -> Result<String, CliError> {
    let unit = value.trim();
    if unit.is_empty() {
        return Err(CliError::Failure("the unit must not be empty".to_string()));
    }
    Ok(unit.to_string())
}

fn show(path: &Path) -> Result<(), CliError> {
    let entries = load(path)?;
    if entries.is_empty() {
        println!("{EMPTY_MESSAGE}");
    } else {
        for entry in &entries {
            println!("{}", describe(entry));
        }
    }
    Ok(())
}

fn describe(entry: &Entry) -> String {
    match &entry.measure {
        Some(measure) => format!("{} {} {}", entry.item, measure.qty, measure.unit),
        None => entry.item.clone(),
    }
}

fn clear(path: &Path) -> Result<(), CliError> {
    let _lock = lock(path)?;
    load(path)?;
    save(path, &[])
}

/// Adds `entry` to `entries`. An entry equal to one already there adds its
/// quantity to it and keeps its position; any other entry is appended.
fn combine(entries: &mut Vec<Entry>, entry: Entry, path: &Path) -> Result<(), CliError> {
    let Some(index) = entries
        .iter()
        .position(|existing| existing.is_same_as(&entry))
    else {
        entries.push(entry);
        return Ok(());
    };
    if let (Some(total), Some(added)) = (entries[index].measure.as_mut(), entry.measure) {
        total.qty = total
            .qty
            .checked_add(added.qty)
            .ok_or_else(|| failure(path, "a quantity is too large"))?;
    }
    Ok(())
}

/// Reads the shopping list. A missing file is an empty list; a file that is
/// not a JSON array of entries is an error and is never rewritten. Equal
/// entries in the file are combined, keeping the first position.
fn load(path: &Path) -> Result<Vec<Entry>, CliError> {
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

    let mut entries = Vec::new();
    for value in values {
        let entry = entry_from_json(value)
            .ok_or_else(|| failure(path, "it does not contain only valid entries"))?;
        combine(&mut entries, entry, path)?;
    }
    Ok(entries)
}

/// Takes an exclusive cross-process lock on a sidecar file. It is held until
/// the returned file is dropped, so a read-modify-write cycle cannot interleave
/// with another process.
fn lock(path: &Path) -> Result<File, CliError> {
    let mut name = path.as_os_str().to_owned();
    name.push(".lock");
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&name)
        .map_err(|error| failure(path, &error.to_string()))?;
    file.lock()
        .map_err(|error| failure(path, &error.to_string()))?;
    Ok(file)
}

/// Reads one entry of the file: a plain string (the previous format), or an
/// object with an `item` and, for a measured entry, a positive integer `qty`
/// and a non-empty `unit`. Returns `None` when the value is none of these.
fn entry_from_json(value: serde_json::Value) -> Option<Entry> {
    match value {
        serde_json::Value::String(item) => Some(Entry {
            item: item.trim().to_string(),
            measure: None,
        }),
        serde_json::Value::Object(fields) => {
            let item = fields.get("item")?.as_str()?.trim().to_string();
            let measure = match (fields.get("qty"), fields.get("unit")) {
                (None, None) => None,
                (Some(qty), Some(unit)) => {
                    let qty = qty.as_u64().filter(|qty| *qty >= 1)?;
                    let unit = unit.as_str()?.trim();
                    if unit.is_empty() {
                        return None;
                    }
                    Some(Measure {
                        qty,
                        unit: unit.to_string(),
                    })
                }
                _ => return None,
            };
            Some(Entry { item, measure })
        }
        _ => None,
    }
}

/// Writes the list in the current format: an array of objects with `item`,
/// and `qty` and `unit` when the entry is measured. It writes to a new temporary
/// file next to `path` and renames it over `path` only when the write has
/// succeeded, so a failure keeps the old list. The replacement keeps the
/// permissions of the old list; a new list is readable and writable only by its
/// owner.
fn save(path: &Path, entries: &[Entry]) -> Result<(), CliError> {
    let values: Vec<serde_json::Value> = entries.iter().map(entry_to_json).collect();
    let text = serde_json::to_string_pretty(&values)
        .map_err(|error| CliError::Failure(error.to_string()))?;
    let mut name = path.as_os_str().to_owned();
    name.push(format!(".{}.tmp", std::process::id()));
    let temporary = Path::new(&name);

    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        // Only the owner can read the replacement while it is written.
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let mut file = options.open(temporary)?;
        // The replacement keeps the permissions of the list it replaces.
        match fs::metadata(path) {
            Ok(existing) => file.set_permissions(existing.permissions())?,
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        file.write_all(format!("{text}\n").as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(temporary, path)
    })();
    result.map_err(|error| {
        let _ = fs::remove_file(temporary);
        failure(path, &error.to_string())
    })
}

fn entry_to_json(entry: &Entry) -> serde_json::Value {
    let mut fields = serde_json::Map::new();
    fields.insert("item".to_string(), entry.item.clone().into());
    if let Some(measure) = &entry.measure {
        fields.insert("qty".to_string(), measure.qty.into());
        fields.insert("unit".to_string(), measure.unit.clone().into());
    }
    serde_json::Value::Object(fields)
}

fn failure(path: &Path, reason: &str) -> CliError {
    CliError::Failure(format!("cannot use {}: {reason}", path.display()))
}
