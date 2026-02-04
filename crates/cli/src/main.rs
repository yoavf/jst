use crossterm::{
    cursor::{self, MoveTo},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use jst_shared::{TranslateRequest, TranslateResponse};
use std::io::{stdout, Write};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const API_URL: &str = "https://jst-server.fly.dev/translate";
const DEBOUNCE_MS: u64 = 300;
const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

#[derive(Clone)]
struct AppState {
    input: String,
    cursor_pos: usize,
    translation: String,
    is_translating: bool,
    spinner_frame: usize,
    last_input_time: Instant,
    last_translated_input: String,
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
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(Mutex::new(AppState::new()));

    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Show)?;

    // Initial render
    render(&*state.lock().await, &mut stdout)?;

    // Main loop
    let result = run_loop(state.clone()).await;

    // Cleanup
    terminal::disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen)?;

    // If we have a command to execute, print and run it
    if let Ok(Some(command)) = result {
        println!("$ {}", command);
        std::process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .status()?;
    }

    Ok(())
}

async fn run_loop(state: Arc<Mutex<AppState>>) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut stdout = stdout();
    let client = reqwest::Client::new();
    let mut last_poll = Instant::now();

    loop {
        // Poll for events with a short timeout for responsive spinner
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

                render(&*state_guard, &mut stdout)?;
            }
        }

        // Check if we need to trigger translation (debounce)
        {
            let mut state_guard = state.lock().await;
            let now = Instant::now();
            let elapsed = now.duration_since(state_guard.last_input_time);

            if elapsed >= Duration::from_millis(DEBOUNCE_MS)
                && !state_guard.input.is_empty()
                && state_guard.input != state_guard.last_translated_input
                && !state_guard.is_translating
            {
                state_guard.is_translating = true;
                state_guard.last_translated_input = state_guard.input.clone();
                let input = state_guard.input.clone();
                drop(state_guard);

                // Spawn translation task
                let state_clone = state.clone();
                let client_clone = client.clone();
                tokio::spawn(async move {
                    let result = translate(&client_clone, &input).await;
                    let mut state_guard = state_clone.lock().await;
                    state_guard.is_translating = false;
                    if let Ok(translation) = result {
                        // Only update if input hasn't changed
                        if state_guard.last_translated_input == input {
                            state_guard.translation = translation;
                        }
                    }
                });
            }
        }

        // Update spinner
        if last_poll.elapsed() >= Duration::from_millis(80) {
            let mut state_guard = state.lock().await;
            if state_guard.is_translating {
                state_guard.spinner_frame = (state_guard.spinner_frame + 1) % SPINNER_FRAMES.len();
                render(&*state_guard, &mut stdout)?;
            }
            last_poll = Instant::now();
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
            // Accept translation into input for editing
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

fn render(state: &AppState, stdout: &mut impl Write) -> Result<(), Box<dyn std::error::Error>> {
    // Clear screen and move to top
    execute!(
        stdout,
        MoveTo(0, 0),
        Clear(ClearType::All),
    )?;

    // Render prompt line
    execute!(
        stdout,
        SetForegroundColor(Color::Green),
        Print("› "),
        ResetColor,
        Print(&state.input),
    )?;

    // Render translation line
    execute!(stdout, MoveTo(0, 1))?;

    if state.is_translating {
        let spinner = SPINNER_FRAMES[state.spinner_frame];
        execute!(
            stdout,
            SetForegroundColor(Color::DarkGrey),
            Print(format!("  {} translating...", spinner)),
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

    // Render help line
    execute!(
        stdout,
        MoveTo(0, 3),
        SetForegroundColor(Color::DarkGrey),
        Print("Enter: execute  |  Esc: quit  |  Tab: edit translation"),
        ResetColor,
    )?;

    // Position cursor on input line
    let cursor_col = 2 + state.cursor_pos as u16; // 2 for "› "
    execute!(stdout, MoveTo(cursor_col, 0))?;

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
