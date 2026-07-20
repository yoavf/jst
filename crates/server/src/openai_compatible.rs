use jst_shared::{build_system_prompt, TranslateRequest, TranslateResponse};
use serde::{Deserialize, Serialize};

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
    req: &TranslateRequest,
) -> Result<TranslateResponse, Box<dyn std::error::Error + Send + Sync>> {
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
    let response = request.send().await?;

    let status = response.status();
    let body = read_limited_body(response, MAX_LLM_RESPONSE_BYTES).await?;

    if !status.is_success() {
        return Err(format!("LLM API returned {status}").into());
    }

    let chat_response: ChatResponse = serde_json::from_str(&body)?;

    let choice = chat_response
        .choices
        .first()
        .ok_or("LLM API returned no choices")?;
    let content = strip_code_fence(choice.message.content.trim());

    let response = serde_json::from_str(content).map_err(|error| {
        format!(
            "failed to parse model response (finish_reason: {:?}): {error}",
            choice.finish_reason
        )
    })?;
    validate_translation_response(&response)?;
    Ok(response)
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
    use super::{strip_code_fence, validate_translation_response};
    use jst_shared::{CommandEffects, TranslateResponse};

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
}
