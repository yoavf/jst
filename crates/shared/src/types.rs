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
    #[serde(default)]
    pub effects: CommandEffects,
    #[serde(default)]
    pub matches_request: Option<bool>,
    #[serde(default)]
    pub explanation: Option<String>,
}

impl TranslateResponse {
    pub fn model_warning(&self) -> Option<&'static str> {
        if self.matches_request == Some(false) {
            return Some("The generated command may not match your request.");
        }

        if self.effects.deletes_data {
            return Some("The generated command may delete data.");
        }
        if self.effects.changes_remote_data {
            return Some("The generated command may change remote data.");
        }
        if self.effects.changes_processes {
            return Some("The generated command may start, stop, or alter processes.");
        }
        if self.effects.installs_software {
            return Some("The generated command may install or remove software.");
        }
        if self.effects.uses_privilege {
            return Some("The generated command may use elevated privileges.");
        }
        if self.effects.executes_remote_code {
            return Some("The generated command may execute downloaded code.");
        }

        None
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CommandEffects {
    #[serde(default)]
    pub reads_data: bool,
    #[serde(default)]
    pub modifies_data: bool,
    #[serde(default)]
    pub deletes_data: bool,
    #[serde(default)]
    pub uses_network: bool,
    #[serde(default)]
    pub changes_remote_data: bool,
    #[serde(default)]
    pub changes_processes: bool,
    #[serde(default)]
    pub installs_software: bool,
    #[serde(default)]
    pub uses_privilege: bool,
    #[serde(default)]
    pub executes_remote_code: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[cfg(test)]
mod tests {
    use super::{CommandEffects, TranslateResponse};

    fn response(effects: CommandEffects) -> TranslateResponse {
        TranslateResponse {
            command: "example".to_string(),
            effects,
            matches_request: Some(true),
            explanation: None,
        }
    }

    #[test]
    fn ordinary_reads_and_writes_do_not_require_confirmation() {
        let response = response(CommandEffects {
            reads_data: true,
            modifies_data: true,
            uses_network: true,
            ..CommandEffects::default()
        });
        assert_eq!(response.model_warning(), None);
    }

    #[test]
    fn dangerous_effects_require_confirmation() {
        let cases = [
            CommandEffects {
                deletes_data: true,
                ..CommandEffects::default()
            },
            CommandEffects {
                changes_remote_data: true,
                ..CommandEffects::default()
            },
            CommandEffects {
                changes_processes: true,
                ..CommandEffects::default()
            },
            CommandEffects {
                installs_software: true,
                ..CommandEffects::default()
            },
            CommandEffects {
                uses_privilege: true,
                ..CommandEffects::default()
            },
            CommandEffects {
                executes_remote_code: true,
                ..CommandEffects::default()
            },
        ];

        for effects in cases {
            assert!(response(effects).model_warning().is_some());
        }
    }

    #[test]
    fn request_mismatch_requires_confirmation() {
        let mut response = response(CommandEffects::default());
        response.matches_request = Some(false);
        assert!(response.model_warning().is_some());
    }

    #[test]
    fn older_server_responses_remain_compatible() {
        let response: TranslateResponse =
            serde_json::from_str(r#"{"command":"pwd"}"#).expect("valid response");
        assert_eq!(response.command, "pwd");
        assert_eq!(response.model_warning(), None);
    }
}
