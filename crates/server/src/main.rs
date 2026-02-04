use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;

mod mistral;

use jst_shared::{ErrorResponse, TranslateRequest, TranslateResponse};

struct AppState {
    mistral_api_key: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "jst_server=info".into()),
        )
        .init();

    let mistral_api_key = std::env::var("MISTRAL_API_KEY")
        .expect("MISTRAL_API_KEY environment variable must be set");

    let state = Arc::new(AppState { mistral_api_key });

    let app = Router::new()
        .route("/", get(health))
        .route("/health", get(health))
        .route("/translate", post(translate))
        .layer(CorsLayer::permissive())
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
    State(state): State<Arc<AppState>>,
    Json(req): Json<TranslateRequest>,
) -> impl IntoResponse {
    info!("Translate request: {:?}", req.input);

    match mistral::translate(&state.mistral_api_key, &req).await {
        Ok(command) => {
            info!("Translated to: {:?}", command);
            (StatusCode::OK, Json(TranslateResponse { command })).into_response()
        }
        Err(e) => {
            tracing::error!("Translation error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
                .into_response()
        }
    }
}
