pub fn build_system_prompt(context: Option<&str>, os: Option<&str>, shell: Option<&str>) -> String {
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

Schema:
{"command":"string","effects":{"reads_data":bool,"modifies_data":bool,"deletes_data":bool,"uses_network":bool,"changes_remote_data":bool,"changes_processes":bool,"installs_software":bool,"uses_privilege":bool,"executes_remote_code":bool},"matches_request":bool,"explanation":"one short sentence describing what the command does"}"##,
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
