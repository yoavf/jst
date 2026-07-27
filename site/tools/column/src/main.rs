use std::{
    env, fs,
    io::{self, Read},
    process,
};

struct Config {
    separator: Option<String>,
    output_separator: String,
    files: Vec<String>,
}

fn usage() {
    println!(
        "Usage: column [OPTION]... [FILE]...\n\
         Format input into aligned columns.\n\n\
         Options:\n\
           -t, --table                 create a table\n\
           -s, --separator STRING      split columns on STRING\n\
           -o, --output-separator STR  separate output columns with STR\n\
               --help                  display this help\n\
               --version               display version"
    );
}

fn take_value(
    arguments: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<String, String> {
    arguments
        .next()
        .ok_or_else(|| format!("column: option '{option}' needs a value"))
}

fn parse_arguments() -> Result<Config, String> {
    let mut separator = None;
    let mut output_separator = "  ".to_string();
    let mut files = Vec::new();
    let mut positional_only = false;
    let mut arguments = env::args().skip(1);

    while let Some(argument) = arguments.next() {
        if positional_only {
            files.push(argument);
            continue;
        }
        match argument.as_str() {
            "--" => positional_only = true,
            "-t" | "--table" => {}
            "-s" | "--separator" => {
                separator = Some(take_value(&mut arguments, &argument)?);
            }
            "-o" | "--output-separator" => {
                output_separator = take_value(&mut arguments, &argument)?;
            }
            "--help" => {
                usage();
                process::exit(0);
            }
            "--version" => {
                println!("column (JST WASI tools) 0.1.0");
                process::exit(0);
            }
            _ if argument.starts_with("--separator=") => {
                separator = Some(argument["--separator=".len()..].to_string());
            }
            _ if argument.starts_with("--output-separator=") => {
                output_separator = argument["--output-separator=".len()..].to_string();
            }
            _ if argument.starts_with("-s") && argument.len() > 2 => {
                separator = Some(argument[2..].to_string());
            }
            _ if argument.starts_with("-o") && argument.len() > 2 => {
                output_separator = argument[2..].to_string();
            }
            _ if argument.starts_with('-') && argument != "-" => {
                return Err(format!("column: unsupported option '{argument}'"));
            }
            _ => files.push(argument),
        }
    }

    if separator.as_deref() == Some("") {
        return Err("column: separator cannot be empty".to_string());
    }
    Ok(Config {
        separator,
        output_separator,
        files,
    })
}

fn read_input(files: &[String]) -> Result<String, String> {
    if files.is_empty() {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| format!("column: stdin: {error}"))?;
        return Ok(input);
    }

    let mut input = String::new();
    for path in files {
        if path == "-" {
            io::stdin()
                .read_to_string(&mut input)
                .map_err(|error| format!("column: stdin: {error}"))?;
        } else {
            input.push_str(
                &fs::read_to_string(path).map_err(|error| format!("column: {path}: {error}"))?,
            );
        }
        if !input.ends_with('\n') {
            input.push('\n');
        }
    }
    Ok(input)
}

fn main() {
    let config = parse_arguments().unwrap_or_else(|message| {
        eprintln!("{message}");
        process::exit(1);
    });
    let input = read_input(&config.files).unwrap_or_else(|message| {
        eprintln!("{message}");
        process::exit(1);
    });

    let rows = input
        .lines()
        .map(|line| match config.separator.as_deref() {
            Some(separator) => line
                .split(separator)
                .map(str::to_string)
                .collect::<Vec<_>>(),
            None => line
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>(),
        })
        .collect::<Vec<_>>();
    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut widths = vec![0; column_count];
    for row in &rows {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.chars().count());
        }
    }

    for row in rows {
        for (index, value) in row.iter().enumerate() {
            print!("{value}");
            if index + 1 < row.len() {
                let padding = widths[index].saturating_sub(value.chars().count());
                print!(
                    "{}{output}",
                    " ".repeat(padding),
                    output = config.output_separator
                );
            }
        }
        println!();
    }
}
