use jst_shared::{build_system_prompt, TranslateRequest, TranslateResponse};
use serde::{Deserialize, Serialize};

const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

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
}

pub async fn translate(
    client: &reqwest::Client,
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
        max_tokens: 256,
        response_format: ResponseFormat {
            r#type: "json_object".to_string(),
        },
    };

    if dev_log_enabled() {
        if let Ok(payload) = serde_json::to_string_pretty(&chat_request) {
            tracing::info!("JST_DEV_LOG outbound OpenRouter request:\n{}", payload);
        }
    }

    let response = client
        .post(OPENROUTER_API_URL)
        .bearer_auth(api_key)
        .json(&chat_request)
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if dev_log_enabled() {
        tracing::info!(
            "JST_DEV_LOG inbound OpenRouter response (status={}):\n{}",
            status,
            body
        );
    }

    if !status.is_success() {
        return Err(format!("OpenRouter API error: {} - {}", status, body).into());
    }

    let chat_response: ChatResponse = serde_json::from_str(&body)?;

    let content = chat_response
        .choices
        .first()
        .map(|choice| choice.message.content.trim())
        .ok_or("OpenRouter returned no choices")?;
    let content = strip_code_fence(content);

    Ok(serde_json::from_str(content)?)
}

fn strip_code_fence(content: &str) -> &str {
    let content = content.trim();
    if !content.starts_with("```") || !content.ends_with("```") {
        return content;
    }

    let inner = &content[3..content.len() - 3];
    inner.strip_prefix("json\n").unwrap_or(inner).trim()
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

#[cfg(test)]
mod tests {
    use super::strip_code_fence;

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
}
