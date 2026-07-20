use jst_shared::{build_system_prompt, TranslateRequest, TranslateResponse};
use serde::{Deserialize, Serialize};

const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const MAX_OPENROUTER_RESPONSE_BYTES: usize = 64 * 1024;

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
        max_tokens: 512,
        response_format: ResponseFormat {
            r#type: "json_object".to_string(),
        },
    };

    let response = client
        .post(OPENROUTER_API_URL)
        .bearer_auth(api_key)
        .json(&chat_request)
        .send()
        .await?;

    let status = response.status();
    let body = read_limited_body(response, MAX_OPENROUTER_RESPONSE_BYTES).await?;

    if !status.is_success() {
        return Err(format!("OpenRouter API returned {status}").into());
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
        return Err("OpenRouter response exceeded size limit".into());
    }

    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len() + chunk.len() > limit {
            return Err("OpenRouter response exceeded size limit".into());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8(body)?)
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

    #[test]
    fn handles_short_fences_without_panicking() {
        assert_eq!(strip_code_fence("```"), "");
    }
}
