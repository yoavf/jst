use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

const FLUSH_INTERVAL: Duration = Duration::from_secs(60);
const SNAPSHOT_CACHE_TTL: Duration = Duration::from_secs(60);
const MAX_BUFFERED_COMMANDS: usize = 256;
const MAX_BASE_COMMAND_BYTES: usize = 32;
const TOP_COMMANDS: usize = 20;
const DAILY_TREND_DAYS: u64 = 30;
const DAY_KEY_TTL_SECONDS: u64 = 40 * 24 * 60 * 60;
const TOTAL_KEY: &str = "jst:stats:total";
const COMMANDS_KEY: &str = "jst:stats:commands";
const DAY_KEY_PREFIX: &str = "jst:stats:day:";
const COMMAND_WRAPPERS: [&str; 8] = [
    "sudo", "doas", "env", "time", "nice", "command", "builtin", "noglob",
];

/// Buffers anonymous aggregate counters locally and flushes them to a shared
/// serverless Redis so any number of machines contribute to the same totals.
/// Only the base command name is counted — never arguments, paths, or input.
pub struct StatsCollector {
    client: reqwest::Client,
    url: String,
    token: String,
    buffer: Mutex<Buffer>,
    cache: Mutex<Option<CachedSnapshot>>,
}

#[derive(Default)]
struct Buffer {
    total: u64,
    commands: HashMap<String, u64>,
}

struct CachedSnapshot {
    fetched_at: Instant,
    snapshot: StatsSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StatsSnapshot {
    pub total: u64,
    pub top_commands: Vec<CommandCount>,
    pub daily: Vec<DayCount>,
    pub generated_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommandCount {
    pub command: String,
    pub count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DayCount {
    pub date: String,
    pub count: u64,
}

impl StatsCollector {
    pub fn from_env(client: &reqwest::Client) -> Option<Arc<Self>> {
        let url = std::env::var("UPSTASH_REDIS_REST_URL").ok();
        let token = std::env::var("UPSTASH_REDIS_REST_TOKEN").ok();
        let (Some(url), Some(token)) = (url, token) else {
            info!("Usage stats disabled: UPSTASH_REDIS_REST_URL/UPSTASH_REDIS_REST_TOKEN not set");
            return None;
        };
        if url.is_empty() || token.is_empty() {
            info!("Usage stats disabled: Upstash configuration is empty");
            return None;
        }
        info!(
            "Usage stats enabled, flushing every {}s",
            FLUSH_INTERVAL.as_secs()
        );
        Some(Arc::new(Self {
            client: client.clone(),
            url,
            token,
            buffer: Mutex::new(Buffer::default()),
            cache: Mutex::new(None),
        }))
    }

    pub fn record(&self, command: &str) {
        let mut buffer = self.buffer.lock().expect("stats buffer lock poisoned");
        buffer.total += 1;
        let Some(base) = base_command(command) else {
            return;
        };
        if buffer.commands.len() < MAX_BUFFERED_COMMANDS || buffer.commands.contains_key(&base) {
            *buffer.commands.entry(base).or_default() += 1;
        }
    }

    pub async fn flush_loop(self: Arc<Self>) {
        loop {
            tokio::time::sleep(FLUSH_INTERVAL).await;
            self.flush().await;
        }
    }

    pub async fn flush(&self) {
        let buffer = {
            let mut pending = self.buffer.lock().expect("stats buffer lock poisoned");
            std::mem::take(&mut *pending)
        };
        if buffer.total == 0 && buffer.commands.is_empty() {
            return;
        }
        if let Err(error) = self
            .post_pipeline(flush_body(&buffer, &utc_date(unix_now())))
            .await
        {
            warn!("Usage stats flush failed, keeping counts for next flush: {error}");
            let mut pending = self.buffer.lock().expect("stats buffer lock poisoned");
            pending.merge(buffer);
        }
    }

    pub async fn snapshot(
        &self,
    ) -> Result<StatsSnapshot, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(cached) = self.cached_snapshot() {
            return Ok(cached);
        }

        match self.fetch_snapshot().await {
            Ok(snapshot) => {
                let mut cache = self.cache.lock().expect("stats cache lock poisoned");
                *cache = Some(CachedSnapshot {
                    fetched_at: Instant::now(),
                    snapshot: snapshot.clone(),
                });
                Ok(snapshot)
            }
            Err(error) => {
                let stale = self
                    .cache
                    .lock()
                    .expect("stats cache lock poisoned")
                    .as_ref()
                    .map(|cached| cached.snapshot.clone());
                stale.ok_or(error)
            }
        }
    }

    fn cached_snapshot(&self) -> Option<StatsSnapshot> {
        let cache = self.cache.lock().expect("stats cache lock poisoned");
        cache
            .as_ref()
            .filter(|cached| cached.fetched_at.elapsed() < SNAPSHOT_CACHE_TTL)
            .map(|cached| cached.snapshot.clone())
    }

    async fn fetch_snapshot(
        &self,
    ) -> Result<StatsSnapshot, Box<dyn std::error::Error + Send + Sync>> {
        let today = unix_now() / 86_400;
        let dates: Vec<String> = (0..DAILY_TREND_DAYS)
            .map(|offset| utc_date((today - (DAILY_TREND_DAYS - 1) + offset) * 86_400))
            .collect();

        let mut commands = vec![
            serde_json::json!(["GET", TOTAL_KEY]),
            serde_json::json!([
                "ZRANGE",
                COMMANDS_KEY,
                "0",
                (TOP_COMMANDS - 1).to_string(),
                "REV",
                "WITHSCORES"
            ]),
        ];
        for date in &dates {
            commands.push(serde_json::json!(["GET", day_key(date)]));
        }

        let results = self
            .post_pipeline(serde_json::Value::Array(commands))
            .await?;
        parse_snapshot(&results, &dates)
    }

    async fn post_pipeline(
        &self,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let response = self
            .client
            .post(format!("{}/pipeline", self.url.trim_end_matches('/')))
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(format!("stats store returned {}", response.status()).into());
        }
        Ok(response.json::<serde_json::Value>().await?)
    }
}

impl Buffer {
    fn merge(&mut self, other: Buffer) {
        self.total += other.total;
        for (command, count) in other.commands {
            *self.commands.entry(command).or_default() += count;
        }
    }
}

/// Extracts the effective command name for the usage histogram: skips leading
/// environment assignments and wrappers such as sudo, and strips any path.
/// Returns None when the first meaningful token is not a plausible command.
pub fn base_command(command: &str) -> Option<String> {
    for token in command.split_whitespace() {
        if is_env_assignment(token) {
            continue;
        }
        let name = token
            .rsplit('/')
            .next()
            .unwrap_or(token)
            .to_ascii_lowercase();
        if COMMAND_WRAPPERS.contains(&name.as_str()) {
            continue;
        }
        if name.is_empty()
            || name.len() > MAX_BASE_COMMAND_BYTES
            || !name.chars().all(is_command_character)
        {
            return None;
        }
        return Some(name);
    }
    None
}

fn is_command_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | '+')
}

fn is_env_assignment(token: &str) -> bool {
    let Some((name, _)) = token.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn flush_body(buffer: &Buffer, today: &str) -> serde_json::Value {
    let mut commands = Vec::with_capacity(buffer.commands.len() + 3);
    if buffer.total > 0 {
        commands.push(serde_json::json!([
            "INCRBY",
            TOTAL_KEY,
            buffer.total.to_string()
        ]));
        commands.push(serde_json::json!([
            "INCRBY",
            day_key(today),
            buffer.total.to_string()
        ]));
        commands.push(serde_json::json!([
            "EXPIRE",
            day_key(today),
            DAY_KEY_TTL_SECONDS.to_string()
        ]));
    }
    for (command, count) in &buffer.commands {
        commands.push(serde_json::json!([
            "ZINCRBY",
            COMMANDS_KEY,
            count.to_string(),
            command
        ]));
    }
    serde_json::Value::Array(commands)
}

fn parse_snapshot(
    results: &serde_json::Value,
    dates: &[String],
) -> Result<StatsSnapshot, Box<dyn std::error::Error + Send + Sync>> {
    let entries = results
        .as_array()
        .ok_or("stats store returned no results")?;
    if entries.len() != 2 + dates.len() {
        return Err("stats store returned an unexpected number of results".into());
    }
    let total = entries
        .first()
        .and_then(|entry| entry.get("result"))
        .and_then(value_as_u64)
        .unwrap_or(0);
    let scores = entries
        .get(1)
        .and_then(|entry| entry.get("result"))
        .and_then(serde_json::Value::as_array)
        .ok_or("stats store returned no command scores")?;
    if scores.len() % 2 != 0 {
        return Err("stats store returned malformed command scores".into());
    }

    let mut top_commands = Vec::with_capacity(scores.len() / 2);
    for pair in scores.chunks_exact(2) {
        let (Some(command), Some(count)) = (pair[0].as_str(), value_as_u64(&pair[1])) else {
            return Err("stats store returned malformed command scores".into());
        };
        top_commands.push(CommandCount {
            command: command.to_string(),
            count,
        });
    }

    let mut daily = Vec::with_capacity(dates.len());
    for (entry, date) in entries[2..].iter().zip(dates) {
        let Some(result) = entry.get("result") else {
            return Err("stats store returned a malformed daily count".into());
        };
        daily.push(DayCount {
            date: date.clone(),
            count: value_as_u64(result).unwrap_or(0),
        });
    }

    Ok(StatsSnapshot {
        total,
        top_commands,
        daily,
        generated_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    })
}

fn day_key(date: &str) -> String {
    format!("{DAY_KEY_PREFIX}{date}")
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Formats a Unix timestamp as a UTC `YYYY-MM-DD` date.
fn utc_date(unix_secs: u64) -> String {
    let days = (unix_secs / 86_400) as i64;
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = (shifted - era * 146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    let year = if month <= 2 { year + 1 } else { year };
    format!("{year:04}-{month:02}-{day:02}")
}

fn value_as_u64(value: &serde_json::Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|text| text.parse::<u64>().ok()))
        .or_else(|| value.as_f64().map(|number| number as u64))
}

#[cfg(test)]
mod tests {
    use super::{base_command, flush_body, parse_snapshot, utc_date, Buffer};
    use std::collections::HashMap;

    #[test]
    fn extracts_plain_commands() {
        assert_eq!(base_command("find . -type f").as_deref(), Some("find"));
        assert_eq!(base_command("git status").as_deref(), Some("git"));
        assert_eq!(base_command("lsof -i :3000").as_deref(), Some("lsof"));
    }

    #[test]
    fn skips_wrappers_and_env_assignments() {
        assert_eq!(base_command("sudo rm -rf /tmp/x").as_deref(), Some("rm"));
        assert_eq!(
            base_command("sudo FOO=bar apt-get update").as_deref(),
            Some("apt-get")
        );
        assert_eq!(
            base_command("FOO=bar BAR=baz grep x").as_deref(),
            Some("grep")
        );
        assert_eq!(base_command("time ls -la").as_deref(), Some("ls"));
    }

    #[test]
    fn strips_paths_and_normalizes_case() {
        assert_eq!(base_command("/usr/bin/find .").as_deref(), Some("find"));
        assert_eq!(
            base_command("./scripts/deploy.sh").as_deref(),
            Some("deploy.sh")
        );
        assert_eq!(
            base_command("PYTHON3 -m http.server").as_deref(),
            Some("python3")
        );
    }

    #[test]
    fn rejects_implausible_commands() {
        assert_eq!(base_command("(echo hi)"), None);
        assert_eq!(base_command("FOO=bar"), None);
        assert_eq!(base_command(""), None);
        assert_eq!(base_command(&format!("{} --flag", "x".repeat(33))), None);
    }

    #[test]
    fn buffer_merge_accumulates_counts() {
        let mut target = Buffer {
            total: 2,
            commands: HashMap::from([("find".to_string(), 2)]),
        };
        target.merge(Buffer {
            total: 3,
            commands: HashMap::from([("find".to_string(), 1), ("git".to_string(), 2)]),
        });
        assert_eq!(target.total, 5);
        assert_eq!(target.commands["find"], 3);
        assert_eq!(target.commands["git"], 2);
    }

    #[test]
    fn flush_body_batches_increments() {
        let body = flush_body(
            &Buffer {
                total: 2,
                commands: HashMap::from([("find".to_string(), 2)]),
            },
            "2026-07-20",
        );
        assert_eq!(
            body,
            serde_json::json!([
                ["INCRBY", "jst:stats:total", "2"],
                ["INCRBY", "jst:stats:day:2026-07-20", "2"],
                ["EXPIRE", "jst:stats:day:2026-07-20", "3456000"],
                ["ZINCRBY", "jst:stats:commands", "2", "find"]
            ])
        );
    }

    #[test]
    fn parses_snapshot_results() {
        let dates = vec!["2026-07-19".to_string(), "2026-07-20".to_string()];
        let snapshot = parse_snapshot(
            &serde_json::json!([
                {"result": "42"},
                {"result": ["find", "30", "git", 12]},
                {"result": null},
                {"result": "7"}
            ]),
            &dates,
        )
        .expect("valid snapshot");
        assert_eq!(snapshot.total, 42);
        assert_eq!(snapshot.top_commands[0].command, "find");
        assert_eq!(snapshot.top_commands[0].count, 30);
        assert_eq!(snapshot.top_commands[1].count, 12);
        assert_eq!(
            snapshot.daily,
            vec![
                super::DayCount {
                    date: "2026-07-19".to_string(),
                    count: 0
                },
                super::DayCount {
                    date: "2026-07-20".to_string(),
                    count: 7
                },
            ]
        );
    }

    #[test]
    fn parses_empty_store() {
        let snapshot = parse_snapshot(&serde_json::json!([{"result": null}, {"result": []}]), &[])
            .expect("empty snapshot");
        assert_eq!(snapshot.total, 0);
        assert!(snapshot.top_commands.is_empty());
        assert!(snapshot.daily.is_empty());
    }

    #[test]
    fn rejects_malformed_snapshots() {
        assert!(parse_snapshot(&serde_json::json!({"error": "nope"}), &[]).is_err());
        assert!(parse_snapshot(&serde_json::json!([{"result": null}]), &[]).is_err());
        assert!(parse_snapshot(
            &serde_json::json!([
                {"result": "1"},
                {"result": ["find"]}
            ]),
            &[]
        )
        .is_err());
        assert!(parse_snapshot(
            &serde_json::json!([
                {"result": "1"},
                {"result": []}
            ]),
            &["2026-07-20".to_string()]
        )
        .is_err());
    }

    #[test]
    fn formats_utc_dates() {
        assert_eq!(utc_date(0), "1970-01-01");
        assert_eq!(utc_date(1_582_934_400), "2020-02-29");
        assert_eq!(utc_date(1_719_792_000), "2024-07-01");
        assert_eq!(utc_date(1_784_564_636), "2026-07-20");
    }
}
