use clap::Parser;

/// jst — translate natural language to shell commands
///
/// Type what you want to do in plain English and jst translates it
/// into the right shell command. By default you get an interactive UI
/// where you can review, edit, and confirm before running.
#[derive(Parser, Debug)]
#[command(name = "jst", version, about, long_about = None)]
struct Cli {
    /// Skip the interactive UI and run the translated command immediately
    #[arg(long)]
    yolo: bool,

    /// Output the translated command to stdout without executing it (for scripting)
    #[arg(long, hide = true)]
    print_command: bool,

    /// What you want to do, in plain English
    #[arg(num_args = 0..)]
    prompt: Vec<String>,
}

pub enum CliMode {
    Interactive {
        prefill: Option<String>,
    },
    PrintCommand {
        input: String,
    },
}

pub fn parse_cli_mode() -> CliMode {
    let cli = Cli::parse();
    let prompt = cli.prompt.join(" ");
    let prompt = if prompt.is_empty() { None } else { Some(prompt) };

    if cli.yolo || cli.print_command {
        match prompt {
            Some(input) => CliMode::PrintCommand { input },
            None => CliMode::Interactive { prefill: None },
        }
    } else {
        CliMode::Interactive { prefill: prompt }
    }
}
