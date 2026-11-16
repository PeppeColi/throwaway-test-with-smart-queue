mod catalog;

use std::process::ExitCode;

const USAGE: &str = "usage: ricette recipes list\n       ricette recipes search <ingredient>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();

    match args.as_slice() {
        ["recipes", "list"] => {
            for recipe in catalog::CATALOG {
                println!("{}", recipe.name);
            }
            ExitCode::SUCCESS
        }
        ["recipes", "search", term] => {
            let term = term.trim();
            if term.is_empty() {
                return usage_error();
            }
            let matches = catalog::recipes_with_ingredient(term);
            if matches.is_empty() {
                println!("No recipes match \"{term}\".");
            }
            for recipe in matches {
                println!("{}", recipe.name);
            }
            ExitCode::SUCCESS
        }
        _ => usage_error(),
    }
}

fn usage_error() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(2)
}
