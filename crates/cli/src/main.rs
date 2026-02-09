use crossterm::{
    cursor::{MoveDown, MoveToColumn, MoveUp},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use jst_shared::{TranslateRequest, TranslateResponse};
mod cli_args;
mod cache;
use cache::{
    load_last_command_for_current_shell_session, save_last_command_for_current_shell_session,
    PersistentCache, PromptHistory,
};
use cli_args::{parse_cli_mode, CliMode};
use std::fs::OpenOptions;
use std::io::{stdout, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

const API_URL: &str = "https://jst-server.fly.dev/translate";
const DEBOUNCE_MS: u64 = 300;
const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

#[derive(Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Natural,
    Command,
}

#[derive(Clone)]
struct AppState {
    input: String,
    cursor_pos: usize,
    translation: String,
    is_translating: bool,
    spinner_frame: usize,
    last_input_time: Instant,
    last_translated_input: String,
    prompt: String,
    prompt_width: usize,
    mode: InputMode,
    last_translation: Option<String>,
    last_nl_input: String,
    status_msg: String,
    request_context: Option<String>,
    skip_cache_once: bool,
    last_executed_command: Option<String>,
    session_cache: std::collections::HashMap<String, String>,
    persistent_cache: PersistentCache,
    history: PromptHistory,
    /// Index into history for up/down navigation. None = not browsing history.
    history_index: Option<usize>,
    /// Stash the user's in-progress input when they start browsing history.
    history_stash: String,
}

enum UiWriter {
    Stdout(std::io::Stdout),
}

impl Write for UiWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            UiWriter::Stdout(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            UiWriter::Stdout(w) => w.flush(),
        }
    }
}

impl AppState {
    fn new(prompt: String) -> Self {
        let prompt_width = visible_len(&prompt);
        Self {
            input: String::new(),
            cursor_pos: 0,
            translation: String::new(),
            is_translating: false,
            spinner_frame: 0,
            last_input_time: Instant::now(),
            last_translated_input: String::new(),
            prompt,
            prompt_width,
            mode: InputMode::Natural,
            last_translation: None,
            last_nl_input: String::new(),
            status_msg: String::new(),
            request_context: None,
            skip_cache_once: false,
            last_executed_command: load_last_command_for_current_shell_session(),
            session_cache: std::collections::HashMap::new(),
            persistent_cache: PersistentCache::load(),
            history: PromptHistory::load(),
            history_index: None,
            history_stash: String::new(),
        }
    }
}

/// Try to reuse the user's shell prompt and add a jst marker.
fn get_prompt() -> String {
    if is_warp_terminal() {
        return String::new();
    }

    if let Some(shell_prompt) = detect_prompt() {
        let mut p = format!("[jst] {}", shell_prompt);
        if !p.ends_with(' ') {
            p.push(' ');
        }
        return p;
    }

    // Fallback synthetic prompt
    let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
    let home = std::env::var("HOME").unwrap_or_default();
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "~".to_string());

    let cwd_display = if !home.is_empty() && cwd.starts_with(&home) {
        cwd.replacen(&home, "~", 1)
    } else {
        cwd
    };

    let shell = std::env::var("SHELL").unwrap_or_default();
    let base = if shell.contains("zsh") {
        format!("{}@{} ", user, cwd_display)
    } else {
        format!("{}:{}$ ", user, cwd_display)
    };

    let mut p = format!("[jst] {}", base);
    if !p.ends_with(' ') {
        p.push(' ');
    }
    p
}

fn detect_prompt() -> Option<String> {
    // Warp injects prompt integration metadata that can produce invisible or
    // layout-breaking prompt strings when replayed by jst. Use fallback prompt there.
    if is_warp_terminal() {
        return None;
    }

    // Prefer inherited env when jst is launched from an interactive shell
    if let Ok(env_prompt) = std::env::var("PROMPT") {
        let normalized = normalize_prompt(&env_prompt);
        let trimmed = normalized.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    if let Ok(env_ps1) = std::env::var("PS1") {
        let normalized = normalize_prompt(&env_ps1);
        let trimmed = normalized.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    let shell = std::env::var("SHELL").ok()?;
    let shell_name = std::path::Path::new(&shell)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    if shell_name.contains("zsh") {
        // Prefer prompt-expanded form (%F, %~ etc.) via print -P; fall back to raw
        for cmd in ["print -P -- \"$PROMPT\"", "print -r -- $PROMPT"] {
            if let Ok(out) = Command::new("zsh").arg("-ic").arg(cmd).output() {
                if out.status.success() {
                    let raw = String::from_utf8_lossy(&out.stdout).to_string();
                    let s = normalize_prompt(&raw).trim().to_string();
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
        }
    }

    if shell_name.contains("bash") {
        if let Ok(out) = Command::new("bash")
            .arg("-lc")
            .arg("printf %s \"$PS1\"")
            .output()
        {
            if out.status.success() {
                let raw = String::from_utf8_lossy(&out.stdout).to_string();
                let s = normalize_prompt(&raw).trim().to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }

    None
}

fn is_warp_terminal() -> bool {
    std::env::var("TERM_PROGRAM")
        .map(|v| v == "WarpTerminal")
        .unwrap_or(false)
}

fn subtle_color() -> Color {
    if is_warp_terminal() {
        Color::Grey
    } else {
        Color::DarkGrey
    }
}

fn translation_prefix() -> &'static str {
    if is_warp_terminal() {
        "-> "
    } else {
        "⮑ "
    }
}

fn status_separator() -> &'static str {
    if is_warp_terminal() {
        " | "
    } else {
        " • "
    }
}

fn normalize_prompt(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            match chars.peek().copied() {
                // CSI
                Some('[') => {
                    chars.next();
                    while let Some(c) = chars.next() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                // OSC
                Some(']') => {
                    chars.next();
                    let mut prev_esc = false;
                    while let Some(c) = chars.next() {
                        if c == '\u{7}' {
                            break;
                        }
                        if prev_esc && c == '\\' {
                            break;
                        }
                        prev_esc = c == '\u{1b}';
                    }
                }
                // Other single-char escape
                Some(_) => {
                    chars.next();
                }
                None => {}
            }
            continue;
        }

        if ch.is_control() {
            continue;
        }

        out.push(ch);
    }

    out.trim_start().to_string()
}

fn visible_len(s: &str) -> usize {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut count = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b {
            // Skip ANSI escape sequence: ESC [
            i += 1;
            if i < bytes.len() && bytes[i] == b'[' {
                i += 1;
                while i < bytes.len() {
                    let b = bytes[i];
                    i += 1;
                    if (0x40..=0x7e).contains(&b) {
                        break;
                    }
                }
            }
        } else {
            count += 1;
            i += 1;
        }
    }
    count
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mode = parse_cli_mode();

    match mode {
        CliMode::PrintCommand { input } => {
            run_print_command_mode(input).await?;
            return Ok(());
        }
        CliMode::Interactive { .. } => {}
    }

    let prefill = match mode {
        CliMode::Interactive { prefill } => prefill,
        _ => unreachable!(),
    };

    let prompt = get_prompt();
    let mut initial_state = AppState::new(prompt);

    if let Some(input) = prefill {
        initial_state.input = input.clone();
        initial_state.last_nl_input = input;
        initial_state.cursor_pos = initial_state.input.len();
        // Force immediate translation once loop starts
        initial_state.last_input_time = Instant::now() - Duration::from_millis(DEBOUNCE_MS + 1);
    }

    set_status_for_mode(&mut initial_state);

    let state = Arc::new(Mutex::new(initial_state));
    let needs_render = Arc::new(AtomicBool::new(true));

    // Setup terminal - stay inline, just enable raw mode
    terminal::enable_raw_mode()?;
    let mut ui_out = UiWriter::Stdout(stdout());

    // Main loop
    let result = run_loop(state.clone(), needs_render, &mut ui_out).await;

    // Cleanup: clear our lines and restore terminal
    {
        let state_guard = state.lock().await;
        cleanup(&*state_guard, &mut ui_out)?;
    }
    terminal::disable_raw_mode()?;

    // Execute the command seamlessly
    if let Ok(Some(command)) = result {
        // Show the command that will run (keep translation line style), then execute
        println!("{}{}", translation_prefix(), command);
        std::io::stdout().flush()?;
        execute_command(&command)?;
    }

    Ok(())
}

async fn run_print_command_mode(
    raw_input: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let trimmed_input = raw_input.trim().to_string();
    if trimmed_input.is_empty() {
        return Ok(());
    }

    let mut input = trimmed_input.clone();
    let mut context: Option<String> = None;

    if let Some(follow_up) = parse_follow_up_request(&trimmed_input) {
        if let Some(previous_command) = load_last_command_for_current_shell_session() {
            input = follow_up;
            context = Some(follow_up_context(&previous_command));
        } else {
            if dev_log_enabled() {
                dev_log_line("print-command follow-up requested but no previous command found");
            }
            return Ok(());
        }
    }

    let client = reqwest::Client::new();
    let state = Arc::new(Mutex::new(AppState::new(String::new())));
    let command = translate(&client, &input, context, false, state).await?;
    let command = command.trim().to_string();
    if command.is_empty() {
        return Ok(());
    }

    if !command.starts_with("# ") {
        save_last_command_for_current_shell_session(&command);
    }

    println!("{}", command);
    Ok(())
}


fn cleanup(
    _state: &AppState,
    stdout: &mut impl Write,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Clear input + translation + status lines without adding blank lines
    execute!(
        stdout,
        MoveToColumn(0),
        Clear(ClearType::CurrentLine),
        MoveDown(1),
        MoveToColumn(0),
        Clear(ClearType::CurrentLine),
        MoveDown(1),
        MoveToColumn(0),
        Clear(ClearType::CurrentLine),
        MoveUp(2),
        MoveToColumn(0),
    )?;

    stdout.flush()?;
    Ok(())
}

async fn run_loop(
    state: Arc<Mutex<AppState>>,
    needs_render: Arc<AtomicBool>,
    stdout: &mut UiWriter,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut last_poll = Instant::now();

    loop {
        // Poll for events with a short timeout for responsive updates
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key_event) = event::read()? {
                let mut state_guard = state.lock().await;

                match handle_key_event(key_event, &mut state_guard) {
                    KeyAction::Continue => {}
                    KeyAction::Execute(command) => {
                        if !command.starts_with("# ") {
                            let nl_input = state_guard.last_nl_input.trim().to_string();
                            if !nl_input.is_empty() {
                                state_guard
                                    .session_cache
                                    .insert(nl_input.clone(), command.clone());
                                state_guard.persistent_cache.insert(&nl_input, &command);
                                state_guard.history.push(&nl_input);
                            }
                            state_guard.last_executed_command = Some(command.clone());
                            save_last_command_for_current_shell_session(&command);
                        }
                        return Ok(Some(command));
                    }
                    KeyAction::Quit => {
                        return Ok(None);
                    }
                }

                if state_guard.status_msg.is_empty() {
                    set_status_for_mode(&mut state_guard);
                }

                needs_render.store(true, Ordering::SeqCst);
            }
        }

        // Check if we need to trigger translation (debounce)
        {
            let mut state_guard = state.lock().await;
            let now = Instant::now();
            let elapsed = now.duration_since(state_guard.last_input_time);

            // Trim for comparison to avoid re-translating on trailing spaces
            let trimmed_input = state_guard.input.trim().to_string();
            let trimmed_last = state_guard.last_translated_input.trim().to_string();
            let comparison_input = if parse_follow_up_request(&trimmed_input).is_some()
                && state_guard.last_executed_command.is_some()
            {
                parse_follow_up_request(&trimmed_input).unwrap_or_else(|| trimmed_input.clone())
            } else {
                trimmed_input.clone()
            };

            if state_guard.mode == InputMode::Natural && trimmed_input.is_empty() {
                if !state_guard.translation.is_empty() || !state_guard.last_translated_input.is_empty()
                {
                    state_guard.translation.clear();
                    state_guard.last_translated_input.clear();
                    needs_render.store(true, Ordering::SeqCst);
                }
            }

            if elapsed >= Duration::from_millis(DEBOUNCE_MS)
                && !trimmed_input.is_empty()
                && comparison_input != trimmed_last
                && !state_guard.is_translating
                && state_guard.mode == InputMode::Natural
            {
                state_guard.is_translating = true;
                let context = state_guard.request_context.take();
                let skip_cache = std::mem::take(&mut state_guard.skip_cache_once);
                let mut input = trimmed_input.clone();
                let mut context = context;

                if let Some(follow_up) = parse_follow_up_request(&trimmed_input) {
                    if let Some(prev_command) = state_guard.last_executed_command.clone() {
                        input = follow_up;
                        let follow_up_context = follow_up_context(&prev_command);
                        context = match context {
                            Some(existing) => Some(format!("{}\n\n{}", existing, follow_up_context)),
                            None => Some(follow_up_context),
                        };
                    } else {
                        state_guard.translation.clear();
                        state_guard.last_translated_input = state_guard.input.clone();
                        state_guard.is_translating = false;
                        state_guard.status_msg =
                            "No previous command in this shell session".to_string();
                        needs_render.store(true, Ordering::SeqCst);
                        continue;
                    }
                }
                // Track the actual request payload (not raw UI input) so response matching works
                // for follow-up requests that transform input (for example '^ ...').
                state_guard.last_translated_input = input.clone();
                needs_render.store(true, Ordering::SeqCst);
                drop(state_guard);

                // Spawn translation task
                let state_clone = state.clone();
                let client_clone = client.clone();
                let needs_render_clone = needs_render.clone();
                tokio::spawn(async move {
                    let result =
                        translate(&client_clone, &input, context, skip_cache, state_clone.clone())
                            .await;
                    let mut state_guard = state_clone.lock().await;
                    state_guard.is_translating = false;
                    if let Ok(translation) = result {
                        // Only update if input hasn't changed significantly
                        if state_guard.last_translated_input.trim() == input {
                            state_guard.translation = translation;
                            state_guard.last_translation = Some(state_guard.translation.clone());
                        }
                    }
                    // Signal that we need to render the result
                    needs_render_clone.store(true, Ordering::SeqCst);
                });
            }
        }

        // Update spinner and render
        if last_poll.elapsed() >= Duration::from_millis(80) {
            let mut state_guard = state.lock().await;
            if state_guard.is_translating {
                state_guard.spinner_frame = (state_guard.spinner_frame + 1) % SPINNER_FRAMES.len();
                needs_render.store(true, Ordering::SeqCst);
            }
            last_poll = Instant::now();
        }

        // Render if needed
        if needs_render.swap(false, Ordering::SeqCst) {
            let mut state_guard = state.lock().await;
            render(&mut *state_guard, stdout)?;
        }
    }
}

enum KeyAction {
    Continue,
    Execute(String),
    Quit,
}

fn handle_key_event(event: KeyEvent, state: &mut AppState) -> KeyAction {
    match event.code {
        KeyCode::Esc => {
            if !state.input.is_empty() {
                state.input.clear();
                state.cursor_pos = 0;
                state.last_input_time = Instant::now();
                if state.mode == InputMode::Natural {
                    state.translation.clear();
                    state.last_translated_input.clear();
                    state.last_nl_input.clear();
                    state.last_translation = None;
                    state.request_context = None;
                }
                set_status_for_mode(state);
                KeyAction::Continue
            } else if state.mode == InputMode::Command {
                state.mode = InputMode::Natural;
                state.input = state.last_nl_input.clone();
                state.cursor_pos = state.input.len();
                state.translation.clear();
                state.last_translated_input.clear();
                state.last_input_time = Instant::now();
                set_status_for_mode(state);
                KeyAction::Continue
            } else {
                KeyAction::Quit
            }
        }
        KeyCode::Char('c') if event.modifiers.contains(KeyModifiers::CONTROL) => KeyAction::Quit,
        KeyCode::Enter => match state.mode {
            InputMode::Natural => {
                if !state.translation.is_empty() && !state.translation.starts_with("# ") {
                    KeyAction::Execute(state.translation.clone())
                } else {
                    KeyAction::Continue
                }
            }
            InputMode::Command => KeyAction::Execute(state.input.clone()),
        },
        KeyCode::Char('e') if event.modifiers.contains(KeyModifiers::CONTROL) => {
            if !state.translation.is_empty() && !state.translation.starts_with("# ") {
                state.last_translation = Some(state.translation.clone());
                state.input = state.translation.clone();
                state.cursor_pos = state.input.len();
                state.mode = InputMode::Command;
                state.translation.clear();
                state.last_translated_input.clear();
                set_status_for_mode(state);
            }
            KeyAction::Continue
        }
        KeyCode::Tab => {
            if state.mode == InputMode::Natural {
                if !state.translation.is_empty() && !state.translation.starts_with("# ") {
                    state.last_translation = Some(state.translation.clone());
                    state.input = state.translation.clone();
                    state.cursor_pos = state.input.len();
                    state.mode = InputMode::Command;
                    state.translation.clear();
                    state.last_translated_input.clear();
                    set_status_for_mode(state);
                } else {
                    state.status_msg = "No translation to accept yet".to_string();
                }
            } else {
                state.status_msg = "Tab accepts translation in natural mode".to_string();
            }
            KeyAction::Continue
        }
        KeyCode::Char('r') if event.modifiers.contains(KeyModifiers::CONTROL) => {
            if state.mode == InputMode::Natural
                && !state.translation.is_empty()
                && !state.translation.starts_with("# ")
            {
                if let Some(prev) = state.last_translation.clone() {
                    state.request_context =
                        Some(format!("previous_translation_rejected: {}", prev));
                } else {
                    state.request_context = None;
                }
                state.skip_cache_once = true;
                state.last_translated_input.clear();
                state.last_input_time = Instant::now() - Duration::from_millis(DEBOUNCE_MS + 1);
            } else {
                state.status_msg = "Regenerate works in natural mode".to_string();
            }
            KeyAction::Continue
        }
        KeyCode::Up if state.mode == InputMode::Natural => {
            let entries = state.history.entries();
            if !entries.is_empty() {
                let idx = match state.history_index {
                    None => {
                        state.history_stash = state.input.clone();
                        entries.len() - 1
                    }
                    Some(i) if i > 0 => i - 1,
                    Some(i) => i,
                };
                state.history_index = Some(idx);
                state.input = entries[idx].clone();
                state.cursor_pos = state.input.len();
                state.last_nl_input = state.input.clone();
                state.last_input_time = Instant::now();
            }
            KeyAction::Continue
        }
        KeyCode::Down if state.mode == InputMode::Natural => {
            let entries = state.history.entries();
            if let Some(idx) = state.history_index {
                if idx + 1 < entries.len() {
                    let new_idx = idx + 1;
                    state.history_index = Some(new_idx);
                    state.input = entries[new_idx].clone();
                } else {
                    state.history_index = None;
                    state.input = state.history_stash.clone();
                }
                state.cursor_pos = state.input.len();
                state.last_nl_input = state.input.clone();
                state.last_input_time = Instant::now();
            }
            KeyAction::Continue
        }
        KeyCode::Char(c) => {
            state.input.insert(state.cursor_pos, c);
            state.cursor_pos += 1;
            state.last_input_time = Instant::now();
            if state.mode == InputMode::Natural {
                state.last_nl_input = state.input.clone();
                state.history_index = None;
            }
            KeyAction::Continue
        }
        KeyCode::Backspace => {
            if state.cursor_pos > 0 {
                state.cursor_pos -= 1;
                state.input.remove(state.cursor_pos);
                state.last_input_time = Instant::now();
                if state.mode == InputMode::Natural {
                    state.last_nl_input = state.input.clone();
                    state.history_index = None;
                }
            }
            KeyAction::Continue
        }
        KeyCode::Delete => {
            if state.cursor_pos < state.input.len() {
                state.input.remove(state.cursor_pos);
                state.last_input_time = Instant::now();
                if state.mode == InputMode::Natural {
                    state.last_nl_input = state.input.clone();
                    state.history_index = None;
                }
            }
            KeyAction::Continue
        }
        KeyCode::Left => {
            if state.cursor_pos > 0 {
                state.cursor_pos -= 1;
            }
            KeyAction::Continue
        }
        KeyCode::Right => {
            if state.cursor_pos < state.input.len() {
                state.cursor_pos += 1;
            }
            KeyAction::Continue
        }
        KeyCode::Home => {
            state.cursor_pos = 0;
            KeyAction::Continue
        }
        KeyCode::End => {
            state.cursor_pos = state.input.len();
            KeyAction::Continue
        }
        _ => KeyAction::Continue,
    }
}

fn set_status_for_mode(state: &mut AppState) {
    state.status_msg.clear();
}

fn parse_follow_up_request(input: &str) -> Option<String> {
    if !input.starts_with('^') {
        return None;
    }

    let follow_up = input[1..].trim();
    if follow_up.is_empty() {
        None
    } else {
        Some(follow_up.to_string())
    }
}

fn follow_up_context(previous_command: &str) -> String {
    format!(
        "Follow-up edit request.\nPrevious command:\n{}\n\nInterpret the user input as modifications to the previous command. Return a single updated shell command only.",
        previous_command
    )
}

fn status_parts(state: &AppState) -> Vec<(String, bool)> {
    let trimmed_input = state.input.trim();
    let translation_ready = !state.translation.is_empty() && !state.translation.starts_with("# ");

    let mut parts: Vec<(String, bool)> = Vec::new();

    match state.mode {
        InputMode::Natural => {
            parts.push(("ENTER run".to_string(), translation_ready));
            parts.push(("TAB accept cmd".to_string(), translation_ready));
            parts.push(("CTRL+R regenerate".to_string(), translation_ready));
            parts.push((
                "^ edit last cmd".to_string(),
                state.last_executed_command.is_some(),
            ));
            if trimmed_input.is_empty() {
                parts.push(("ESC quit".to_string(), true));
            } else {
                parts.push(("ESC clear".to_string(), true));
            }
        }
        InputMode::Command => {
            parts.push((
                "ENTER run edited cmd".to_string(),
                !trimmed_input.is_empty(),
            ));
            if trimmed_input.is_empty() {
                parts.push(("ESC back to natural".to_string(), true));
            } else {
                parts.push(("ESC clear".to_string(), true));
            }
            parts.push(("CTRL+C quit".to_string(), true));
        }
    }

    parts
}

fn render(
    state: &mut AppState,
    stdout: &mut impl Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let subtle = subtle_color();

    // Move to start of current line and clear it
    execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine),)?;

    // Render shell-like prompt + input
    execute!(
        stdout,
        SetForegroundColor(subtle),
        Print(&state.prompt),
        ResetColor,
        Print(&state.input),
    )?;

    // Translation line
    execute!(
        stdout,
        Print("\n"),
        MoveToColumn(0),
        Clear(ClearType::CurrentLine)
    )?;
    if state.is_translating {
        let spinner = SPINNER_FRAMES[state.spinner_frame];
        execute!(
            stdout,
            SetForegroundColor(subtle),
            Print(format!("{} ", spinner)),
            ResetColor,
        )?;
    } else if !state.translation.is_empty() {
        execute!(
            stdout,
            SetForegroundColor(Color::Cyan),
            Print(translation_prefix()),
            Print(&state.translation),
            ResetColor,
        )?;
    } else if state.mode == InputMode::Command && !state.last_nl_input.trim().is_empty() {
        execute!(
            stdout,
            SetForegroundColor(subtle),
            Print("nl: "),
            Print(&state.last_nl_input),
            ResetColor,
        )?;
    }

    // Status bar line
    execute!(
        stdout,
        Print("\n"),
        MoveToColumn(0),
        Clear(ClearType::CurrentLine)
    )?;
    if !state.status_msg.is_empty() {
        execute!(
            stdout,
            SetForegroundColor(subtle),
            Print(&state.status_msg),
            ResetColor,
        )?;
    } else {
        let parts = status_parts(state);
        for (idx, (text, active)) in parts.into_iter().enumerate() {
            if idx > 0 {
                execute!(
                    stdout,
                    SetForegroundColor(subtle),
                    Print(status_separator()),
                    ResetColor
                )?;
            }
            if active {
                execute!(stdout, ResetColor, Print(text))?;
            } else {
                execute!(
                    stdout,
                    SetForegroundColor(subtle),
                    Print(text),
                    ResetColor
                )?;
            }
        }
    }

    // Move back up to input line
    execute!(stdout, MoveUp(2))?;

    // Position cursor on input line (prompt length + cursor position)
    let cursor_col = state.prompt_width as u16 + state.cursor_pos as u16;
    execute!(stdout, MoveToColumn(cursor_col))?;

    stdout.flush()?;
    Ok(())
}

async fn translate(
    client: &reqwest::Client,
    input: &str,
    context: Option<String>,
    skip_cache: bool,
    state: Arc<Mutex<AppState>>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(String::new());
    }

    if !skip_cache {
        let mut state_guard = state.lock().await;
        if let Some(cmd) = state_guard.session_cache.get(input) {
            if dev_log_enabled() {
                dev_log_line(&format!(
                    "cache hit (session) input={:?} command={:?}",
                    input, cmd
                ));
            }
            return Ok(cmd.clone());
        }
        if let Some(cmd) = state_guard.persistent_cache.get(input) {
            let cmd_clone = cmd.clone();
            state_guard
                .session_cache
                .insert(input.to_string(), cmd_clone.clone());
            if dev_log_enabled() {
                dev_log_line(&format!(
                    "cache hit (persistent) input={:?} command={:?}",
                    input, cmd_clone
                ));
            }
            return Ok(cmd_clone);
        }
    }

    let request = TranslateRequest {
        input: input.to_string(),
        context,
        os: Some(std::env::consts::OS.to_string()),
        shell: std::env::var("SHELL").ok(),
    };

    if dev_log_enabled() {
        if let Ok(payload) = serde_json::to_string_pretty(&request) {
            dev_log_line(&format!("outbound /translate request:\n{}", payload));
        }
    }

    let response = client.post(API_URL).json(&request).send().await?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if dev_log_enabled() {
        dev_log_line(&format!(
            "inbound /translate response (status={}):\n{}",
            status, body
        ));
    }

    if status.is_success() {
        let translate_response: TranslateResponse = serde_json::from_str(&body)?;
        let command = translate_response.command;
        let mut state_guard = state.lock().await;
        state_guard
            .session_cache
            .insert(input.to_string(), command.clone());
        if skip_cache {
            state_guard.persistent_cache.insert(input, &command);
        }
        Ok(command)
    } else {
        Ok("# unable to translate".to_string())
    }
}

fn dev_log_enabled() -> bool {
    match std::env::var("JST_DEV_LOG") {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            v == "1" || v == "true" || v == "yes" || v == "on"
        }
        Err(_) => false,
    }
}

fn dev_log_file_path() -> PathBuf {
    std::env::temp_dir().join("jst-dev.log")
}

fn dev_log_line(message: &str) {
    if !dev_log_enabled() {
        return;
    }

    let path = dev_log_file_path();
    let mut file = match OpenOptions::new().create(true).append(true).open(path) {
        Ok(f) => f,
        Err(_) => return,
    };

    let ts = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => 0,
    };

    let _ = writeln!(file, "[{}] JST_DEV_LOG {}", ts, message);
}


fn execute_command(command: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    append_to_shell_history(command);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = Command::new("sh").arg("-c").arg(command).exec();
        eprintln!("exec failed: {}", err);
    }

    #[cfg(not(unix))]
    {
        Command::new("sh").arg("-c").arg(command).status()?;
    }

    Ok(())
}

fn append_to_shell_history(command: &str) {
    let command = command.trim();
    if command.is_empty() {
        return;
    }

    let shell = std::env::var("SHELL").unwrap_or_default();
    let history_path = std::env::var("HISTFILE")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| default_history_file_for_shell(&shell));

    let Some(path) = history_path else {
        return;
    };

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut file = match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => f,
        Err(_) => return,
    };

    let _ = writeln!(file, "{}", command);

    if dev_log_enabled() {
        dev_log_line(&format!(
            "history append path={} command={:?}",
            path.display(),
            command
        ));
    }
}

fn default_history_file_for_shell(shell_path: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let shell = Path::new(shell_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    if shell.contains("zsh") {
        Some(home.join(".zsh_history"))
    } else if shell.contains("bash") {
        Some(home.join(".bash_history"))
    } else {
        None
    }
}
