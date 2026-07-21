use jst_shared::{build_system_prompt, TranslateRequest, TranslateResponse};
use serde::{Deserialize, Serialize};
use tracing::warn;

const MAX_LLM_RESPONSE_BYTES: usize = 16 * 1024;
const MAX_COMMAND_BYTES: usize = 2 * 1024;
const MAX_EXPLANATION_BYTES: usize = 1024;
// Headroom for reasoning-capable models: thinking tokens count against
// max_tokens on some providers, and a small budget truncates the JSON output.
const MAX_OUTPUT_TOKENS: u32 = 2048;

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
    response_format: ResponseFormat,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    r#type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
    finish_reason: Option<String>,
}

pub async fn translate(
    client: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    model: &str,
    fallback_model: Option<&str>,
    req: &TranslateRequest,
) -> Result<TranslateResponse, Box<dyn std::error::Error + Send + Sync>> {
    let primary_error = match translate_with_model(client, api_url, api_key, model, req).await {
        Ok(response) => return Ok(response),
        Err(error) => error,
    };

    let Some(fallback_model) = fallback_model.filter(|fallback| *fallback != model) else {
        return Err(primary_error);
    };

    warn!(
        primary_model = model,
        fallback_model,
        error = %primary_error,
        "primary LLM failed, trying fallback"
    );

    translate_with_model(client, api_url, api_key, fallback_model, req)
        .await
        .map_err(|fallback_error| {
            format!(
                "primary LLM ({model}) failed: {primary_error}; fallback LLM ({fallback_model}) failed: {fallback_error}"
            )
            .into()
        })
}

async fn translate_with_model(
    client: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    model: &str,
    req: &TranslateRequest,
) -> Result<TranslateResponse, Box<dyn std::error::Error + Send + Sync>> {
    let max_retries = 1;
    for attempt in 0..=max_retries {
        match call_llm(client, api_url, api_key, model, req).await {
            Ok(response) => return Ok(response),
            Err((error, finish_reason)) if finish_reason.as_deref() == Some("error") => {
                if attempt < max_retries {
                    warn!(
                        attempt,
                        error = %error,
                        "LLM returned finish_reason=error, retrying"
                    );
                    continue;
                }
                return Err(error);
            }
            Err((error, _)) => return Err(error),
        }
    }
    unreachable!()
}

async fn call_llm(
    client: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    model: &str,
    req: &TranslateRequest,
) -> Result<TranslateResponse, (Box<dyn std::error::Error + Send + Sync>, Option<String>)> {
    let system_prompt = build_system_prompt(req.os.as_deref(), req.shell.as_deref());

    let chat_request = ChatRequest {
        model: model.to_string(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: system_prompt,
            },
            Message {
                role: "user".to_string(),
                content: req.input.clone(),
            },
        ],
        temperature: 0.0,
        max_tokens: MAX_OUTPUT_TOKENS,
        response_format: ResponseFormat {
            r#type: "json_object".to_string(),
        },
    };

    let request = client.post(api_url).json(&chat_request);
    let request = if api_key.is_empty() {
        request
    } else {
        request.bearer_auth(api_key)
    };
    let response = request.send().await.map_err(|e| (e.into(), None))?;

    let status = response.status();
    let body = read_limited_body(response, MAX_LLM_RESPONSE_BYTES)
        .await
        .map_err(|e| (e, None))?;

    if !status.is_success() {
        return Err((format!("LLM API returned {status}").into(), None));
    }

    let chat_response: ChatResponse = serde_json::from_str(&body).map_err(|e| (e.into(), None))?;

    let choice = chat_response
        .choices
        .first()
        .ok_or(("LLM API returned no choices".into(), None))?;
    let content = strip_code_fence(choice.message.content.trim());

    serde_json::from_str::<TranslateResponse>(content)
        .map_err(|error| {
            let finish_reason = choice.finish_reason.clone();
            warn!(
                finish_reason = ?choice.finish_reason,
                content_length = content.len(),
                content_preview = %content.chars().take(200).collect::<String>(),
                "failed to parse LLM response"
            );
            (
                format!(
                    "failed to parse model response (finish_reason: {:?}): {error}",
                    finish_reason
                )
                .into(),
                finish_reason,
            )
        })
        .and_then(|response| {
            validate_translation_response(&response)
                .map_err(|e| (e.into(), choice.finish_reason.clone()))?;
            Ok(response)
        })
}

fn validate_translation_response(response: &TranslateResponse) -> Result<(), &'static str> {
    if response.command.is_empty() || response.command.len() > MAX_COMMAND_BYTES {
        return Err("LLM command exceeded size limit");
    }
    if response.explanation.len() > MAX_EXPLANATION_BYTES {
        return Err("LLM explanation exceeded size limit");
    }
    Ok(())
}

fn strip_code_fence(content: &str) -> &str {
    let content = content.trim();
    if content == "```" {
        return "";
    }

    let Some(inner) = content
        .strip_prefix("```")
        .and_then(|inner| inner.strip_suffix("```"))
    else {
        return content;
    };

    inner.strip_prefix("json\n").unwrap_or(inner).trim()
}

async fn read_limited_body(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err("LLM response exceeded size limit".into());
    }

    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len() + chunk.len() > limit {
            return Err("LLM response exceeded size limit".into());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8(body)?)
}

#[cfg(test)]
mod tests {
    use super::{strip_code_fence, translate, validate_translation_response};
    use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
    use jst_shared::{CommandEffects, TranslateRequest, TranslateResponse};
    use serde_json::{json, Value};
    use std::sync::{Arc, Mutex};

    fn response(command: String, explanation: String) -> TranslateResponse {
        TranslateResponse {
            command,
            effects: CommandEffects::default(),
            matches_request: true,
            explanation,
        }
    }

    #[test]
    fn strips_json_code_fences() {
        assert_eq!(
            strip_code_fence("```json\n{\"command\":\"pwd\"}\n```"),
            r#"{"command":"pwd"}"#
        );
    }

    #[test]
    fn preserves_plain_json() {
        assert_eq!(
            strip_code_fence(r#"{"command":"pwd"}"#),
            r#"{"command":"pwd"}"#
        );
    }

    #[test]
    fn handles_short_fences_without_panicking() {
        assert_eq!(strip_code_fence("```"), "");
    }

    #[test]
    fn rejects_oversized_translation_fields() {
        assert!(validate_translation_response(&response("pwd".to_string(), String::new())).is_ok());
        assert!(validate_translation_response(&response(String::new(), String::new())).is_err());
        assert!(
            validate_translation_response(&response("x".repeat(2 * 1024 + 1), String::new()))
                .is_err()
        );
        assert!(
            validate_translation_response(&response("pwd".to_string(), "x".repeat(1025))).is_err()
        );
    }

    #[tokio::test]
    async fn falls_back_to_the_configured_model_when_the_primary_is_unavailable() {
        async fn mock_completion(
            State(seen_models): State<Arc<Mutex<Vec<String>>>>,
            Json(request): Json<Value>,
        ) -> (StatusCode, Json<Value>) {
            let model = request["model"]
                .as_str()
                .expect("request model")
                .to_string();
            seen_models.lock().expect("model log").push(model.clone());

            if model == "granite" {
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(json!({ "error": "model unavailable" })),
                );
            }

            let content = json!({
                "command": "pwd",
                "effects": {
                    "reads_data": true,
                    "modifies_data": false,
                    "deletes_data": false,
                    "uses_network": false,
                    "changes_remote_data": false,
                    "changes_processes": false,
                    "installs_software": false,
                    "uses_privilege": false,
                    "executes_remote_code": false
                },
                "matches_request": true,
                "explanation": "Shows the current directory."
            })
            .to_string();
            (
                StatusCode::OK,
                Json(json!({
                    "choices": [{
                        "message": { "role": "assistant", "content": content },
                        "finish_reason": "stop"
                    }]
                })),
            )
        }

        let seen_models = Arc::new(Mutex::new(Vec::new()));
        let app = Router::new()
            .route("/chat/completions", post(mock_completion))
            .with_state(seen_models.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock LLM");
        let address = listener.local_addr().expect("mock LLM address");
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve mock LLM");
        });

        let request = TranslateRequest {
            input: "show the current directory".to_string(),
            os: Some("macos".to_string()),
            shell: Some("zsh".to_string()),
        };
        let response = translate(
            &reqwest::Client::new(),
            &format!("http://{address}/chat/completions"),
            "",
            "granite",
            Some("gemini"),
            &request,
        )
        .await
        .expect("fallback translation");

        assert_eq!(response.command, "pwd");
        assert_eq!(
            *seen_models.lock().expect("model log"),
            vec!["granite", "gemini"]
        );
        server.abort();
    }
}
