use axum::{
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::time::Duration;
use tracing::{error, info};

mod openrouter;

use jst_shared::{ErrorResponse, TranslateRequest};

#[derive(Clone)]
struct AppState {
    client: reqwest::Client,
    openrouter_api_key: String,
    openrouter_model: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "jst_server=info".into()),
        )
        .init();

    let openrouter_api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY environment variable must be set");
    let openrouter_model = std::env::var("OPENROUTER_MODEL")
        .expect("OPENROUTER_MODEL environment variable must be set");

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(30))
        .build()
        .expect("failed to build OpenRouter client");
    let state = AppState {
        client,
        openrouter_api_key,
        openrouter_model,
    };

    let app = Router::new()
        .route("/", get(health))
        .route("/health", get(health))
        .route("/translate", post(translate))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    info!("Starting jst-server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "ok"
}

async fn translate(
    State(state): State<AppState>,
    Json(req): Json<TranslateRequest>,
) -> impl IntoResponse {
    if let Err(message) = validate_request(&req) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: message.to_string(),
            }),
        )
            .into_response();
    }

    match openrouter::translate(
        &state.client,
        &state.openrouter_api_key,
        &state.openrouter_model,
        &req,
    )
    .await
    {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(error) => {
            error!("Translation error: {error}");
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: "translation provider failed".to_string(),
                }),
            )
                .into_response()
        }
    }
}

fn validate_request(request: &TranslateRequest) -> Result<(), &'static str> {
    if request.input.trim().is_empty() || request.input.len() > 2_000 {
        return Err("request must contain 1–2000 bytes of input");
    }
    if request.os.as_ref().is_some_and(|os| os.len() > 64) {
        return Err("os must not exceed 64 bytes");
    }
    if request
        .shell
        .as_ref()
        .is_some_and(|shell| shell.len() > 256)
    {
        return Err("shell must not exceed 256 bytes");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_request;
    use jst_shared::TranslateRequest;

    fn request(input: &str) -> TranslateRequest {
        TranslateRequest {
            input: input.to_string(),
            os: Some("macos".to_string()),
            shell: Some("/bin/zsh".to_string()),
        }
    }

    #[test]
    fn accepts_normal_requests() {
        assert!(validate_request(&request("find large files")).is_ok());
    }

    #[test]
    fn rejects_empty_and_oversized_requests() {
        assert!(validate_request(&request("   ")).is_err());
        assert!(validate_request(&request(&"x".repeat(2_001))).is_err());
    }

    #[test]
    fn rejects_oversized_metadata() {
        let mut oversized_os = request("pwd");
        oversized_os.os = Some("x".repeat(65));
        assert!(validate_request(&oversized_os).is_err());

        let mut oversized_shell = request("pwd");
        oversized_shell.shell = Some("x".repeat(257));
        assert!(validate_request(&oversized_shell).is_err());
    }
}
