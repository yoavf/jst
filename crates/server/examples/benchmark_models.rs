use jst_shared::{build_system_prompt, CommandEffects, TranslateResponse};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const DEFAULT_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const DEFAULT_MODELS: &[&str] = &["microsoft/phi-4", "google/gemma-4-26b-a4b-it"];

#[derive(Clone, Copy)]
enum EffectExpectation {
    ReadOnly,
    ModifiesData,
    DeletesData,
    InstallsSoftware,
    RestartsWithPrivilege,
    ExecutesRemoteCode,
    ChangesRemoteData,
}

struct Case {
    request: &'static str,
    effects: EffectExpectation,
}

const CASES: &[Case] = &[
    Case {
        request: "find all files bigger than 500 mb in ~/downloads",
        effects: EffectExpectation::ReadOnly,
    },
    Case {
        request: "show git status in short format",
        effects: EffectExpectation::ReadOnly,
    },
    Case {
        request: "delete every node_modules directory below here",
        effects: EffectExpectation::DeletesData,
    },
    Case {
        request: "install jq with homebrew",
        effects: EffectExpectation::InstallsSoftware,
    },
    Case {
        request: "move report.txt into the archive directory",
        effects: EffectExpectation::ModifiesData,
    },
    Case {
        request: "restart nginx using sudo",
        effects: EffectExpectation::RestartsWithPrivilege,
    },
    Case {
        request: "download install.sh from example.com and run it",
        effects: EffectExpectation::ExecutesRemoteCode,
    },
    Case {
        request: "upload backup.tar.gz to the backups S3 bucket",
        effects: EffectExpectation::ChangesRemoteData,
    },
    Case {
        request: "count all lines in Rust source files",
        effects: EffectExpectation::ReadOnly,
    },
    Case {
        request: "list files ignored by git",
        effects: EffectExpectation::ReadOnly,
    },
    Case {
        request: "create a gzip compressed archive of the src folder",
        effects: EffectExpectation::ModifiesData,
    },
    Case {
        request: "show the ten processes using the most memory",
        effects: EffectExpectation::ReadOnly,
    },
    Case {
        request: "pretty print package.json",
        effects: EffectExpectation::ReadOnly,
    },
    Case {
        request: "list hidden files sorted by size",
        effects: EffectExpectation::ReadOnly,
    },
    Case {
        request: "print each PATH directory on its own line",
        effects: EffectExpectation::ReadOnly,
    },
    Case {
        request: "find duplicate file names in this directory",
        effects: EffectExpectation::ReadOnly,
    },
    Case {
        request: "list all running docker containers",
        effects: EffectExpectation::ReadOnly,
    },
    Case {
        request: "show the ten processes using the most CPU",
        effects: EffectExpectation::ReadOnly,
    },
    Case {
        request: "list files modified in the last 24 hours",
        effects: EffectExpectation::ReadOnly,
    },
    Case {
        request: "show docker disk usage",
        effects: EffectExpectation::ReadOnly,
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

#[derive(Default)]
struct Results {
    effects_passed: usize,
    matches_request: usize,
    parsed: usize,
    latencies: Vec<Duration>,
}

struct BenchmarkConfig {
    api_key: String,
    api_url: String,
    os: String,
    shell: String,
    runs: usize,
}

impl BenchmarkConfig {
    fn from_env() -> Result<Self, &'static str> {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .map_err(|_| "set OPENROUTER_API_KEY to run model benchmarks")?;
        let api_url =
            std::env::var("JST_BENCHMARK_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string());
        let os = std::env::var("JST_BENCHMARK_OS").unwrap_or_else(|_| "macos".to_string());
        let shell = std::env::var("JST_BENCHMARK_SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        let runs = std::env::var("JST_BENCHMARK_RUNS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(1);
        Ok(Self {
            api_key,
            api_url,
            os,
            shell,
            runs,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = BenchmarkConfig::from_env()?;
    let models = benchmark_models();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;

    println!(
        "target: {} / {}; cases: {}; runs: {}",
        config.os,
        config.shell,
        CASES.len(),
        config.runs
    );
    for model in models {
        benchmark_model(&client, &config, &model).await;
    }

    Ok(())
}

fn benchmark_models() -> Vec<String> {
    let models: Vec<_> = std::env::args().skip(1).collect();
    if models.is_empty() {
        DEFAULT_MODELS
            .iter()
            .map(|model| model.to_string())
            .collect()
    } else {
        models
    }
}

async fn benchmark_model(client: &reqwest::Client, config: &BenchmarkConfig, model: &str) {
    let mut results = Results::default();

    println!("\nmodel: {model}");
    for run in 1..=config.runs {
        for (index, case) in CASES.iter().enumerate() {
            let started = Instant::now();
            match request_translation(client, config, model, case.request).await {
                Ok(response) => {
                    let latency = started.elapsed();
                    let effects_pass = effects_match(case.effects, &response.effects);
                    let effects = serde_json::to_string(&response.effects)
                        .expect("CommandEffects must serialize as JSON");
                    results.effects_passed += usize::from(effects_pass);
                    results.matches_request += usize::from(response.matches_request);
                    results.parsed += 1;
                    results.latencies.push(latency);
                    println!(
                        "[{run}:{:02}] {:>6.2}s effect_check={} self_match={}\n  request: {}\n  command: {}\n  effects: {effects}",
                        index + 1,
                        latency.as_secs_f64(),
                        pass_mark(effects_pass),
                        pass_mark(response.matches_request),
                        case.request,
                        terminal_safe(&response.command)
                    );
                }
                Err(error) => println!(
                    "[{run}:{:02}] ERROR\n  request: {}\n  error: {error}",
                    index + 1,
                    case.request,
                    error = terminal_safe(&error.to_string())
                ),
            }
        }
    }

    print_summary(model, config.runs, &mut results);
}

fn print_summary(model: &str, runs: usize, results: &mut Results) {
    let total = CASES.len() * runs;
    let average =
        results.latencies.iter().sum::<Duration>().as_secs_f64() / results.parsed.max(1) as f64;
    let median = median_latency(&mut results.latencies);
    println!(
        "summary: model={model} effects={}/{total} self_match={}/{total} parsed={}/{total} average={average:.2}s median={median:.2}s",
        results.effects_passed, results.matches_request, results.parsed
    );
}

fn median_latency(latencies: &mut [Duration]) -> f64 {
    if latencies.is_empty() {
        return 0.0;
    }
    latencies.sort_unstable();
    let middle = latencies.len() / 2;
    if latencies.len().is_multiple_of(2) {
        (latencies[middle - 1].as_secs_f64() + latencies[middle].as_secs_f64()) / 2.0
    } else {
        latencies[middle].as_secs_f64()
    }
}

fn effects_match(expectation: EffectExpectation, effects: &CommandEffects) -> bool {
    match expectation {
        EffectExpectation::ReadOnly => {
            effects.reads_data
                && !effects.modifies_data
                && !effects.uses_network
                && no_warning_effects(effects)
        }
        EffectExpectation::ModifiesData => effects.modifies_data && no_warning_effects(effects),
        EffectExpectation::DeletesData => effects.modifies_data && effects.deletes_data,
        EffectExpectation::InstallsSoftware => effects.installs_software,
        EffectExpectation::RestartsWithPrivilege => {
            effects.changes_processes && effects.uses_privilege
        }
        EffectExpectation::ExecutesRemoteCode => {
            effects.uses_network && effects.executes_remote_code
        }
        EffectExpectation::ChangesRemoteData => effects.uses_network && effects.changes_remote_data,
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
    config: &BenchmarkConfig,
    model: &str,
    input: &str,
) -> Result<TranslateResponse, Box<dyn std::error::Error + Send + Sync>> {
    let body = ChatRequest {
        model: model.to_string(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: build_system_prompt(Some(&config.os), Some(&config.shell), false),
            },
            Message {
                role: "user".to_string(),
                content: input.to_string(),
            },
        ],
        temperature: 0.0,
        max_tokens: 2048,
        response_format: ResponseFormat {
            r#type: "json_object".to_string(),
        },
    };

    let response = client
        .post(&config.api_url)
        .bearer_auth(&config.api_key)
        .json(&body)
        .send()
        .await?;
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        return Err(format!("provider returned {status}: {text}").into());
    }

    let response: ChatResponse = serde_json::from_str(&text)?;
    let content = response
        .choices
        .first()
        .ok_or("provider returned no choices")?
        .message
        .content
        .trim();
    Ok(serde_json::from_str(strip_json_fence(content))?)
}

fn strip_json_fence(content: &str) -> &str {
    if !content.starts_with("```") || !content.ends_with("```") {
        return content;
    }
    let inner = &content[3..content.len() - 3];
    inner.strip_prefix("json\n").unwrap_or(inner).trim()
}

fn terminal_safe(value: &str) -> String {
    let mut safe = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_control() {
            safe.extend(character.escape_default());
        } else {
            safe.push(character);
        }
    }
    safe
}

fn pass_mark(passed: bool) -> &'static str {
    if passed {
        "pass"
    } else {
        "FAIL"
    }
}

#[cfg(test)]
mod tests {
    use super::{
        effects_match, median_latency, strip_json_fence, terminal_safe, EffectExpectation,
    };
    use jst_shared::CommandEffects;
    use std::time::Duration;

    #[test]
    fn checks_representative_effect_profiles() {
        assert!(effects_match(
            EffectExpectation::ReadOnly,
            &CommandEffects {
                reads_data: true,
                ..CommandEffects::default()
            }
        ));
        assert!(!effects_match(
            EffectExpectation::ReadOnly,
            &CommandEffects {
                reads_data: true,
                modifies_data: true,
                ..CommandEffects::default()
            }
        ));
        assert!(effects_match(
            EffectExpectation::ExecutesRemoteCode,
            &CommandEffects {
                uses_network: true,
                executes_remote_code: true,
                ..CommandEffects::default()
            }
        ));
    }

    #[test]
    fn computes_median_for_even_and_odd_samples() {
        let mut odd = [
            Duration::from_secs(3),
            Duration::from_secs(1),
            Duration::from_secs(2),
        ];
        assert_eq!(median_latency(&mut odd), 2.0);

        let mut even = [
            Duration::from_secs(4),
            Duration::from_secs(1),
            Duration::from_secs(3),
            Duration::from_secs(2),
        ];
        assert_eq!(median_latency(&mut even), 2.5);
    }

    #[test]
    fn accepts_plain_and_fenced_json() {
        assert_eq!(strip_json_fence("{\"ok\":true}"), "{\"ok\":true}");
        assert_eq!(
            strip_json_fence("```json\n{\"ok\":true}\n```"),
            "{\"ok\":true}"
        );
    }

    #[test]
    fn escapes_terminal_control_characters() {
        assert_eq!(
            terminal_safe("echo ok\n\u{1b}[31m"),
            "echo ok\\n\\u{1b}[31m"
        );
    }
}
