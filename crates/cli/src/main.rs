mod safety;

use clap::Parser;
use jst_shared::{TranslateRequest, TranslateResponse};
use std::io::{self, Write};
use std::process::Command;

const API_URL: &str = "https://jst-server.fly.dev/translate";

#[derive(Parser, Debug)]
#[command(
    name = "jst",
    version,
    about = "Run shell commands from natural-language requests"
)]
struct Cli {
    /// Skip all safety confirmations
    #[arg(long)]
    yolo: bool,

    /// What you want to do, in plain English
    #[arg(required = true, num_args = 1.., trailing_var_arg = true, allow_hyphen_values = true)]
    prompt: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();
    let input = cli.prompt.join(" ");
    let response = translate(&input).await?;
    let command = clean_command(&response.command);

    if command.is_empty() || command.starts_with("# unable to translate") {
        return Err("unable to translate request".into());
    }

    println!("→ {command}");

    let local_warning = safety::warning_for_command(&command);
    let model_warning = response.model_warning();
    if !cli.yolo && (local_warning.is_some() || model_warning.is_some()) {
        if let Some(explanation) = response.explanation.as_deref() {
            eprintln!("\n{explanation}");
        }
        eprintln!(
            "⚠ {}",
            local_warning
                .or(model_warning)
                .unwrap_or("This command may have side effects.")
        );
        if !confirm()? {
            eprintln!("Aborted.");
            return Ok(());
        }
    }

    execute_command(&command)
}

async fn translate(
    input: &str,
) -> Result<TranslateResponse, Box<dyn std::error::Error + Send + Sync>> {
    let request = TranslateRequest {
        input: input.to_string(),
        context: None,
        os: Some(std::env::consts::OS.to_string()),
        shell: std::env::var("SHELL").ok(),
    };

    let response = reqwest::Client::new()
        .post(API_URL)
        .json(&request)
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;

    if !status.is_success() {
        return Err(format!("translation service returned {status}: {body}").into());
    }

    Ok(serde_json::from_str(&body)?)
}

fn clean_command(command: &str) -> String {
    let trimmed = command.trim();
    if !trimmed.starts_with("```") || !trimmed.ends_with("```") {
        return trimmed.to_string();
    }

    let inner = &trimmed[3..trimmed.len() - 3];
    let inner = inner.strip_prefix("bash\n").unwrap_or(inner);
    let inner = inner.strip_prefix("sh\n").unwrap_or(inner);
    inner.trim().to_string()
}

fn confirm() -> io::Result<bool> {
    eprint!("Run it? [y/N] ");
    io::stderr().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn execute_command(command: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = Command::new(shell).arg("-c").arg(command).exec();
        Err(format!("failed to execute command: {error}").into())
    }

    #[cfg(not(unix))]
    {
        let status = Command::new(shell).arg("-c").arg(command).status()?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("command exited with {status}").into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{clean_command, Cli};
    use clap::Parser;

    #[test]
    fn strips_markdown_fences() {
        assert_eq!(clean_command("```bash\npwd\n```"), "pwd");
    }

    #[test]
    fn trims_plain_commands() {
        assert_eq!(clean_command("  pwd\n"), "pwd");
    }

    #[test]
    fn joins_unquoted_prompt_arguments() {
        let cli = Cli::try_parse_from([
            "jst",
            "find",
            "all",
            "files",
            "bigger",
            "than",
            "500",
            "mb",
            "in",
            "~/downloads",
        ])
        .expect("valid arguments");

        assert_eq!(
            cli.prompt.join(" "),
            "find all files bigger than 500 mb in ~/downloads"
        );
        assert!(!cli.yolo);
    }

    #[test]
    fn accepts_yolo_before_prompt() {
        let cli = Cli::try_parse_from(["jst", "--yolo", "remove", "build", "files"])
            .expect("valid arguments");
        assert!(cli.yolo);
    }

    #[test]
    fn rejects_missing_prompt() {
        assert!(Cli::try_parse_from(["jst"]).is_err());
    }
}
