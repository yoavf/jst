pub fn build_system_prompt(os: Option<&str>, shell: Option<&str>, explain: bool) -> String {
    let mut prompt = String::from(
        r##"Translate a natural-language request into one executable shell command and describe its concrete effects.

Rules:
- Output only valid JSON matching the schema below. No markdown or extra text.
- If the input is ambiguous, output the most likely intended command.
- If the input is already a valid command, output it as-is.
- For loops and multi-command operations, use proper shell syntax.
- Prefer common, portable commands when possible.
- Set matches_request false if the command does not accurately implement the request.
- Describe effects literally; do not make a subjective risk judgment.
- changes_processes means managing long-running processes or services; starting this command itself does not count.
- changes_remote_data means uploading, pushing, posting, or deleting remote data; network reads do not count.
- installs_software includes installing, upgrading, or removing software and packages.
- uses_privilege means sudo, administrator, root, or equivalent elevation.
- executes_remote_code means code is downloaded and executed without a separate review step.
- If translation is impossible, set command to "# unable to translate".
- A revision request contains an original_request, current_command, and requested_change.
- For revisions, preserve the original request and everything already implemented by current_command except what requested_change explicitly alters.
- Return a complete replacement command for every revision and recalculate every effect from that replacement.
"##,
    );

    if explain {
        prompt.push_str(
            r##"
- parts is mandatory and must contain a semantic breakdown with 1 to 8 items.
- List parts in command order. Their fragments, concatenated exactly, must reproduce command.
- Copy every fragment verbatim from command, including spaces and shell operators.
- meaning must briefly explain what that fragment contributes, using plain language.
- source must be copied verbatim from the user's request when that wording maps to the fragment; use an empty string for shell syntax with no direct source phrase.
- Explain non-obvious flags, pipes, chaining, redirects, quoting, and globbing without splitting the command into noisy single-character parts.
- explanation must also be a useful standalone fallback: explain every command stage and all non-obvious flags in order, even though the same information appears in parts.

Required shape:
{"command":"du -ah . | sort -hr | head -n 10","effects":{"reads_data":true,"modifies_data":false,"deletes_data":false,"uses_network":false,"changes_remote_data":false,"changes_processes":false,"installs_software":false,"uses_privilege":false,"executes_remote_code":false},"matches_request":true,"explanation":"du measures disk usage: -a includes files as well as folders and -h uses readable size units. The first pipe sends those measurements to sort; -h understands readable sizes and -r puts the largest first. The final pipe sends that ordering to head, where -n 10 keeps only the first ten results.","parts":[{"fragment":"du -ah .","meaning":"measure every file and folder using readable size units","source":"files in this folder"},{"fragment":" | sort -hr","meaning":"pipe those measurements into a readable-size sort, largest first","source":"largest"},{"fragment":" | head -n 10","meaning":"pipe the sorted list into head and keep its first ten entries","source":"show the 10"}]}"##,
        );
    } else {
        prompt.push_str(
            r##"

Required shape:
{"command":"pwd","effects":{"reads_data":true,"modifies_data":false,"deletes_data":false,"uses_network":false,"changes_remote_data":false,"changes_processes":false,"installs_software":false,"uses_privilege":false,"executes_remote_code":false},"matches_request":true,"explanation":"Prints the current directory."}"##,
        );
    }

    if let Some(os) = os {
        prompt.push_str(&format!("\n\nTarget OS: {}", os));
    }

    if let Some(shell) = shell {
        prompt.push_str(&format!("\nTarget shell: {}", shell));
    }

    prompt
}

#[cfg(test)]
mod tests {
    use super::build_system_prompt;

    #[test]
    fn includes_target_environment_and_required_effects() {
        let prompt = build_system_prompt(Some("macos"), Some("/bin/zsh"), false);

        assert!(prompt.contains("Target OS: macos"));
        assert!(prompt.contains("Target shell: /bin/zsh"));
        assert!(prompt.contains("\"deletes_data\":false"));
        assert!(prompt.contains("\"matches_request\":true"));
        assert!(!prompt.contains("semantic breakdown"));
    }

    #[test]
    fn requests_exact_semantic_parts_for_explanations() {
        let prompt = build_system_prompt(Some("macos"), Some("zsh"), true);

        assert!(prompt.contains("semantic breakdown"));
        assert!(prompt.contains("mandatory"));
        assert!(prompt.contains("concatenated exactly"));
        assert!(prompt.contains("standalone fallback"));
        assert!(prompt.contains("\"parts\":["));
        assert!(prompt.contains("\"source\":\"largest\""));
    }
}
