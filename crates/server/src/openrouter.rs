use jst_shared::{build_system_prompt, TranslateRequest};
use serde::{Deserialize, Serialize};

const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
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
    api_key: &str,
    model: &str,
    req: &TranslateRequest,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();

    let system_prompt = build_system_prompt(
        req.context.as_deref(),
        req.os.as_deref(),
        req.shell.as_deref(),
    );

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
        max_tokens: 500,
    };

    if dev_log_enabled() {
        if let Ok(payload) = serde_json::to_string_pretty(&chat_request) {
            tracing::info!("JST_DEV_LOG outbound OpenRouter request:\n{}", payload);
        }
    }

    let response = client
        .post(OPENROUTER_API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
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

    let command = chat_response
        .choices
        .first()
        .map(|c| c.message.content.trim().to_string())
        .unwrap_or_else(|| "# unable to translate".to_string());

    Ok(command)
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
