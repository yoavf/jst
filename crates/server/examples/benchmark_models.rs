use jst_shared::{build_system_prompt, CommandEffects, TranslateRequest, TranslateResponse};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const DEFAULT_MODELS: &[&str] = &[
    "google/gemini-2.5-flash-lite",
    "ibm-granite/granite-4.1-8b",
    "openai/gpt-4.1-nano",
    "meta-llama/llama-4-scout",
    "microsoft/phi-4",
    "cohere/command-r7b-12-2024",
    "amazon/nova-lite-v1",
    "ibm-granite/granite-4.0-h-micro",
    "mistralai/codestral-2508",
];

struct Case {
    input: &'static str,
    command_terms: &'static [&'static str],
    forbidden_terms: &'static [&'static str],
}

const CASES: &[Case] = &[
    Case {
        input: "find all files bigger than 500 mb in ~/downloads",
        command_terms: &["find", "-size", "+500"],
        forbidden_terms: &["-exec", "-delete", ">", "&&", "$("],
    },
    Case {
        input: "show git status in short format",
        command_terms: &["git", "status"],
        forbidden_terms: &[],
    },
    Case {
        input: "delete every node_modules directory below here",
        command_terms: &["find", "node_modules"],
        forbidden_terms: &[],
    },
    Case {
        input: "install jq with homebrew",
        command_terms: &["brew", "install", "jq"],
        forbidden_terms: &[],
    },
    Case {
        input: "move report.txt into the archive directory",
        command_terms: &["mv", "report.txt", "archive"],
        forbidden_terms: &["&&"],
    },
    Case {
        input: "restart nginx using sudo",
        command_terms: &["nginx"],
        forbidden_terms: &[],
    },
    Case {
        input: "download install.sh from example.com and run it",
        command_terms: &["example.com", "install.sh"],
        forbidden_terms: &[],
    },
    Case {
        input: "upload backup.tar.gz to the backups S3 bucket",
        command_terms: &["aws", "s3", "backup.tar.gz"],
        forbidden_terms: &[],
    },
];

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
    response_format: ResponseFormat,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ResponseFormat {
    r#type: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| "set OPENROUTER_API_KEY to run model benchmarks")?;
    let models = std::env::var("JST_BENCHMARK_MODELS")
        .ok()
        .map(|models| {
            models
                .split(',')
                .map(str::trim)
                .filter(|model| !model.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            DEFAULT_MODELS
                .iter()
                .map(|model| model.to_string())
                .collect()
        });
    let runs = std::env::var("JST_BENCHMARK_RUNS")
        .ok()
        .and_then(|runs| runs.parse::<usize>().ok())
        .unwrap_or(1);

    let client = reqwest::Client::new();
    for model in models {
        benchmark_model(&client, &api_key, &model, runs).await;
    }

    Ok(())
}

async fn benchmark_model(client: &reqwest::Client, api_key: &str, model: &str, runs: usize) {
    let mut command_passes = 0;
    let mut effect_passes = 0;
    let mut successes = 0;
    let mut total_latency = Duration::ZERO;

    println!("\n{model}");
    for _ in 0..runs {
        for (case_index, case) in CASES.iter().enumerate() {
            let started = Instant::now();
            match request_translation(client, api_key, model, case.input).await {
                Ok(response) => {
                    let latency = started.elapsed();
                    total_latency += latency;
                    successes += 1;
                    let command = response.command.to_ascii_lowercase();
                    let command_pass = case
                        .command_terms
                        .iter()
                        .all(|term| command.contains(&term.to_ascii_lowercase()))
                        && case
                            .forbidden_terms
                            .iter()
                            .all(|term| !command.contains(&term.to_ascii_lowercase()));
                    let effect_pass = effects_match_case(case_index, &response.effects);
                    command_passes += usize::from(command_pass);
                    effect_passes += usize::from(effect_pass);
                    println!(
                        "  {:>6.2}s command={} effects={}  {}",
                        latency.as_secs_f64(),
                        pass_mark(command_pass),
                        pass_mark(effect_pass),
                        response.command
                    );
                }
                Err(error) => println!("  ERROR  {error}"),
            }
        }
    }

    let total_cases = CASES.len() * runs;
    let average = if successes == 0 {
        0.0
    } else {
        total_latency.as_secs_f64() / successes as f64
    };
    println!(
        "  score: command {command_passes}/{total_cases}, effects {effect_passes}/{total_cases}, parsed {successes}/{total_cases}, average {average:.2}s"
    );
}

fn effects_match_case(case_index: usize, effects: &CommandEffects) -> bool {
    match case_index {
        0 => effects.reads_data && !effects.modifies_data && no_warning_effects(effects),
        1 => effects.reads_data && no_warning_effects(effects),
        2 => effects.modifies_data && effects.deletes_data,
        3 => effects.installs_software,
        4 => effects.modifies_data && no_warning_effects(effects),
        5 => effects.changes_processes && effects.uses_privilege,
        6 => effects.uses_network && effects.executes_remote_code,
        7 => effects.uses_network && effects.changes_remote_data,
        _ => false,
    }
}

fn no_warning_effects(effects: &CommandEffects) -> bool {
    !effects.deletes_data
        && !effects.changes_remote_data
        && !effects.changes_processes
        && !effects.installs_software
        && !effects.uses_privilege
        && !effects.executes_remote_code
}

async fn request_translation(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    input: &str,
) -> Result<TranslateResponse, Box<dyn std::error::Error + Send + Sync>> {
    let request = TranslateRequest {
        input: input.to_string(),
        os: Some("macos".to_string()),
        shell: Some("/bin/zsh".to_string()),
        explain: false,
        revision: None,
    };
    let body = ChatRequest {
        model: model.to_string(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: build_system_prompt(
                    request.os.as_deref(),
                    request.shell.as_deref(),
                    request.explain,
                ),
            },
            Message {
                role: "user".to_string(),
                content: request.input,
            },
        ],
        temperature: 0.0,
        max_tokens: 2048,
        response_format: ResponseFormat {
            r#type: "json_object".to_string(),
        },
    };

    let response = client
        .post(OPENROUTER_API_URL)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?;
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        return Err(format!("OpenRouter returned {status}: {text}").into());
    }

    let response: ChatResponse = serde_json::from_str(&text)?;
    let content = response
        .choices
        .first()
        .ok_or("OpenRouter returned no choices")?
        .message
        .content
        .trim();
    let content = strip_json_fence(content);
    Ok(serde_json::from_str(content)?)
}

fn strip_json_fence(content: &str) -> &str {
    if !content.starts_with("```") || !content.ends_with("```") {
        return content;
    }
    let inner = &content[3..content.len() - 3];
    inner.strip_prefix("json\n").unwrap_or(inner).trim()
}

fn pass_mark(passed: bool) -> &'static str {
    if passed {
        "pass"
    } else {
        "FAIL"
    }
}
