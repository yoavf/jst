use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TranslateRequest {
    pub input: String,
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub shell: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranslateResponse {
    pub command: String,
    pub effects: CommandEffects,
    pub matches_request: bool,
    pub explanation: String,
}

impl TranslateResponse {
    pub fn model_warnings(&self) -> Vec<&'static str> {
        let mut warnings = Vec::new();

        if !self.matches_request {
            warnings.push("The generated command may not match your request.");
        }
        if self.effects.deletes_data {
            warnings.push("The generated command may delete data.");
        }
        if self.effects.changes_remote_data {
            warnings.push("The generated command may change remote data.");
        }
        if self.effects.changes_processes {
            warnings.push("The generated command may start, stop, or alter processes.");
        }
        if self.effects.installs_software {
            warnings.push("The generated command may install or remove software.");
        }
        if self.effects.uses_privilege {
            warnings.push("The generated command may use elevated privileges.");
        }
        if self.effects.executes_remote_code {
            warnings.push("The generated command may execute downloaded code.");
        }

        warnings
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CommandEffects {
    pub reads_data: bool,
    pub modifies_data: bool,
    pub deletes_data: bool,
    pub uses_network: bool,
    pub changes_remote_data: bool,
    pub changes_processes: bool,
    pub installs_software: bool,
    pub uses_privilege: bool,
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
            matches_request: true,
            explanation: String::new(),
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
        assert!(response.model_warnings().is_empty());
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
            assert!(!response(effects).model_warnings().is_empty());
        }
    }

    #[test]
    fn request_mismatch_requires_confirmation() {
        let mut response = response(CommandEffects::default());
        response.matches_request = false;
        assert!(!response.model_warnings().is_empty());
    }

    #[test]
    fn reports_all_dangerous_effects() {
        let mut response = response(CommandEffects {
            deletes_data: true,
            changes_remote_data: true,
            uses_privilege: true,
            ..CommandEffects::default()
        });
        response.matches_request = false;

        assert_eq!(response.model_warnings().len(), 4);
    }

    #[test]
    fn incomplete_model_responses_fail_closed() {
        assert!(serde_json::from_str::<TranslateResponse>(r#"{"command":"pwd"}"#).is_err());
        assert!(serde_json::from_str::<TranslateResponse>(
            r#"{"command":"pwd","effects":{},"matches_request":true,"explanation":"Prints the directory."}"#
        )
        .is_err());
    }
}
