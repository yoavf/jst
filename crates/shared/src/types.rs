use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TranslateRequest {
    pub input: String,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub shell: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranslateResponse {
    pub command: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}
