pub fn build_system_prompt(os: Option<&str>, shell: Option<&str>) -> String {
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

Required shape:
{"command":"pwd","effects":{"reads_data":true,"modifies_data":false,"deletes_data":false,"uses_network":false,"changes_remote_data":false,"changes_processes":false,"installs_software":false,"uses_privilege":false,"executes_remote_code":false},"matches_request":true,"explanation":"Prints the current directory."}"##,
    );

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
        let prompt = build_system_prompt(Some("macos"), Some("/bin/zsh"));

        assert!(prompt.contains("Target OS: macos"));
        assert!(prompt.contains("Target shell: /bin/zsh"));
        assert!(prompt.contains("\"deletes_data\":false"));
        assert!(prompt.contains("\"matches_request\":true"));
    }
}
