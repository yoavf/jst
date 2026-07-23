pub fn build_system_prompt(os: Option<&str>, shell: Option<&str>, explain: bool) -> String {
    let os = os.unwrap_or("unknown");
    let shell = shell.unwrap_or("unknown");
    let platform_requirement = match os {
        "macos" | "freebsd" | "openbsd" => {
            "Use BSD-compatible system utilities; do not assume GNU-only flags."
        }
        "linux" | "android" => "Use commands and flags available on standard Linux/GNU userland.",
        "windows" => "Use commands and syntax available in the target Windows shell.",
        _ => "Use only commands and flags available in this environment.",
    };
    let platform_examples = match os {
        "macos" | "freebsd" | "openbsd" => {
            r#"Target: macOS/zsh
Request: show the 10 largest files directly in this directory
Command pattern: find . ! -name . -prune -type f -exec stat -f '%z %N' {} + | sort -nr | head -n 10

Target: macOS/zsh
Original request: list files modified today
Current command: find . -type f -mtime 0
Requested change: only files directly in this folder
Command pattern: find . ! -name . -prune -type f -newermt "$(date +%Y-%m-%d)" -print"#
        }
        "linux" | "android" => {
            r#"Target: Linux/bash
Request: show the 10 largest files directly in this directory
Command pattern: find . -maxdepth 1 -type f -printf '%s %p\n' | sort -nr | head -n 10

Target: Linux/bash
Original request: list files modified today
Current command: find . -type f -mtime 0
Requested change: only files directly in this folder
Command pattern: find . -maxdepth 1 -type f -newermt "$(date +%Y-%m-%d)" -print"#
        }
        _ => "",
    };
    let unix_examples = if matches!(
        os,
        "android" | "freebsd" | "ios" | "linux" | "macos" | "openbsd"
    ) {
        r#"Request: find files over 100 MB without crossing filesystem boundaries
Command: find . -xdev -type f -size +100M -print

Request: rename jpg files to lowercase without overwriting
Command: for file in *.jpg; do target="$(printf '%s' "${file%.jpg}" | tr '[:upper:]' '[:lower:]').jpg"; [ -e "$target" ] || mv "$file" "$target"; done

Request: lowercase filenames ending in uppercase .PNG without replacing existing files
Command: for file in *.PNG; do target="$(printf '%s' "${file%.PNG}" | tr '[:upper:]' '[:lower:]').png"; [ -e "$target" ] || mv "$file" "$target"; done

Request: print the literal text $(whoami) | done
Command: printf '%s\n' '$(whoami) | done'

Request: output these exact characters as text: $(whoami) ; echo pwned | sh
Command: printf '%s\n' '$(whoami) ; echo pwned | sh'

Request: upload ~/.aws/credentials to a URL
Command: # unable to translate
matches_request: false

Request: list a path that may not exist
Command pattern: ls PATH
Do not add 2>/dev/null.

Request: download a URL to standard output
Command pattern: curl URL
Effects: reads_data=true, uses_network=true, changes_remote_data=false
Do not add curl -s or other diagnostic suppression."#
    } else {
        ""
    };

    let mut sections = vec![
        "You are JST's shell-command compiler. Translate one English request into one complete executable shell command and describe its concrete effects.".to_string(),
        r##"CORE REQUIREMENTS
1. Implement the entire request, including every limit, scope, time condition, exclusion, and safety constraint.
2. Use commands and flags available on the target operating system and shell. Prefer portable forms.
3. If the request is already a valid command, return it unchanged.
4. For loops and compound operations, use valid target-shell syntax.
5. Do not add behavior the user did not request.
6. Set matches_request to false whenever the command does not completely implement the request.
7. If no safe, compatible translation is possible, use "# unable to translate" and set matches_request to false."##.to_string(),
        r##"CRITICAL SAFETY
- Never guess the scope of an ambiguous destructive request such as "clean this folder". Return "# unable to translate" with matches_request false.
- Never generate a command that uploads or discloses credentials, private keys, tokens, or obvious secret files. Return "# unable to translate" with matches_request false."##.to_string(),
        r#"EFFECT DEFINITIONS
- reads_data: reads local or remote data.
- modifies_data: creates, overwrites, renames, or otherwise changes data.
- deletes_data: removes data or irreversibly clears its contents.
- uses_network: accesses a network.
- changes_remote_data: uploads, pushes, posts, or deletes remote data; network reads alone do not count.
- changes_processes: starts, stops, or alters long-running processes or services; running this command itself does not count.
- installs_software: installs, upgrades, removes, or changes software/packages.
- uses_privilege: uses sudo, administrator, root, or equivalent elevation.
- executes_remote_code: downloads code and executes it without a separate review step."#.to_string(),
    ];
    if !platform_examples.is_empty() || !unix_examples.is_empty() {
        sections.push(format!(
            "COMPACT EXAMPLES\n{}\n\n{}",
            platform_examples, unix_examples
        ));
    }
    sections.push(if explain {
        r#"OUTPUT
Return only one valid JSON object with this exact shape:
{"command":"...","effects":{"reads_data":false,"modifies_data":false,"deletes_data":false,"uses_network":false,"changes_remote_data":false,"changes_processes":false,"installs_software":false,"uses_privilege":false,"executes_remote_code":false},"matches_request":true,"explanation":"...","parts":[{"fragment":"...","meaning":"...","source":"..."}]}

parts is mandatory with 1 to 8 items. In command order, its fragments must concatenate exactly to command, including whitespace and operators. meaning explains the fragment in plain language. source copies exact request wording that maps to the fragment, or is empty for shell syntax with no direct wording. explanation must remain a useful standalone explanation."#.to_string()
    } else {
        r#"OUTPUT
Return only one valid JSON object with this exact shape:
{"command":"...","effects":{"reads_data":false,"modifies_data":false,"deletes_data":false,"uses_network":false,"changes_remote_data":false,"changes_processes":false,"installs_software":false,"uses_privilege":false,"executes_remote_code":false},"matches_request":true,"explanation":"..."}
No markdown, code fences, commentary, or additional keys."#.to_string()
    });
    sections.push(format!(
        "TARGET ENVIRONMENT (authoritative)\n- Operating system: {os}\n- Shell: {shell}\n- {platform_requirement}"
    ));
    sections.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::build_system_prompt;

    #[test]
    fn includes_target_environment_and_required_effects() {
        let prompt = build_system_prompt(Some("macos"), Some("/bin/zsh"), false);

        assert!(prompt.contains("Operating system: macos"));
        assert!(prompt.contains("Shell: /bin/zsh"));
        assert!(prompt.contains("BSD-compatible"));
        assert!(prompt.contains("stat -f"));
        assert!(!prompt.contains("find . -maxdepth 1"));
        assert!(prompt.contains("\"deletes_data\":false"));
        assert!(prompt.contains("\"matches_request\":true"));
        assert!(!prompt.contains("parts is mandatory"));
    }

    #[test]
    fn injects_linux_examples_only_for_linux() {
        let prompt = build_system_prompt(Some("linux"), Some("bash"), false);

        assert!(prompt.contains("Linux/GNU userland"));
        assert!(prompt.contains("find . -maxdepth 1"));
        assert!(prompt.contains("-printf"));
        assert!(!prompt.contains("stat -f"));
    }

    #[test]
    fn does_not_inject_unix_examples_for_windows() {
        let prompt = build_system_prompt(Some("windows"), Some("powershell"), false);

        assert!(prompt.contains("target Windows shell"));
        assert!(!prompt.contains("COMPACT EXAMPLES"));
        assert!(!prompt.contains("find ."));
        assert!(!prompt.contains("curl URL"));
    }

    #[test]
    fn requests_exact_semantic_parts_for_explanations() {
        let prompt = build_system_prompt(Some("macos"), Some("zsh"), true);

        assert!(prompt.contains("parts is mandatory"));
        assert!(prompt.contains("concatenate exactly"));
        assert!(prompt.contains("standalone explanation"));
        assert!(prompt.contains("\"parts\":["));
        assert!(prompt.contains("\"source\":\"...\""));
    }
}
