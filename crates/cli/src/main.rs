mod installation;
mod safety;

use clap::Parser;
use jst_shared::{TranslateRequest, TranslateResponse};
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

const DEFAULT_API_URL: &str = "https://jst-server.fly.dev/translate";
const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const INSTALLATION_ID_HEADER: &str = "x-jst-installation-id";
const CONFIRMATION_WIDTH: usize = 88;

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

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();
    let input = cli.prompt.join(" ");
    let response = translate(&input).await?;
    let command = clean_command(&response.command);

    if command.is_empty() || command.starts_with("# unable to translate") {
        return Err("unable to translate request".into());
    }
    if contains_unsafe_terminal_character(&command) {
        return Err("generated command contains unsafe terminal characters".into());
    }

    println!("→ {command}");
    io::stdout().flush()?;

    let local_warnings = safety::warnings_for_command(&command);
    let model_warnings = response.model_warnings();
    if should_confirm(cli.yolo, &local_warnings, &model_warnings) {
        if !response.explanation.is_empty() {
            let explanation = terminal_safe(&response.explanation);
            eprintln!("\n{}", indent_wrapped(&explanation, CONFIRMATION_WIDTH));
        }
        let color = should_use_color();
        eprintln!();
        for warning in local_warnings.iter().chain(&model_warnings) {
            eprintln!("{}", format_warning(warning, CONFIRMATION_WIDTH, color));
        }
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
        os: Some(std::env::consts::OS.to_string()),
        shell: shell_name(),
    };
    let api_url = std::env::var("JST_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string());
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(30))
        .build()?;
    let installation_id = installation::installation_id()?;

    let response = client
        .post(api_url)
        .header(INSTALLATION_ID_HEADER, installation_id)
        .json(&request)
        .send()
        .await?;
    let status = response.status();
    let body = read_limited_body(response, MAX_RESPONSE_BYTES).await?;

    if !status.is_success() {
        return Err(format!("translation service returned {status}").into());
    }

    Ok(serde_json::from_str(&body)?)
}

fn clean_command(command: &str) -> String {
    let trimmed = command.trim();
    if trimmed == "```" {
        return String::new();
    }

    let Some(inner) = trimmed
        .strip_prefix("```")
        .and_then(|inner| inner.strip_suffix("```"))
    else {
        return trimmed.to_string();
    };

    let normalized = inner.replace("\r\n", "\n");
    let inner = normalized
        .split_once('\n')
        .filter(|(language, _)| {
            matches!(
                language.trim().to_ascii_lowercase().as_str(),
                "bash" | "sh" | "shell" | "zsh"
            )
        })
        .map_or(normalized.as_str(), |(_, command)| command);
    inner.trim().to_string()
}

fn shell_name() -> Option<String> {
    std::env::var("SHELL").ok().and_then(|shell| {
        Path::new(&shell)
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
    })
}

fn should_confirm(yolo: bool, local_warnings: &[&str], model_warnings: &[&str]) -> bool {
    !yolo && (!local_warnings.is_empty() || !model_warnings.is_empty())
}

fn contains_unsafe_terminal_character(value: &str) -> bool {
    value.chars().any(is_unsafe_terminal_character)
}

fn is_unsafe_terminal_character(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
}

fn terminal_safe(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if is_unsafe_terminal_character(character) {
            escaped.extend(character.escape_default());
        } else {
            escaped.push(character);
        }
    }
    escaped
}

fn should_use_color() -> bool {
    io::stderr().is_terminal()
        && std::env::var_os("NO_COLOR").is_none()
        && std::env::var("TERM").map_or(true, |term| term != "dumb")
}

fn indent_wrapped(value: &str, width: usize) -> String {
    wrap_text(value, width.saturating_sub(2))
        .into_iter()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_warning(value: &str, width: usize, color: bool) -> String {
    let wrapped = wrap_text(value, width.saturating_sub(2));
    let warning = wrapped
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                format!("⚠ {line}")
            } else {
                format!("  {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    if color {
        format!("\x1b[1;31m{warning}\x1b[0m")
    } else {
        warning
    }
}

fn wrap_text(value: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in value.split_whitespace() {
        let separator = usize::from(!current.is_empty());
        if !current.is_empty() && current.chars().count() + separator + word.chars().count() > width
        {
            lines.push(current);
            current = String::new();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

async fn read_limited_body(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err("translation response exceeded size limit".into());
    }

    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len() + chunk.len() > limit {
            return Err("translation response exceeded size limit".into());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8(body)?)
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
    use super::{
        clean_command, contains_unsafe_terminal_character, format_warning, indent_wrapped,
        should_confirm, terminal_safe, Cli,
    };
    use clap::Parser;

    #[test]
    fn strips_markdown_fences() {
        assert_eq!(clean_command("```bash\npwd\n```"), "pwd");
        assert_eq!(clean_command("```zsh\npwd\n```"), "pwd");
        assert_eq!(clean_command("```Bash\r\npwd\r\n```"), "pwd");
        assert_eq!(clean_command("```"), "");
    }

    #[test]
    fn trims_plain_commands() {
        assert_eq!(clean_command("  pwd\n"), "pwd");
    }

    #[test]
    fn wraps_and_indents_explanations() {
        let formatted = indent_wrapped(
            "This command finds all files within the specified directory and deletes them.",
            32,
        );

        assert_eq!(
            formatted,
            "  This command finds all files\n  within the specified directory\n  and deletes them."
        );
        assert!(formatted.lines().all(|line| line.chars().count() <= 32));
    }

    #[test]
    fn formats_warnings_with_optional_color() {
        let plain = "⚠ This warning describes\n  a destructive command";

        assert_eq!(
            format_warning("This warning describes a destructive command", 24, false),
            plain
        );
        assert_eq!(
            format_warning("This warning describes a destructive command", 24, true),
            format!("\x1b[1;31m{plain}\x1b[0m")
        );
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

    #[test]
    fn confirmation_policy_combines_both_warning_sources() {
        assert!(!should_confirm(false, &[], &[]));
        assert!(should_confirm(false, &["local"], &[]));
        assert!(should_confirm(false, &[], &["model"]));
        assert!(!should_confirm(true, &["local"], &["model"]));
    }

    #[test]
    fn rejects_terminal_spoofing_characters() {
        assert!(contains_unsafe_terminal_character("echo ok\rmalicious"));
        assert!(contains_unsafe_terminal_character("echo \u{1b}[2J"));
        assert!(contains_unsafe_terminal_character("echo \u{202e}txt"));
        assert!(!contains_unsafe_terminal_character("echo safe"));
        assert_eq!(terminal_safe("line\nnext\u{1b}"), "line\\nnext\\u{1b}");
    }
}
