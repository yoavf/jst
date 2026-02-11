use regex::Regex;
use std::sync::OnceLock;

pub fn is_destructive_command(command: &str) -> bool {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

    let patterns = PATTERNS.get_or_init(|| {
        [
            // rm (but not "rmdir" alone which is safe on non-empty dirs)
            r"\brm\s",
            // find ... -delete / -exec rm
            r"\bfind\b.*-delete\b",
            r"\bfind\b.*-exec\s+rm\b",
            // shred / srm / wipe
            r"\b(shred|srm|wipe)\b",
            // dd (disk destroyer)
            r"\bdd\s",
            // mkfs / format
            r"\b(mkfs|newfs)\b",
            // chmod/chown recursive or broad
            r"\b(chmod|chown)\s.*-[rR]",
            // truncate / overwrite via redirect to important paths
            r">\s*/dev/sd",
            r">\s*/dev/disk",
            // git clean -f, git reset --hard, git push --force
            r"\bgit\s+clean\b.*-[fdxF]",
            r"\bgit\s+reset\s+--hard\b",
            r"\bgit\s+push\b.*--force\b",
            r"\bgit\s+push\b.*-f\b",
            // docker system prune, docker rm
            r"\bdocker\s+(system\s+prune|rm\b|rmi\b)",
            // drop/truncate in SQL
            r"(?i)\b(drop|truncate)\s+(table|database|schema)\b",
            // kill / killall / pkill
            r"\b(kill|killall|pkill)\s",
            // systemctl stop/disable/mask
            r"\bsystemctl\s+(stop|disable|mask)\b",
            // launchctl remove/unload
            r"\blaunchctl\s+(remove|unload)\b",
            // mv to /dev/null
            r"\bmv\b.*/dev/null",
            // overwrite with : > or > on files
            r":\s*>",
        ]
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect()
    });

    for segment in command.split(['|', ';', '&']) {
        let trimmed = segment.trim();
        for pat in patterns {
            if pat.is_match(trimmed) {
                return true;
            }
        }
    }

    false
}
