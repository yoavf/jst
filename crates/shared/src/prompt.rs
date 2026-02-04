pub fn build_system_prompt(context: Option<&str>, os: Option<&str>, shell: Option<&str>) -> String {
    let mut prompt = String::from(
        r#"You are a CLI command translator. Convert natural language into executable shell commands.

Rules:
- Output ONLY the shell command, nothing else. No explanation, no markdown, no backticks.
- If the input is ambiguous, output the most likely intended command.
- If the input is already a valid command, output it as-is.
- For loops and multi-command operations, use proper shell syntax.
- Prefer common, portable commands when possible.
- If you truly cannot translate the input, output: # unable to translate"#,
    );

    if let Some(os) = os {
        prompt.push_str(&format!("\n\nTarget OS: {}", os));
    }

    if let Some(shell) = shell {
        prompt.push_str(&format!("\nTarget shell: {}", shell));
    }

    if let Some(ctx) = context {
        prompt.push_str(&format!("\n\nProject context:\n{}", ctx));
    }

    prompt
}
