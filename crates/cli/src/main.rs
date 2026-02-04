use crossterm::{
    cursor::{MoveToColumn, MoveUp},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use jst_shared::{TranslateRequest, TranslateResponse};
use std::io::{stdout, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const API_URL: &str = "https://jst-server.fly.dev/translate";
const DEBOUNCE_MS: u64 = 300;
const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const PROMPT: &str = "> ";

#[derive(Clone)]
struct AppState {
    input: String,
    cursor_pos: usize,
    translation: String,
    is_translating: bool,
    spinner_frame: usize,
    last_input_time: Instant,
    last_translated_input: String,
    has_translation_line: bool,
}

impl AppState {
    fn new() -> Self {
        Self {
            input: String::new(),
            cursor_pos: 0,
            translation: String::new(),
            is_translating: false,
            spinner_frame: 0,
            last_input_time: Instant::now(),
            last_translated_input: String::new(),
            has_translation_line: false,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(Mutex::new(AppState::new()));
    let needs_render = Arc::new(AtomicBool::new(true));

    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();

    // Initial render before loop
    {
        let mut state_guard = state.lock().await;
        render(&mut *state_guard, &mut stdout)?;
    }

    // Main loop
    let result = run_loop(state.clone(), needs_render).await;

    // Cleanup
    {
        let state_guard = state.lock().await;
        cleanup(&*state_guard, &mut stdout)?;
    }
    terminal::disable_raw_mode()?;

    // Execute the command
    if let Ok(Some(command)) = result {
        println!("{}", command);
        stdout.flush()?;

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let err = std::process::Command::new("sh")
                .arg("-c")
                .arg(&command)
                .exec();
            eprintln!("exec failed: {}", err);
        }

        #[cfg(not(unix))]
        {
            std::process::Command::new("sh")
                .arg("-c")
                .arg(&command)
                .status()?;
        }
    }

    Ok(())
}

fn cleanup(state: &AppState, stdout: &mut impl Write) -> Result<(), Box<dyn std::error::Error>> {
    execute!(
        stdout,
        MoveToColumn(0),
        Clear(ClearType::CurrentLine),
    )?;

    if state.has_translation_line {
        execute!(
            stdout,
            Print("\n"),
            Clear(ClearType::CurrentLine),
            MoveUp(1),
        )?;
    }

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
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key_event) = event::read()? {
                let mut state_guard = state.lock().await;

                match handle_key_event(key_event, &mut state_guard) {
                    KeyAction::Continue => {}
                    KeyAction::Execute => {
                        let command = state_guard.translation.clone();
                        if !command.is_empty() && !command.starts_with("# ") {
                            return Ok(Some(command));
                        }
                    }
                    KeyAction::Quit => {
                        return Ok(None);
                    }
                }

                needs_render.store(true, Ordering::SeqCst);
            }
        }

        // Check if we need to trigger translation (debounce)
        {
            let mut state_guard = state.lock().await;
            let now = Instant::now();
            let elapsed = now.duration_since(state_guard.last_input_time);

            let trimmed_input = state_guard.input.trim();
            let trimmed_last = state_guard.last_translated_input.trim();

            if elapsed >= Duration::from_millis(DEBOUNCE_MS)
                && !trimmed_input.is_empty()
                && trimmed_input != trimmed_last
                && !state_guard.is_translating
            {
                state_guard.is_translating = true;
                state_guard.last_translated_input = state_guard.input.clone();
                let input = state_guard.input.trim().to_string();
                needs_render.store(true, Ordering::SeqCst);
                drop(state_guard);

                let state_clone = state.clone();
                let client_clone = client.clone();
                let needs_render_clone = needs_render.clone();
                tokio::spawn(async move {
                    let result = translate(&client_clone, &input).await;
                    let mut state_guard = state_clone.lock().await;
                    state_guard.is_translating = false;
                    if let Ok(translation) = result {
                        if state_guard.last_translated_input.trim() == input {
                            state_guard.translation = translation;
                        }
                    }
                    needs_render_clone.store(true, Ordering::SeqCst);
                });
            }
        }

        // Update spinner
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
    Execute,
    Quit,
}

fn handle_key_event(event: KeyEvent, state: &mut AppState) -> KeyAction {
    match event.code {
        KeyCode::Esc => KeyAction::Quit,
        KeyCode::Char('c') if event.modifiers.contains(KeyModifiers::CONTROL) => KeyAction::Quit,
        KeyCode::Enter => KeyAction::Execute,
        KeyCode::Char(c) => {
            state.input.insert(state.cursor_pos, c);
            state.cursor_pos += 1;
            state.last_input_time = Instant::now();
            KeyAction::Continue
        }
        KeyCode::Backspace => {
            if state.cursor_pos > 0 {
                state.cursor_pos -= 1;
                state.input.remove(state.cursor_pos);
                state.last_input_time = Instant::now();
            }
            KeyAction::Continue
        }
        KeyCode::Delete => {
            if state.cursor_pos < state.input.len() {
                state.input.remove(state.cursor_pos);
                state.last_input_time = Instant::now();
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
        KeyCode::Tab => {
            if !state.translation.is_empty() && !state.translation.starts_with("# ") {
                state.input = state.translation.clone();
                state.cursor_pos = state.input.len();
                state.last_input_time = Instant::now();
            }
            KeyAction::Continue
        }
        _ => KeyAction::Continue,
    }
}

fn render(state: &mut AppState, stdout: &mut impl Write) -> Result<(), Box<dyn std::error::Error>> {
    // Clear current line
    execute!(
        stdout,
        MoveToColumn(0),
        Clear(ClearType::CurrentLine),
        Print(PROMPT),
        Print(&state.input),
    )?;

    // Render translation line below if needed
    let show_translation = state.is_translating || !state.translation.is_empty();

    if show_translation {
        execute!(stdout, Print("\r\n"), MoveToColumn(0), Clear(ClearType::CurrentLine))?;

        if state.is_translating {
            let spinner = SPINNER_FRAMES[state.spinner_frame];
            execute!(
                stdout,
                SetForegroundColor(Color::DarkGrey),
                Print(format!("  {} ", spinner)),
                ResetColor,
            )?;
        } else {
            execute!(
                stdout,
                SetForegroundColor(Color::Cyan),
                Print("  "),
                Print(&state.translation),
                ResetColor,
            )?;
        }

        execute!(stdout, MoveUp(1))?;
        state.has_translation_line = true;
    } else if state.has_translation_line {
        execute!(
            stdout,
            Print("\r\n"),
            MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            MoveUp(1),
        )?;
        state.has_translation_line = false;
    }

    // Position cursor
    let cursor_col = PROMPT.len() as u16 + state.cursor_pos as u16;
    execute!(stdout, MoveToColumn(cursor_col))?;

    stdout.flush()?;
    Ok(())
}

async fn translate(client: &reqwest::Client, input: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let request = TranslateRequest {
        input: input.to_string(),
        context: None,
        os: Some(std::env::consts::OS.to_string()),
        shell: std::env::var("SHELL").ok(),
    };

    let response = client
        .post(API_URL)
        .json(&request)
        .send()
        .await?;

    if response.status().is_success() {
        let translate_response: TranslateResponse = response.json().await?;
        Ok(translate_response.command)
    } else {
        Ok("# unable to translate".to_string())
    }
}
