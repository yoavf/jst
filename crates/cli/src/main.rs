use crossterm::{
    cursor::{MoveDown, MoveToColumn, MoveUp},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use jst_shared::{TranslateRequest, TranslateResponse};
use std::io::{stdout, Write};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
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
        }
    }
}

/// Try to reuse the user's shell prompt and add a jst marker.
fn get_prompt() -> String {
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
    // Prefer inherited env when jst is launched from an interactive shell
    if let Ok(env_prompt) = std::env::var("PROMPT") {
        let trimmed = env_prompt.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    if let Ok(env_ps1) = std::env::var("PS1") {
        let trimmed = env_ps1.trim();
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
                    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
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
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }

    None
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
    let args: Vec<String> = std::env::args().skip(1).collect();

    let prompt = get_prompt();
    let mut initial_state = AppState::new(prompt);

    if !args.is_empty() {
        let input = args.join(" ");
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
    let mut stdout = stdout();

    // Main loop
    let result = run_loop(state.clone(), needs_render).await;

    // Cleanup: clear our lines and restore terminal
    {
        let state_guard = state.lock().await;
        cleanup(&*state_guard, &mut stdout)?;
    }
    terminal::disable_raw_mode()?;

    // Execute the command seamlessly
    if let Ok(Some(command)) = result {
        // Show the command that will run (keep translation line style), then execute
        println!("⮑ {}", command);
        stdout.flush()?;

        execute_command(&command)?;
    }

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
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut stdout = stdout();
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
            let trimmed_input = state_guard.input.trim();
            let trimmed_last = state_guard.last_translated_input.trim();

            if elapsed >= Duration::from_millis(DEBOUNCE_MS)
                && !trimmed_input.is_empty()
                && trimmed_input != trimmed_last
                && !state_guard.is_translating
                && state_guard.mode == InputMode::Natural
            {
                state_guard.is_translating = true;
                state_guard.last_translated_input = state_guard.input.clone();
                let context = state_guard.request_context.take();
                let input = state_guard.input.trim().to_string();
                needs_render.store(true, Ordering::SeqCst);
                drop(state_guard);

                // Spawn translation task
                let state_clone = state.clone();
                let client_clone = client.clone();
                let needs_render_clone = needs_render.clone();
                tokio::spawn(async move {
                    let result = translate(&client_clone, &input, context).await;
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
            render(&mut *state_guard, &mut stdout)?;
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
            if state.mode == InputMode::Command {
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
            if state.mode == InputMode::Natural {
                if let Some(prev) = state.last_translation.clone() {
                    state.request_context =
                        Some(format!("previous_translation_rejected: {}", prev));
                } else {
                    state.request_context = None;
                }
                state.last_translated_input.clear();
                state.last_input_time = Instant::now() - Duration::from_millis(DEBOUNCE_MS + 1);
            } else {
                state.status_msg = "Regenerate works in natural mode".to_string();
            }
            KeyAction::Continue
        }
        KeyCode::Char(c) => {
            state.input.insert(state.cursor_pos, c);
            state.cursor_pos += 1;
            state.last_input_time = Instant::now();
            if state.mode == InputMode::Natural {
                state.last_nl_input = state.input.clone();
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
    state.status_msg = match state.mode {
        InputMode::Natural => {
            "ENTER run • CTRL+R regenerate • TAB accept cmd • ESC quit".to_string()
        }
        InputMode::Command => {
            "ENTER run edited cmd • ESC back to natural • CTRL+C quit".to_string()
        }
    };
}

fn render(state: &mut AppState, stdout: &mut impl Write) -> Result<(), Box<dyn std::error::Error>> {
    // Move to start of current line and clear it
    execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine),)?;

    // Render shell-like prompt + input
    execute!(
        stdout,
        SetForegroundColor(Color::DarkGrey),
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
            SetForegroundColor(Color::DarkGrey),
            Print(format!("  {} ", spinner)),
            ResetColor,
        )?;
    } else if !state.translation.is_empty() {
        execute!(
            stdout,
            SetForegroundColor(Color::Cyan),
            Print("  ⮑ "),
            Print(&state.translation),
            ResetColor,
        )?;
    }

    // Status bar line
    execute!(
        stdout,
        Print("\n"),
        MoveToColumn(0),
        Clear(ClearType::CurrentLine),
        SetForegroundColor(Color::DarkGrey),
        Print(&state.status_msg),
        ResetColor,
    )?;

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
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let request = TranslateRequest {
        input: input.to_string(),
        context,
        os: Some(std::env::consts::OS.to_string()),
        shell: std::env::var("SHELL").ok(),
    };

    let response = client.post(API_URL).json(&request).send().await?;

    if response.status().is_success() {
        let translate_response: TranslateResponse = response.json().await?;
        Ok(translate_response.command)
    } else {
        Ok("# unable to translate".to_string())
    }
}

fn execute_command(command: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
