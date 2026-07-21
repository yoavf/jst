use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TranslateRequest {
    pub input: String,
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub shell: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub explain: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<CommandRevision>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranslateResponse {
    pub command: String,
    pub effects: CommandEffects,
    pub matches_request: bool,
    pub explanation: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<CommandPart>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandPart {
    pub fragment: String,
    pub meaning: String,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRevision {
    pub command: String,
    pub instruction: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
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
    use super::{CommandEffects, CommandRevision, TranslateRequest, TranslateResponse};

    fn response(effects: CommandEffects) -> TranslateResponse {
        TranslateResponse {
            command: "example".to_string(),
            effects,
            matches_request: true,
            explanation: String::new(),
            parts: Vec::new(),
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

    #[test]
    fn old_responses_without_parts_remain_compatible() {
        let response = serde_json::from_str::<TranslateResponse>(
            r#"{"command":"pwd","effects":{"reads_data":true,"modifies_data":false,"deletes_data":false,"uses_network":false,"changes_remote_data":false,"changes_processes":false,"installs_software":false,"uses_privilege":false,"executes_remote_code":false},"matches_request":true,"explanation":"Prints the directory."}"#,
        )
        .expect("old response remains valid");

        assert!(response.parts.is_empty());
    }

    #[test]
    fn optional_explanation_fields_preserve_the_old_wire_shape() {
        let ordinary_request = TranslateRequest {
            input: "pwd".to_string(),
            os: None,
            shell: None,
            explain: false,
            revision: None,
        };
        let ordinary_json = serde_json::to_string(&ordinary_request).expect("serializes");
        assert!(!ordinary_json.contains("\"explain\""));
        assert!(!ordinary_json.contains("\"revision\""));

        let explained_request = TranslateRequest {
            explain: true,
            ..ordinary_request
        };

        assert!(serde_json::to_string(&explained_request)
            .expect("serializes")
            .contains("\"explain\":true"));

        let revision = TranslateRequest {
            input: "show files".to_string(),
            os: None,
            shell: None,
            explain: true,
            revision: Some(CommandRevision {
                command: "find .".to_string(),
                instruction: "only include Rust files".to_string(),
                replacement: None,
            }),
        };
        let revision_json = serde_json::to_string(&revision).expect("serializes");
        assert!(revision_json.contains("\"command\":\"find .\""));
        assert!(revision_json.contains("\"instruction\":\"only include Rust files\""));
        assert!(!revision_json.contains("\"replacement\""));

        let manual_revision = TranslateRequest {
            input: "show files".to_string(),
            os: None,
            shell: None,
            explain: true,
            revision: Some(CommandRevision {
                command: "find .".to_string(),
                instruction: String::new(),
                replacement: Some("find . -type f".to_string()),
            }),
        };
        assert!(serde_json::to_string(&manual_revision)
            .expect("serializes")
            .contains("\"replacement\":\"find . -type f\""));

        let ordinary_response = response(CommandEffects::default());
        assert!(!serde_json::to_string(&ordinary_response)
            .expect("serializes")
            .contains("\"parts\""));
    }
}
