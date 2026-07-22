mod installation;
mod safety;

use clap::{CommandFactory, Parser};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use jst_shared::{
    CommandEffects, CommandPart, CommandRevision, TranslateRequest, TranslateResponse,
};
use std::fmt;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

const DEFAULT_API_URL: &str = "https://jst-server.fly.dev/translate";
const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const INSTALLATION_ID_HEADER: &str = "x-jst-installation-id";
const CONFIRMATION_WIDTH: usize = 88;
const MAX_MANUAL_COMMAND_BYTES: usize = 2 * 1024;
const MAX_REVISION_INSTRUCTION_BYTES: usize = 512;

#[derive(Parser, Debug)]
#[command(
    name = "jst",
    version,
    about = "Turn plain English into a shell command and run it",
    after_help = "Examples:\n  jst show the 10 largest files here\n  jst --dry find files larger than 500 MB\n  jst -i remove stopped Docker containers\n\nUse --dry to preview or -i to review before running."
)]
struct Cli {
    /// Skip all safety confirmations
    #[arg(long, conflicts_with_all = ["interactive", "dry"])]
    yolo: bool,

    /// Approve, explain, revise, or manually edit before running
    #[arg(short, long)]
    interactive: bool,

    /// Show the generated command without running it
    #[arg(long, conflicts_with = "interactive")]
    dry: bool,

    /// What you want to do, in plain English
    #[arg(required = true, num_args = 1.., trailing_var_arg = true)]
    prompt: Vec<String>,
}

#[derive(Debug)]
enum JstError {
    Network,
    Server(u16),
    LlmProvider,
    Deserialization,
    Other(String),
}

impl fmt::Display for JstError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            JstError::Network => write!(
                f,
                "couldn't reach the jst server — check your connection and try again"
            ),
            JstError::Server(429) => write!(
                f,
                "rate limit reached — slow down, or run your own jst server"
            ),
            JstError::LlmProvider => write!(f, "trouble reaching the LLM; try again in a moment"),
            JstError::Server(code) => write!(
                f,
                "the jst server is having trouble (HTTP {code}); try again in a moment"
            ),
            JstError::Deserialization => {
                write!(f, "got an unexpected response from the jst server")
            }
            JstError::Other(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for JstError {}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(error) = run().await {
        let color = should_use_color();
        eprintln!("{}", format_error(&error, color));
        std::process::exit(1);
    }
}

async fn run() -> Result<(), JstError> {
    if std::env::args_os().len() == 1 {
        let mut command = Cli::command();
        let mut stdout = io::stdout().lock();
        command
            .write_help(&mut stdout)
            .map_err(|error| JstError::Other(format!("{error}")))?;
        writeln!(stdout).map_err(|error| JstError::Other(format!("{error}")))?;
        return Ok(());
    }

    let cli = Cli::parse();
    let input = cli.prompt.join(" ");
    let use_color = should_use_color();
    let interactive = cli.interactive;

    if interactive && !io::stdin().is_terminal() {
        return Err(JstError::Other(
            "interactive mode requires an interactive terminal".to_string(),
        ));
    }

    let response = translate_with_spinner(&input, interactive, None, use_color).await?;
    if interactive {
        return review_command(&input, response, false, use_color).await;
    }

    let command = validated_command(&response)?;
    print_command(&command, use_color, ProposalKind::Initial)?;
    if cli.dry {
        return Ok(());
    }

    let width = terminal_width();
    let local_warnings = safety::warnings_for_command(&command);
    let model_warnings = response.model_warnings();
    if should_confirm(cli.yolo, &local_warnings, &model_warnings) {
        if !response.explanation.is_empty() {
            let explanation = terminal_safe(&response.explanation);
            eprintln!("\n{}", indent_wrapped(&explanation, width));
        }
        print_warnings(&local_warnings, &model_warnings, width, use_color);
        if !confirm()? {
            eprintln!("Aborted.");
            return Ok(());
        }
    }

    execute_command(&command)
}

async fn translate_with_spinner(
    input: &str,
    explain: bool,
    revision: Option<CommandRevision>,
    use_color: bool,
) -> Result<TranslateResponse, JstError> {
    let spinner = if use_color {
        let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(async move {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut i = 0;
            loop {
                if tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(80)) => false,
                    _ = &mut stop_rx => true,
                } {
                    break;
                }
                eprint!("\r{} ", frames[i % frames.len()]);
                io::stderr().flush().ok();
                i += 1;
            }
        });
        Some((handle, stop_tx))
    } else {
        None
    };

    let result = translate(input, explain, revision).await;

    if let Some((handle, stop_tx)) = spinner {
        let _ = stop_tx.send(());
        let _ = handle.await;
        eprint!("\r\x1b[K");
        io::stderr().flush().ok();
    }

    result
}

async fn translate(
    input: &str,
    explain: bool,
    revision: Option<CommandRevision>,
) -> Result<TranslateResponse, JstError> {
    let request = TranslateRequest {
        input: input.to_string(),
        os: Some(std::env::consts::OS.to_string()),
        shell: shell_name(),
        explain,
        revision,
    };
    let api_url = std::env::var("JST_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string());
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| JstError::Network)?;
    let installation_id =
        installation::installation_id().map_err(|error| JstError::Other(format!("{error}")))?;

    let response = client
        .post(api_url)
        .header(INSTALLATION_ID_HEADER, installation_id)
        .json(&request)
        .send()
        .await
        .map_err(|_| JstError::Network)?;
    let status = response.status();
    let body = read_limited_body(response, MAX_RESPONSE_BYTES).await?;

    if status == reqwest::StatusCode::BAD_GATEWAY {
        return Err(JstError::LlmProvider);
    }
    if !status.is_success() {
        return Err(JstError::Server(status.as_u16()));
    }

    serde_json::from_str(&body).map_err(|_| JstError::Deserialization)
}

async fn review_command(
    input: &str,
    mut response: TranslateResponse,
    mut explanation_visible: bool,
    use_color: bool,
) -> Result<(), JstError> {
    let width = terminal_width();
    let mut source_context = input.to_string();
    let mut proposal_kind = ProposalKind::Initial;

    'proposal: loop {
        let command = validated_command(&response)?;
        print_command(&command, use_color, proposal_kind)?;

        if explanation_visible {
            eprintln!(
                "\n{}",
                format_proposal_explanation(
                    &response,
                    &command,
                    &source_context,
                    proposal_kind,
                    width,
                    use_color,
                )
            );
        }

        let local_warnings = safety::warnings_for_command(&command);
        let model_warnings = if matches!(proposal_kind, ProposalKind::Manual) {
            Vec::new()
        } else {
            response.model_warnings()
        };
        print_warnings(&local_warnings, &model_warnings, width, use_color);

        loop {
            match review_action(use_color)? {
                ReviewAction::Run => return execute_command(&command),
                ReviewAction::Abort => {
                    eprintln!("Aborted.");
                    return Ok(());
                }
                ReviewAction::Why => {
                    if explanation_visible {
                        if use_color {
                            eprintln!("\n\x1b[2mExplanation is shown above.\x1b[0m");
                        } else {
                            eprintln!("\nExplanation is shown above.");
                        }
                    } else {
                        eprintln!(
                            "\n{}",
                            format_proposal_explanation(
                                &response,
                                &command,
                                &source_context,
                                proposal_kind,
                                width,
                                use_color,
                            )
                        );
                        explanation_visible = true;
                    }
                }
                ReviewAction::AskAi => {
                    let instruction = match revision_instruction(use_color)? {
                        InteractiveInput::Submitted(instruction) => instruction,
                        InteractiveInput::Cancelled => continue,
                        InteractiveInput::Interrupted => {
                            eprintln!("Aborted.");
                            return Ok(());
                        }
                    };
                    let revision = CommandRevision {
                        command,
                        instruction: instruction.clone(),
                    };
                    response =
                        translate_with_spinner(input, true, Some(revision), use_color).await?;
                    source_context = format!("{input} {instruction}");
                    proposal_kind = ProposalKind::Revised;
                    eprintln!();
                    continue 'proposal;
                }
                ReviewAction::Edit => {
                    let replacement = match manual_command(&command, use_color)? {
                        InteractiveInput::Submitted(replacement) => replacement,
                        InteractiveInput::Cancelled => continue,
                        InteractiveInput::Interrupted => {
                            eprintln!("Aborted.");
                            return Ok(());
                        }
                    };
                    if replacement == command {
                        return execute_command(&command);
                    }
                    let local_warnings = safety::warnings_for_command(&replacement);
                    if local_warnings.is_empty() {
                        eprintln!();
                        print_command(&replacement, use_color, ProposalKind::Manual)?;
                        return execute_command(&replacement);
                    }

                    response = TranslateResponse {
                        command: replacement,
                        effects: CommandEffects::default(),
                        matches_request: true,
                        explanation: String::new(),
                        parts: Vec::new(),
                    };
                    proposal_kind = ProposalKind::Manual;
                    explanation_visible = false;
                    eprintln!();
                    continue 'proposal;
                }
            }
        }
    }
}

fn validated_command(response: &TranslateResponse) -> Result<String, JstError> {
    let command = clean_command(&response.command);
    if command.is_empty() || command.starts_with("# unable to translate") {
        return Err(JstError::Other("unable to translate request".to_string()));
    }
    if contains_unsafe_terminal_character(&command) {
        return Err(JstError::Other(
            "generated command contains unsafe terminal characters".to_string(),
        ));
    }
    Ok(command)
}

#[derive(Clone, Copy)]
enum ProposalKind {
    Initial,
    Revised,
    Manual,
}

fn print_command(command: &str, color: bool, kind: ProposalKind) -> Result<(), JstError> {
    let marker = match kind {
        ProposalKind::Initial => None,
        ProposalKind::Revised => Some(("✦ AI revision", "33")),
        ProposalKind::Manual => Some(("✎ your edit", "35")),
    };
    if let Some((label, ansi)) = marker {
        if color {
            println!("\x1b[1;{ansi}m{label}\x1b[0m");
        } else {
            println!("{label}");
        }
    }
    if color {
        println!("\x1b[1;36m→\x1b[0m \x1b[1m{command}\x1b[0m");
    } else {
        println!("→ {command}");
    }
    io::stdout()
        .flush()
        .map_err(|error| JstError::Other(format!("{error}")))
}

fn print_warnings(local_warnings: &[&str], model_warnings: &[&str], width: usize, color: bool) {
    if local_warnings.is_empty() && model_warnings.is_empty() {
        return;
    }
    eprintln!();
    for warning in local_warnings.iter().chain(model_warnings) {
        eprintln!("{}", format_warning(warning, width, color));
    }
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

#[derive(Debug, PartialEq, Eq)]
enum ReviewAction {
    Run,
    Abort,
    Why,
    AskAi,
    Edit,
}

fn parse_review_action(value: &str) -> Option<ReviewAction> {
    match value.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" | "r" | "run" => Some(ReviewAction::Run),
        "" | "n" | "no" | "q" | "quit" => Some(ReviewAction::Abort),
        "w" | "why" | "explain" => Some(ReviewAction::Why),
        "a" | "ask" | "ai" | "change" => Some(ReviewAction::AskAi),
        "e" | "edit" | "manual" => Some(ReviewAction::Edit),
        _ => None,
    }
}

fn format_review_prompt(color: bool) -> String {
    if color {
        concat!(
            "\x1b[1mRun it?\x1b[0m  ",
            "\x1b[32m[y]es\x1b[0m  ",
            "\x1b[31m[n]o\x1b[0m  ",
            "\x1b[34m[w]hy\x1b[0m  ",
            "\x1b[33m[a]sk AI\x1b[0m  ",
            "\x1b[35m[e]dit\x1b[0m  ",
            "\x1b[1;36m›\x1b[0m"
        )
        .to_string()
    } else {
        "Run it?  [y]es  [n]o  [w]hy  [a]sk AI  [e]dit  ›".to_string()
    }
}

fn review_action(color: bool) -> Result<ReviewAction, JstError> {
    loop {
        let prompt = format!("\n{} ", format_review_prompt(color));
        match read_interactive_input(&prompt, 16, "")? {
            InteractiveInput::Submitted(answer) => {
                if let Some(action) = parse_review_action(&answer) {
                    return Ok(action);
                }
                eprintln!("Choose y, n, w, a, or e.");
            }
            InteractiveInput::Cancelled => continue,
            InteractiveInput::Interrupted => return Ok(ReviewAction::Abort),
        }
    }
}

fn revision_instruction(color: bool) -> Result<InteractiveInput, JstError> {
    let prompt = if color {
        "\n\x1b[1;33m✦ What should AI change?\x1b[0m "
    } else {
        "\n✦ What should AI change? "
    };
    normalize_draft(
        read_interactive_input(prompt, MAX_REVISION_INSTRUCTION_BYTES, "")?,
        "No changes requested.",
    )
}

fn manual_command(command: &str, color: bool) -> Result<InteractiveInput, JstError> {
    let prompt = format_edit_prompt(color);
    let result = normalize_draft(
        read_interactive_input(&prompt, MAX_MANUAL_COMMAND_BYTES, command)?,
        "Command unchanged.",
    )?;
    if let InteractiveInput::Submitted(command) = &result {
        if contains_unsafe_terminal_character(command) {
            eprintln!("The replacement contains unsafe terminal characters.");
            return Ok(InteractiveInput::Cancelled);
        }
    }
    Ok(result)
}

fn format_edit_prompt(color: bool) -> String {
    if color {
        "\n\x1b[1;35mEdit the command before running it\x1b[0m\n\
         \x1b[2mesc to cancel · enter to run\x1b[0m\n\
         \x1b[1;35m›\x1b[0m "
            .to_string()
    } else {
        "\nEdit the command before running it\n\
         esc to cancel · enter to run\n\
         › "
        .to_string()
    }
}

#[derive(Debug, PartialEq, Eq)]
enum InteractiveInput {
    Submitted(String),
    Cancelled,
    Interrupted,
}

fn normalize_draft(
    input: InteractiveInput,
    empty_message: &str,
) -> Result<InteractiveInput, JstError> {
    match input {
        InteractiveInput::Submitted(value) => {
            let value = value.trim();
            if value.is_empty() {
                eprintln!("{empty_message}");
                Ok(InteractiveInput::Cancelled)
            } else {
                Ok(InteractiveInput::Submitted(value.to_string()))
            }
        }
        InteractiveInput::Cancelled => {
            eprintln!("↩ Back to actions.");
            Ok(InteractiveInput::Cancelled)
        }
        InteractiveInput::Interrupted => Ok(InteractiveInput::Interrupted),
    }
}

struct RawModeGuard;

impl RawModeGuard {
    fn enable() -> Result<Self, JstError> {
        enable_raw_mode().map_err(|error| JstError::Other(format!("{error}")))?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

fn read_interactive_input(
    prompt: &str,
    max_bytes: usize,
    initial: &str,
) -> Result<InteractiveInput, JstError> {
    if initial.len() > max_bytes {
        return Err(JstError::Other(
            "initial interactive input exceeds its size limit".to_string(),
        ));
    }
    eprint!("{prompt}");
    eprint!("{initial}");
    io::stderr()
        .flush()
        .map_err(|error| JstError::Other(format!("{error}")))?;
    let raw_mode = RawModeGuard::enable()?;
    let mut value = initial.to_string();
    let mut cursor = value.len();

    loop {
        let event = event::read().map_err(|error| JstError::Other(format!("{error}")))?;
        match event {
            Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
                match key.code {
                    KeyCode::Esc => {
                        eprint!("\r\x1b[2K");
                        drop(raw_mode);
                        eprint!("\r\n");
                        io::stderr().flush().ok();
                        return Ok(InteractiveInput::Cancelled);
                    }
                    KeyCode::Enter => {
                        drop(raw_mode);
                        eprint!("\r\n");
                        io::stderr().flush().ok();
                        return Ok(InteractiveInput::Submitted(value));
                    }
                    KeyCode::Backspace => {
                        if let Some(previous) = previous_char_start(&value, cursor) {
                            value.replace_range(previous..cursor, "");
                            cursor = previous;
                            let tail = &value[cursor..];
                            eprint!("\x08{tail} ");
                            move_cursor_left(tail.chars().count() + 1);
                            io::stderr().flush().ok();
                        }
                    }
                    KeyCode::Delete => {
                        if let Some(next) = next_char_end(&value, cursor) {
                            value.replace_range(cursor..next, "");
                            let tail = &value[cursor..];
                            eprint!("{tail} ");
                            move_cursor_left(tail.chars().count() + 1);
                            io::stderr().flush().ok();
                        }
                    }
                    KeyCode::Left => {
                        if let Some(previous) = previous_char_start(&value, cursor) {
                            cursor = previous;
                            eprint!("\x1b[D");
                            io::stderr().flush().ok();
                        }
                    }
                    KeyCode::Right => {
                        if let Some(next) = next_char_end(&value, cursor) {
                            cursor = next;
                            eprint!("\x1b[C");
                            io::stderr().flush().ok();
                        }
                    }
                    KeyCode::Home => {
                        let distance = value[..cursor].chars().count();
                        cursor = 0;
                        move_cursor_left(distance);
                        io::stderr().flush().ok();
                    }
                    KeyCode::End => {
                        let distance = value[cursor..].chars().count();
                        cursor = value.len();
                        move_cursor_right(distance);
                        io::stderr().flush().ok();
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        drop(raw_mode);
                        eprint!("^C\r\n");
                        io::stderr().flush().ok();
                        return Ok(InteractiveInput::Interrupted);
                    }
                    KeyCode::Char('d')
                        if key.modifiers.contains(KeyModifiers::CONTROL) && value.is_empty() =>
                    {
                        drop(raw_mode);
                        eprint!("\r\n");
                        io::stderr().flush().ok();
                        return Ok(InteractiveInput::Interrupted);
                    }
                    KeyCode::Char(character)
                        if !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        if value.len() + character.len_utf8() <= max_bytes
                            && !character.is_control()
                        {
                            value.insert(cursor, character);
                            cursor += character.len_utf8();
                            let tail = &value[cursor..];
                            eprint!("{character}{tail}");
                            move_cursor_left(tail.chars().count());
                            io::stderr().flush().ok();
                        } else {
                            eprint!("\x07");
                            io::stderr().flush().ok();
                        }
                    }
                    _ => {}
                }
            }
            Event::Paste(pasted) => {
                let mut inserted = String::new();
                for character in pasted.chars().filter(|character| !character.is_control()) {
                    if value.len() + inserted.len() + character.len_utf8() > max_bytes {
                        eprint!("\x07");
                        break;
                    }
                    inserted.push(character);
                }
                if !inserted.is_empty() {
                    value.insert_str(cursor, &inserted);
                    cursor += inserted.len();
                    let tail = &value[cursor..];
                    eprint!("{inserted}{tail}");
                    move_cursor_left(tail.chars().count());
                }
                io::stderr().flush().ok();
            }
            _ => {}
        }
    }
}

fn previous_char_start(value: &str, cursor: usize) -> Option<usize> {
    value[..cursor]
        .char_indices()
        .next_back()
        .map(|(index, _)| index)
}

fn next_char_end(value: &str, cursor: usize) -> Option<usize> {
    value[cursor..]
        .chars()
        .next()
        .map(|character| cursor + character.len_utf8())
}

fn move_cursor_left(distance: usize) {
    if distance > 0 {
        eprint!("\x1b[{distance}D");
    }
}

fn move_cursor_right(distance: usize) {
    if distance > 0 {
        eprint!("\x1b[{distance}C");
    }
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

fn terminal_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(CONFIRMATION_WIDTH)
        .clamp(44, 100)
}

fn format_detailed_explanation(
    response: &TranslateResponse,
    command: &str,
    input: &str,
    width: usize,
    color: bool,
) -> String {
    let mut sections = Vec::new();
    if explanation_parts_are_valid(&response.parts, command) {
        sections.push(format_explanation_parts(
            &response.parts,
            input,
            width,
            color,
        ));
    } else if !response.explanation.is_empty() {
        sections.push(indent_wrapped(&terminal_safe(&response.explanation), width));
    }
    sections.push(format_effects(&response.effects, width, color));
    sections.join("\n\n")
}

fn format_proposal_explanation(
    response: &TranslateResponse,
    command: &str,
    input: &str,
    kind: ProposalKind,
    width: usize,
    color: bool,
) -> String {
    if matches!(kind, ProposalKind::Manual) {
        return indent_wrapped(
            "This command was entered manually. JST did not send it to AI; only local safety checks were applied.",
            width,
        );
    }

    format_detailed_explanation(response, command, input, width, color)
}

fn explanation_parts_are_valid(parts: &[CommandPart], command: &str) -> bool {
    if parts.is_empty() || parts.len() > 8 {
        return false;
    }

    let mut reconstructed = String::new();
    for part in parts {
        if part.fragment.trim().is_empty() || part.meaning.trim().is_empty() {
            return false;
        }
        reconstructed.push_str(&part.fragment);
    }
    reconstructed == command
}

fn format_explanation_parts(
    parts: &[CommandPart],
    input: &str,
    width: usize,
    color: bool,
) -> String {
    let normalized_input = input.to_lowercase();
    let safe_parts = parts
        .iter()
        .map(|part| {
            let fragment = terminal_safe(part.fragment.trim());
            let meaning = terminal_safe(part.meaning.trim());
            let source = part.source.trim();
            let source = if !source.is_empty() && normalized_input.contains(&source.to_lowercase())
            {
                terminal_safe(source)
            } else {
                String::new()
            };
            let description = if source.is_empty() {
                meaning
            } else {
                format!("{meaning} (“{source}”)")
            };
            (fragment, description)
        })
        .collect::<Vec<_>>();
    let label_width = safe_parts
        .iter()
        .map(|(fragment, _)| fragment.chars().count())
        .max()
        .unwrap_or(0)
        .min(26);
    let use_columns = width >= 64;
    let mut lines = Vec::new();

    for (fragment, description) in safe_parts {
        if use_columns && fragment.chars().count() <= label_width {
            let description_width = width.saturating_sub(label_width + 4).max(20);
            let wrapped = wrap_text(&description, description_width);
            let padding = " ".repeat(label_width.saturating_sub(fragment.chars().count()));
            let label = format!("{fragment}{padding}");
            let label = style_explanation_fragment(&label, color);
            for (index, line) in wrapped.into_iter().enumerate() {
                if index == 0 {
                    lines.push(format!("  {label}  {line}"));
                } else {
                    lines.push(format!("  {}  {line}", " ".repeat(label_width)));
                }
            }
        } else {
            let fragment = style_explanation_fragment(&fragment, color);
            lines.push(format!("  {fragment}"));
            lines.extend(
                wrap_text(&description, width.saturating_sub(4))
                    .into_iter()
                    .map(|line| format!("    {line}")),
            );
        }
    }

    lines.join("\n")
}

fn style_explanation_fragment(value: &str, color: bool) -> String {
    if color {
        format!("\x1b[1;34m{value}\x1b[0m")
    } else {
        value.to_string()
    }
}

fn format_effects(effects: &CommandEffects, width: usize, color: bool) -> String {
    let mut values = Vec::new();
    if effects.reads_data {
        values.push("reads local data");
    }
    if effects.modifies_data {
        values.push("modifies local data");
    }
    if effects.deletes_data {
        values.push("deletes local data");
    }
    if effects.uses_network {
        values.push("uses the network");
    }
    if effects.changes_remote_data {
        values.push("changes remote data");
    }
    if effects.changes_processes {
        values.push("changes processes");
    }
    if effects.installs_software {
        values.push("changes installed software");
    }
    if effects.uses_privilege {
        values.push("uses elevated privileges");
    }
    if effects.executes_remote_code {
        values.push("executes downloaded code");
    }

    let summary = if values.is_empty() {
        "Effects: no external effects reported.".to_string()
    } else {
        format!("Effects: {}.", values.join("; "))
    };
    let formatted = indent_wrapped(&summary, width);
    if color {
        format!("\x1b[2m{formatted}\x1b[0m")
    } else {
        formatted
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
) -> Result<String, JstError> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(JstError::Network);
    }

    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| JstError::Network)? {
        if body.len() + chunk.len() > limit {
            return Err(JstError::Network);
        }
        body.extend_from_slice(&chunk);
    }
    String::from_utf8(body).map_err(|_| JstError::Deserialization)
}

fn confirm() -> Result<bool, JstError> {
    eprint!("Run it? [y/N] ");
    io::stderr()
        .flush()
        .map_err(|error| JstError::Other(format!("{error}")))?;

    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .map_err(|error| JstError::Other(format!("{error}")))?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn execute_command(command: &str) -> Result<(), JstError> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = Command::new(shell).arg("-c").arg(command).exec();
        Err(JstError::Other(format!(
            "failed to execute command: {error}"
        )))
    }

    #[cfg(not(unix))]
    {
        let status = Command::new(shell)
            .arg("-c")
            .arg(command)
            .status()
            .map_err(|error| JstError::Other(format!("failed to execute command: {error}")))?;
        if status.success() {
            Ok(())
        } else {
            Err(JstError::Other(format!("command exited with {status}")))
        }
    }
}

fn format_error(error: &JstError, color: bool) -> String {
    let message = format!("jst: {error}");
    if color {
        format!("\x1b[1;31m{message}\x1b[0m")
    } else {
        message
    }
}

#[cfg(test)]
mod tests {
    use super::{
        clean_command, contains_unsafe_terminal_character, format_detailed_explanation,
        format_edit_prompt, format_error, format_proposal_explanation, format_review_prompt,
        format_warning, indent_wrapped, next_char_end, parse_review_action, previous_char_start,
        should_confirm, terminal_safe, Cli, JstError, ProposalKind, ReviewAction,
    };
    use clap::{error::ErrorKind, CommandFactory, Parser};
    use jst_shared::{CommandEffects, CommandPart, TranslateResponse};

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
        assert!(!cli.interactive);
        assert!(!cli.dry);
    }

    #[test]
    fn accepts_yolo_before_prompt() {
        let cli = Cli::try_parse_from(["jst", "--yolo", "remove", "build", "files"])
            .expect("valid arguments");
        assert!(cli.yolo);
    }

    #[test]
    fn accepts_dry_before_prompt() {
        let cli = Cli::try_parse_from(["jst", "--dry", "show", "current", "directory"])
            .expect("valid arguments");
        assert!(cli.dry);
    }

    #[test]
    fn accepts_interactive_before_prompt() {
        let cli = Cli::try_parse_from(["jst", "--interactive", "show", "current", "directory"])
            .expect("valid arguments");
        assert!(cli.interactive);
        assert!(
            Cli::try_parse_from(["jst", "-i", "show", "files"])
                .expect("valid short flag")
                .interactive
        );
    }

    #[test]
    fn rejects_dry_with_yolo() {
        assert!(Cli::try_parse_from(["jst", "--dry", "--yolo", "show", "files"]).is_err());
        assert!(Cli::try_parse_from(["jst", "--interactive", "--yolo", "show", "files"]).is_err());
        assert!(Cli::try_parse_from(["jst", "--interactive", "--dry", "show", "files"]).is_err());
    }

    #[test]
    fn help_includes_onboarding_examples() {
        let output = Cli::command().render_help().to_string();

        assert!(output.contains("Examples:"));
        assert!(output.contains("jst show the 10 largest files here"));
        assert!(output.contains("Use --dry to preview or -i to review before running."));
    }

    #[test]
    fn rejects_unknown_option_before_prompt() {
        let error =
            Cli::try_parse_from(["jst", "--vesrion"]).expect_err("unknown option should fail");

        assert_eq!(error.kind(), ErrorKind::UnknownArgument);
        assert!(error.to_string().contains("--help"));
    }

    #[test]
    fn accepts_option_like_values_after_prompt_begins() {
        let cli = Cli::try_parse_from(["jst", "show", "git", "--version"])
            .expect("trailing prompt values may begin with a hyphen");

        assert_eq!(cli.prompt.join(" "), "show git --version");
    }

    #[test]
    fn confirmation_policy_combines_both_warning_sources() {
        assert!(!should_confirm(false, &[], &[]));
        assert!(should_confirm(false, &["local"], &[]));
        assert!(should_confirm(false, &[], &["model"]));
        assert!(!should_confirm(true, &["local"], &["model"]));
    }

    #[test]
    fn parses_review_actions_with_safe_defaults() {
        assert_eq!(parse_review_action("y"), Some(ReviewAction::Run));
        assert_eq!(parse_review_action("run"), Some(ReviewAction::Run));
        assert_eq!(parse_review_action("w"), Some(ReviewAction::Why));
        assert_eq!(parse_review_action("explain"), Some(ReviewAction::Why));
        assert_eq!(parse_review_action("a"), Some(ReviewAction::AskAi));
        assert_eq!(parse_review_action("change"), Some(ReviewAction::AskAi));
        assert_eq!(parse_review_action("e"), Some(ReviewAction::Edit));
        assert_eq!(parse_review_action("manual"), Some(ReviewAction::Edit));
        assert_eq!(parse_review_action(""), Some(ReviewAction::Abort));
        assert_eq!(parse_review_action("q"), Some(ReviewAction::Abort));
        assert_eq!(parse_review_action("maybe"), None);

        let plain = format_review_prompt(false);
        assert!(plain.contains("[w]hy"));
        assert!(plain.contains("[a]sk AI"));
        assert!(plain.contains("[e]dit"));
        assert!(!plain.contains('\u{1b}'));

        let colored = format_review_prompt(true);
        assert!(colored.contains("[a]sk AI"));
        assert!(colored.contains("[e]dit"));
        assert!(colored.contains("\u{1b}[35m"));
    }

    #[test]
    fn edit_prompt_explains_run_and_cursor_helpers_respect_utf8() {
        let prompt = format_edit_prompt(false);
        assert!(prompt.contains("Edit the command before running it"));
        assert!(prompt.contains("enter to run"));
        assert!(!prompt.contains('\u{1b}'));

        let value = "aéz";
        assert_eq!(next_char_end(value, 0), Some(1));
        assert_eq!(next_char_end(value, 1), Some(3));
        assert_eq!(previous_char_start(value, value.len()), Some(3));
        assert_eq!(previous_char_start(value, 3), Some(1));
    }

    #[test]
    fn manual_explanations_make_the_local_boundary_explicit() {
        let response = TranslateResponse {
            command: "find .".to_string(),
            effects: CommandEffects::default(),
            matches_request: true,
            explanation: "Model-generated explanation".to_string(),
            parts: Vec::new(),
        };

        let explanation = format_proposal_explanation(
            &response,
            &response.command,
            "show files",
            ProposalKind::Manual,
            88,
            false,
        );

        assert!(explanation.contains("did not send it to AI"));
        assert!(explanation.contains("only local safety checks"));
        assert!(!explanation.contains("Model-generated explanation"));
    }

    #[test]
    fn formats_structured_explanations_for_wide_and_narrow_terminals() {
        let response = TranslateResponse {
            command: "du -ah . | sort -hr | head -n 10".to_string(),
            effects: CommandEffects {
                reads_data: true,
                ..CommandEffects::default()
            },
            matches_request: true,
            explanation: "Shows the ten largest entries.".to_string(),
            parts: vec![
                CommandPart {
                    fragment: "du -ah .".to_string(),
                    meaning: "measure every entry".to_string(),
                    source: "files in this folder".to_string(),
                },
                CommandPart {
                    fragment: " | sort -hr".to_string(),
                    meaning: "order sizes largest first".to_string(),
                    source: "largest".to_string(),
                },
                CommandPart {
                    fragment: " | head -n 10".to_string(),
                    meaning: "keep the first ten results".to_string(),
                    source: "show the 10".to_string(),
                },
            ],
        };
        let input = "show the 10 largest files in this folder";

        let wide = format_detailed_explanation(&response, &response.command, input, 88, false);
        assert!(wide.contains("du -ah ."));
        assert!(wide.contains("measure every entry (“files in this folder”)"));
        assert!(wide.contains("Effects: reads local data."));

        let narrow = format_detailed_explanation(&response, &response.command, input, 44, false);
        assert!(narrow.contains("\n    measure every entry"));
        assert!(narrow.lines().all(|line| line.chars().count() <= 44));
    }

    #[test]
    fn malformed_parts_fall_back_to_the_summary() {
        let response = TranslateResponse {
            command: "pwd".to_string(),
            effects: CommandEffects::default(),
            matches_request: true,
            explanation: "Prints the current directory.".to_string(),
            parts: vec![CommandPart {
                fragment: "not-pwd".to_string(),
                meaning: "misleading".to_string(),
                source: "current directory".to_string(),
            }],
        };

        let formatted = format_detailed_explanation(
            &response,
            &response.command,
            "show current directory",
            88,
            false,
        );
        assert!(formatted.contains("Prints the current directory."));
        assert!(!formatted.contains("misleading"));
        assert!(formatted.contains("Effects: no external effects reported."));
    }

    #[test]
    fn rejects_terminal_spoofing_characters() {
        assert!(contains_unsafe_terminal_character("echo ok\rmalicious"));
        assert!(contains_unsafe_terminal_character("echo \u{1b}[2J"));
        assert!(contains_unsafe_terminal_character("echo \u{202e}txt"));
        assert!(!contains_unsafe_terminal_character("echo safe"));
        assert_eq!(terminal_safe("line\nnext\u{1b}"), "line\\nnext\\u{1b}");
    }

    #[test]
    fn describes_errors_in_plain_language() {
        assert_eq!(
            JstError::Network.to_string(),
            "couldn't reach the jst server — check your connection and try again"
        );
        assert_eq!(
            JstError::Server(429).to_string(),
            "rate limit reached — slow down, or run your own jst server"
        );
        assert_eq!(
            JstError::LlmProvider.to_string(),
            "trouble reaching the LLM; try again in a moment"
        );
        assert_eq!(
            JstError::Deserialization.to_string(),
            "got an unexpected response from the jst server"
        );
    }

    #[test]
    fn wraps_error_messages_with_jst_prefix() {
        assert_eq!(
            format_error(&JstError::LlmProvider, false),
            "jst: trouble reaching the LLM; try again in a moment"
        );
        assert_eq!(
            format_error(&JstError::LlmProvider, true),
            "\x1b[1;31mjst: trouble reaching the LLM; try again in a moment\x1b[0m"
        );
    }
}
