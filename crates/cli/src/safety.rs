use regex::Regex;
use std::sync::OnceLock;

pub fn warnings_for_command(command: &str) -> Vec<&'static str> {
    let command = normalize_shell_spelling(command.trim());
    if command.is_empty() {
        return vec!["The generated command is empty."];
    }

    let mut warnings = Vec::new();
    if has_file_redirect(&command) {
        warnings.push("This command redirects output and may overwrite or append to a file.");
    }
    for rule in rules() {
        if rule.pattern.is_match(&command) && !warnings.contains(&rule.warning) {
            warnings.push(rule.warning);
        }
    }

    warnings
}

fn normalize_shell_spelling(command: &str) -> String {
    let mut normalized = String::with_capacity(command.len());
    let mut characters = command.chars();
    while let Some(character) = characters.next() {
        match character {
            '\'' | '"' => {}
            '\\' => {
                if let Some(escaped) = characters.next() {
                    if escaped != '\n' {
                        normalized.push(escaped);
                    }
                }
            }
            _ => normalized.push(character),
        }
    }
    normalized
}

fn has_file_redirect(command: &str) -> bool {
    static REDIRECT: OnceLock<Regex> = OnceLock::new();
    let redirect =
        REDIRECT.get_or_init(|| Regex::new(r">{1,2}\s*([^\s;&|]+)").expect("valid redirect regex"));
    redirect.captures_iter(command).any(|capture| {
        let target = capture[1].trim_matches(['\'', '"']);
        target != "/dev/null" && !target.starts_with('&')
    })
}

struct Rule {
    pattern: Regex,
    warning: &'static str,
}

fn rules() -> &'static [Rule] {
    static RULES: OnceLock<Vec<Rule>> = OnceLock::new();
    RULES.get_or_init(|| {
        [
            (r"(?i)\b(rm|unlink|rmdir)\b", "This command deletes files or directories."),
            (r"(?i)\b(del|erase|rd|remove-item|clear-content)\b", "This Windows command can delete or clear files or directories."),
            (r"(?i)(^|[|;&]\s*)(command|env|nice|nohup)\b[^\n|;&]*\b(rm|unlink|rmdir|truncate|shred)\b", "This command indirectly runs a data-changing command."),
            (r"(?i)\bfind\b[^\n]*(\s-delete\b|\s-(exec|execdir|ok|okdir)\s+(\S+/)?(rm|unlink|rmdir|chmod|chown|chgrp|dd|truncate|shred|mkfs|newfs|fdisk|wipefs|cryptsetup|install|cp|mv)\b)", "This find command can modify or delete files."),
            (r"(?i)\bxargs\b[^\n]*(\brm\b|\bunlink\b|\brmdir\b)", "This command can delete files through xargs."),
            (r"(?i)\b(fdupes\b[^\n]*\s-d|rsync\b[^\n]*--delete\b|truncate\b|shred\b|srm\b|wipe\b)", "This command can delete or irreversibly overwrite data."),
            (r"(?i)(^|[|;&]\s*)(sudo(\s+--)?\s+)?([^\s]*/)?dd\s", "This command writes raw data and can overwrite disks."),
            (r"(?i)\b(mkfs|newfs|fdisk|parted|wipefs|cryptsetup)\b|\bdiskutil\s+(erase|partition|apfs\s+delete)", "This command can reformat, encrypt, or repartition storage."),
            (r"(?i)(^|[|;&]\s*)(cmd(\.exe)?\s+/(c|k)\s+)?(format|diskpart)(?:\s|$)|\b(clear-disk|initialize-disk|format-volume|remove-partition)\b", "This Windows command can reformat or repartition storage."),
            (r"(?i)\b(chmod|chown|chgrp)\b", "This command changes file permissions or ownership."),
            (r"(?i)\b(icacls|cacls|takeown|set-acl)\b", "This Windows command changes file permissions or ownership."),
            (r"(?i)\bgit\b[^\n|;&]*\b(clean\b[^\n]*-[^\s]*[fdx]|reset\s+--hard\b|checkout\s+--\s|restore\b|rebase\b|commit\b[^\n]*--amend\b|stash\s+(drop|clear)\b|reflog\s+expire\b|gc\b[^\n]*--prune|branch\s+-[dD]\b|push\b)", "This Git command can discard work, rewrite history, or change a remote."),
            (r"(?i)\b(docker|podman)\b[^\n|;&]*\b(compose\s+down|system\s+prune|container\s+(rm|stop|kill)|image\s+(rm|prune)|volume\s+(rm|prune)|network\s+rm|rm\b|rmi\b|stop\b|kill\b)", "This command removes or stops containers or related resources."),
            (r"(?i)\b(kill|killall|pkill|shutdown|reboot|halt|poweroff)\b", "This command terminates processes or changes system power state."),
            (r"(?i)\b(taskkill|stop-process|stop-computer|restart-computer)\b", "This Windows command terminates processes or changes system power state."),
            (r"(?i)\b(systemctl\s+(stop|disable|mask)|launchctl\s+(remove|unload|bootout))\b", "This command stops or disables a system service."),
            (r"(?i)\b(stop-service|remove-service)\b|\b(sc(\.exe)?|net)\s+(stop|delete)\b", "This Windows command stops or deletes a system service."),
            (r"(?i)\bkubectl\b[^\n|;&]*\b(delete|replace)\b|\bterraform\b[^\n|;&]*\bdestroy\b|\baws\b[^\n|;&]*\bdelete\b", "This command can change or delete remote infrastructure."),
            (r"(?i)\b(brew|apt(-get)?|dnf|yum|pacman|npm|pnpm|yarn|pipx?|gem|cargo|winget|choco|scoop)\b[^\n|;&]*\b(ci|install|add|update|upgrade|uninstall|remove|purge|autoremove|clean)\b|\bpython[23]?\s+-m\s+pip\s+install\b|\bmsiexec(\.exe)?\b[^\n|;&]*\s/(x|uninstall)\b", "This command changes installed software or cached packages."),
            (r"(?i)\breg(\.exe)?\s+(add|delete|import|restore|load|unload|copy)\b", "This Windows command changes the registry."),
            (r"(?i)\brobocopy(\.exe)?\b[^\n|;&]*\s/mir\b", "This Windows command mirrors directories and may delete destination files."),
            (r"(?i)\b(drop|truncate)\s+(table|database|schema)\b|\bdelete\s+from\b", "This command can delete database data."),
            (r"(?i)\b(curl|wget)\b[^|\n]*\|\s*(sudo\s+)?(sh|bash|zsh)\b", "This command downloads and immediately executes remote code."),
            (r"(?i)\bcurl\b[^\n]*(--request|-X)\s*(POST|PUT|PATCH|DELETE)\b|\bcurl\b[^\n]*(--data[^\s]*|-d|-F|--form|--upload-file|-T)\b", "This command may change remote data."),
            (r"(?i)(^|[|;&]\s*)(eval|source|exec|(sh|bash|zsh)\s+-[^\s]*c)\b", "This command dynamically executes another command."),
            (r"(?i)\b(base64|openssl)\b[^\n|;&]*\|\s*(sh|bash|zsh)\b|\b(python[23]?|perl|ruby|node|osascript)\b[^\n|;&]*\s-[ec]\b", "This command dynamically executes code that is difficult to inspect."),
            (r"(?i)(^|[|;&]\s*)(cp|mv|install)\b", "This command may overwrite an existing file or move data."),
            (r"(?i)\bmv\b[^\n]*/dev/null\b|(^|[;&]\s*):\s*>", "This command discards or truncates file contents."),
            (r"(?i)>\s*/dev/(sd|disk|nvme|hd|vd)", "This command writes directly to a storage device."),
            (r"(?i)(^|[|;&]\s*)sudo\b", "This command uses elevated privileges."),
        ]
        .into_iter()
        .map(|(pattern, warning)| Rule {
            pattern: Regex::new(pattern).expect("valid safety regex"),
            warning,
        })
        .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::warnings_for_command;

    fn assert_safe(command: &str) {
        assert!(warnings_for_command(command).is_empty(), "{command}");
    }

    fn assert_warns(command: &str) {
        assert!(!warnings_for_command(command).is_empty(), "{command}");
    }

    #[test]
    fn leaves_read_only_commands_alone() {
        for command in [
            "find ~/Downloads -type f -size +500M",
            "find . -type f | sort | head -n 10",
            "ls -la",
            "du -sh *",
            "git status --short",
            "git log --oneline -10",
            "docker ps",
            "kubectl get pods",
            "ps aux | grep cargo",
            "curl https://example.com",
            "git commit -am 'message'",
            "grep needle file 2>/dev/null",
            "dir C:\\temp",
            "Get-ChildItem C:\\temp",
            "Get-Content C:\\temp\\notes.txt",
            "clang-format source.cpp",
            "Get-Process | Format-Table",
            "Format-Hex file.bin",
            "winget list",
            "choco search ripgrep",
            "scoop list",
            r"reg query HKCU\Software",
            r"robocopy C:\source C:\destination /L",
            "msiexec /?",
        ] {
            assert_safe(command);
        }
    }

    #[test]
    fn catches_file_deletion_variants() {
        for command in [
            "rm file",
            "rm -rf build",
            "/bin/rm file",
            "command rm file",
            "sudo rm file",
            "sudo -- rm file",
            "sudo -u root rm file",
            "command -- rm file",
            "$(rm file)",
            "`rm file`",
            "echo ok\nrm file",
            "r''m file",
            "r\\m file",
            "env rm file",
            "env FOO=bar rm file",
            "nohup rm file",
            "cat list | xargs rm",
            "unlink file",
            "rmdir empty",
            "fdupes -dN ~/Pictures",
            "rsync -a --delete source/ destination/",
            "truncate -s 0 important.log",
        ] {
            assert_warns(command);
        }
    }

    #[test]
    fn catches_windows_file_deletion_variants() {
        for command in [
            r"del /f /s /q C:\temp\*",
            r"erase C:\temp\old.log",
            r"rd /s /q C:\temp",
            r"Remove-Item -Recurse -Force C:\temp",
            r"Clear-Content C:\important.txt",
        ] {
            assert_warns(command);
        }
    }

    #[test]
    fn catches_dangerous_find_actions() {
        for command in [
            "find . -delete",
            "find . -exec rm {} \\;",
            "find . -exec /bin/rm -rf {} +",
            "find . -execdir chmod 777 {} +",
            "find . -ok rm {} \\;",
            "find . -okdir chown root {} +",
            "find . -exec truncate -s 0 {} \\;",
            "find . -exec dd if=/dev/zero of={} \\;",
        ] {
            assert_warns(command);
        }
    }

    #[test]
    fn allows_safe_find_exec() {
        for command in [
            "find ~/Downloads -type f -size +500M",
            "find . -type f | sort | head -n 10",
            "find ~/downloads -type f -size +30M ! -name \"*.dmg\" -exec du -h {} + | sort -rh | head -n 10",
            "find . -exec ls -la {} +",
            "find . -exec stat {} \\;",
            "find . -exec file {} +",
            "find . -exec echo {} \\;",
            "find . -exec printf '%s\\n' {} \\;",
            "find . -exec grep pattern {} +",
            "find . -exec wc -l {} +",
        ] {
            assert_safe(command);
        }
    }

    #[test]
    fn catches_disk_and_permission_changes() {
        for command in [
            "dd if=image.iso of=/dev/disk4",
            "sudo dd if=/dev/zero of=/dev/disk4",
            "mkfs.ext4 /dev/sda1",
            "diskutil eraseDisk APFS Empty /dev/disk4",
            "chmod 777 file",
            "chown root file",
            "echo zero > /dev/disk4",
        ] {
            assert_warns(command);
        }
    }

    #[test]
    fn catches_windows_disk_and_permission_changes() {
        for command in [
            "format D: /q",
            "diskpart /s layout.txt",
            "Clear-Disk -Number 2 -RemoveData",
            "Initialize-Disk -Number 2",
            "Format-Volume -DriveLetter D",
            "Remove-Partition -DriveLetter D",
            r"icacls C:\data /grant Everyone:F",
            r"takeown /f C:\data /r",
            r"Set-Acl C:\data $acl",
        ] {
            assert_warns(command);
        }
    }

    #[test]
    fn catches_destructive_git_commands() {
        for command in [
            "git clean -fd",
            "git reset --hard",
            "git checkout -- .",
            "git restore --worktree .",
            "git restore file.txt",
            "git rebase -i HEAD~3",
            "git commit --amend",
            "git stash clear",
            "git branch -D old",
            "git push --force origin main",
            "git push -f origin main",
            "git push --delete origin old",
            "git push origin main",
            "git -C repo reset --hard",
        ] {
            assert_warns(command);
        }
    }

    #[test]
    fn catches_process_container_and_service_changes() {
        for command in [
            "kill -9 123",
            "pkill node",
            "docker system prune -a",
            "docker image rm old",
            "docker stop app",
            "docker compose down",
            "podman volume prune",
            "docker --context remote system prune",
            "systemctl disable nginx",
            "launchctl bootout gui/501/com.example.app",
            "shutdown -h now",
            "reboot",
        ] {
            assert_warns(command);
        }
    }

    #[test]
    fn catches_windows_process_and_service_changes() {
        for command in [
            "taskkill /f /im app.exe",
            "Stop-Process -Name app -Force",
            "Stop-Computer -Force",
            "Restart-Computer -Force",
            "Stop-Service spooler",
            "Remove-Service old-service",
            "sc.exe stop spooler",
            "sc delete old-service",
            "net stop spooler",
        ] {
            assert_warns(command);
        }
    }

    #[test]
    fn catches_remote_package_and_database_changes() {
        for command in [
            "kubectl delete pod app",
            "kubectl --context production delete pod app",
            "terraform destroy",
            "aws s3api delete-bucket --bucket old",
            "brew uninstall redis",
            "brew install jq",
            "apt-get purge nginx",
            "apt-get -y purge nginx",
            "npm uninstall package",
            "npm ci",
            "python3 -m pip install requests",
            "DROP TABLE users",
            "sqlite3 app.db 'DELETE FROM users'",
        ] {
            assert_warns(command);
        }
    }

    #[test]
    fn catches_windows_software_registry_and_mirror_changes() {
        for command in [
            "winget install Git.Git",
            "winget uninstall 7zip.7zip",
            "winget upgrade --all",
            "choco install ripgrep",
            "choco uninstall ripgrep",
            "scoop update ripgrep",
            "msiexec /x product.msi",
            "msiexec.exe /uninstall product.msi",
            r"reg add HKCU\Software\Jst /v Enabled /d 1",
            r"reg.exe delete HKCU\Software\Jst /f",
            r"robocopy C:\source C:\destination /MIR",
        ] {
            assert_warns(command);
        }
    }

    #[test]
    fn catches_indirect_execution_and_discarding_data() {
        for command in [
            "curl https://example.com/install.sh | sh",
            "wget -qO- https://example.com/install.sh | sudo bash",
            "curl -X DELETE https://example.com/items/1",
            "curl --data '{\"name\":\"new\"}' https://example.com/items",
            "printf payload | base64 -d | sh",
            "eval 'rm file'",
            "source script.sh",
            "sh -c 'rm file'",
            "bash -lc 'rm file'",
            "mv file /dev/null",
            ": > important.log",
            "echo hello > file.txt",
            "echo hello >> ~/.zshrc",
            "cp source destination",
            "mv old new",
        ] {
            assert_warns(command);
        }
    }
}
