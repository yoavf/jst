use sha2::{Digest, Sha256};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::info;

const RATE_LIMIT_KEY_PREFIX: &str = "jst:rate-limit";

// Atomically checks the configured limits in priority order. Earlier counters
// are intentionally consumed when a later limit rejects the request, matching
// the previous in-memory behavior.
const CHECK_LIMITS_SCRIPT: &str = r#"
local outcomes = {}
local now_ms = tonumber(ARGV[1])

for index = 1, #KEYS, 2 do
  local argument = 2 + ((index - 1) / 2) * 4
  local member = ARGV[argument]
  local limit = tonumber(ARGV[argument + 1])
  local window_ms = tonumber(ARGV[argument + 2])
  local max_entries = tonumber(ARGV[argument + 3])
  local counter_key = KEYS[index]
  local entries_key = KEYS[index + 1]
  local current = tonumber(redis.call("GET", counter_key) or "0")

  if current >= limit then
    table.insert(outcomes, {0, current})
    return outcomes
  end

  if current == 0 then
    redis.call("ZREMRANGEBYSCORE", entries_key, "-inf", now_ms)
    redis.call("ZREM", entries_key, member)
    if redis.call("ZCARD", entries_key) >= max_entries then
      table.insert(outcomes, {-1, 0})
      return outcomes
    end
    redis.call("ZADD", entries_key, now_ms + window_ms, member)
  end

  current = redis.call("INCR", counter_key)
  if current == 1 then
    redis.call("PEXPIRE", counter_key, window_ms)
  end
  table.insert(outcomes, {1, current})
end

return outcomes
"#;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Allowed { limit: u32, remaining: u32 },
    Exhausted { limit: u32 },
    Capacity,
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub monthly_limit: u32,
    pub minute_limit: u32,
    pub daily_ip_limit: u32,
    pub global_daily_limit: u32,
    pub max_client_entries: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Decisions {
    pub minute: Option<Decision>,
    pub daily_ip: Option<Decision>,
    pub monthly: Option<Decision>,
    pub global_daily: Option<Decision>,
}

pub struct RateLimits {
    config: LimitConfig,
    backend: Backend,
}

enum Backend {
    Local(Box<LocalLimits>),
    Upstash(UpstashLimits),
}

struct LocalLimits {
    minute: Option<LocalRateLimiter>,
    daily_ip: Option<LocalRateLimiter>,
    monthly: Option<LocalRateLimiter>,
    global_daily: Option<LocalRateLimiter>,
}

struct UpstashLimits {
    client: reqwest::Client,
    url: String,
    token: String,
}

#[derive(Clone, Copy)]
struct LimitConfig {
    namespace: &'static str,
    minute: Option<Limit>,
    daily_ip: Option<Limit>,
    monthly: Option<Limit>,
    global_daily: Option<Limit>,
}

#[derive(Clone, Copy)]
struct Limit {
    name: &'static str,
    limit: u32,
    max_entries: usize,
    window: Duration,
}

struct ActiveLimit<'a> {
    kind: LimitKind,
    config: Limit,
    namespace: &'static str,
    fingerprint: &'a str,
}

#[derive(Clone, Copy)]
enum LimitKind {
    Minute,
    DailyIp,
    Monthly,
    GlobalDaily,
}

struct LocalRateLimiter {
    entries: Mutex<HashMap<u64, Usage>>,
    config: Limit,
}

#[derive(Clone, Copy)]
struct Usage {
    started_at: Instant,
    requests: u32,
}

impl RateLimits {
    pub fn from_env(client: &reqwest::Client, config: Config) -> Self {
        Self::from_env_scoped(client, config, "")
    }

    pub fn from_env_scoped(
        client: &reqwest::Client,
        config: Config,
        namespace: &'static str,
    ) -> Self {
        let config = LimitConfig::new_scoped(config, namespace);
        let url = std::env::var("UPSTASH_REDIS_REST_URL")
            .ok()
            .filter(|value| !value.is_empty());
        let token = std::env::var("UPSTASH_REDIS_REST_TOKEN")
            .ok()
            .filter(|value| !value.is_empty());

        let backend = match (url, token) {
            (Some(url), Some(token)) => {
                info!("Shared Upstash rate limits enabled");
                Backend::Upstash(UpstashLimits {
                    client: client.clone(),
                    url,
                    token,
                })
            }
            _ => {
                info!("Upstash not configured; using per-process rate limits");
                Backend::Local(Box::new(LocalLimits::new(config)))
            }
        };

        Self { config, backend }
    }

    #[cfg(test)]
    pub fn for_test_local(config: Config) -> Self {
        let config = LimitConfig::new(config);
        Self {
            config,
            backend: Backend::Local(Box::new(LocalLimits::new(config))),
        }
    }

    #[cfg(test)]
    pub fn for_test_upstash(
        client: &reqwest::Client,
        url: String,
        token: String,
        config: Config,
    ) -> Self {
        Self {
            config: LimitConfig::new(config),
            backend: Backend::Upstash(UpstashLimits {
                client: client.clone(),
                url,
                token,
            }),
        }
    }

    pub fn client_limits_enabled(&self) -> bool {
        self.config.minute.is_some()
            || self.config.daily_ip.is_some()
            || self.config.monthly.is_some()
    }

    pub fn ip_limits_enabled(&self) -> bool {
        self.config.minute.is_some() || self.config.daily_ip.is_some()
    }

    pub async fn check(
        &self,
        installation_fingerprint: Option<&str>,
        client_fingerprint: &str,
    ) -> Result<Decisions, String> {
        let active = self
            .config
            .active(installation_fingerprint, client_fingerprint);
        match &self.backend {
            Backend::Local(limits) => Ok(limits.check(&active)),
            Backend::Upstash(upstash) => upstash.check(&active).await,
        }
    }
}

impl LimitConfig {
    #[cfg(test)]
    fn new(config: Config) -> Self {
        Self::new_scoped(config, "")
    }

    fn new_scoped(config: Config, namespace: &'static str) -> Self {
        let client_entries = config.max_client_entries.max(1);
        Self {
            namespace,
            minute: Limit::enabled(
                "minute",
                config.minute_limit,
                client_entries,
                Duration::from_secs(60),
            ),
            daily_ip: Limit::enabled(
                "daily-ip",
                config.daily_ip_limit,
                client_entries,
                Duration::from_secs(24 * 60 * 60),
            ),
            monthly: Limit::enabled(
                "monthly",
                config.monthly_limit,
                client_entries,
                Duration::from_secs(30 * 24 * 60 * 60),
            ),
            global_daily: Limit::enabled(
                "global-daily",
                config.global_daily_limit,
                1,
                Duration::from_secs(24 * 60 * 60),
            ),
        }
    }

    fn active<'a>(
        &self,
        installation_fingerprint: Option<&'a str>,
        client_fingerprint: &'a str,
    ) -> Vec<ActiveLimit<'a>> {
        let mut active = Vec::with_capacity(4);
        if let Some(config) = self.minute {
            active.push(ActiveLimit {
                kind: LimitKind::Minute,
                config,
                namespace: self.namespace,
                fingerprint: client_fingerprint,
            });
        }
        if let Some(config) = self.daily_ip {
            active.push(ActiveLimit {
                kind: LimitKind::DailyIp,
                config,
                namespace: self.namespace,
                fingerprint: client_fingerprint,
            });
        }
        if let (Some(config), Some(fingerprint)) = (self.monthly, installation_fingerprint) {
            active.push(ActiveLimit {
                kind: LimitKind::Monthly,
                config,
                namespace: self.namespace,
                fingerprint,
            });
        }
        if let Some(config) = self.global_daily {
            active.push(ActiveLimit {
                kind: LimitKind::GlobalDaily,
                config,
                namespace: self.namespace,
                fingerprint: if self.namespace.is_empty() {
                    "global"
                } else {
                    self.namespace
                },
            });
        }
        active
    }
}

impl Limit {
    fn enabled(
        name: &'static str,
        limit: u32,
        max_entries: usize,
        window: Duration,
    ) -> Option<Self> {
        (limit > 0).then_some(Self {
            name,
            limit,
            max_entries,
            window,
        })
    }
}

impl LocalLimits {
    fn new(config: LimitConfig) -> Self {
        Self {
            minute: config.minute.map(LocalRateLimiter::new),
            daily_ip: config.daily_ip.map(LocalRateLimiter::new),
            monthly: config.monthly.map(LocalRateLimiter::new),
            global_daily: config.global_daily.map(LocalRateLimiter::new),
        }
    }

    fn check(&self, active: &[ActiveLimit<'_>]) -> Decisions {
        let now = Instant::now();
        let mut decisions = Decisions::default();
        for limit in active {
            let limiter = match limit.kind {
                LimitKind::Minute => self.minute.as_ref(),
                LimitKind::DailyIp => self.daily_ip.as_ref(),
                LimitKind::Monthly => self.monthly.as_ref(),
                LimitKind::GlobalDaily => self.global_daily.as_ref(),
            }
            .expect("active local rate limit is configured");
            let decision = limiter.check_at(limit.fingerprint, now);
            decisions.set(limit.kind, decision);
            if !matches!(decision, Decision::Allowed { .. }) {
                break;
            }
        }
        decisions
    }
}

impl LocalRateLimiter {
    fn new(config: Limit) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            config,
        }
    }

    fn check_at(&self, fingerprint: &str, now: Instant) -> Decision {
        let fingerprint = hash(fingerprint);
        let mut entries = self.entries.lock().expect("rate limiter lock poisoned");

        if let Some(usage) = entries.get_mut(&fingerprint) {
            if now.duration_since(usage.started_at) >= self.config.window {
                *usage = Usage {
                    started_at: now,
                    requests: 1,
                };
                return Decision::Allowed {
                    limit: self.config.limit,
                    remaining: self.config.limit - 1,
                };
            }
            if usage.requests >= self.config.limit {
                return Decision::Exhausted {
                    limit: self.config.limit,
                };
            }

            usage.requests += 1;
            return Decision::Allowed {
                limit: self.config.limit,
                remaining: self.config.limit - usage.requests,
            };
        }

        if entries.len() >= self.config.max_entries {
            entries.retain(|_, usage| now.duration_since(usage.started_at) < self.config.window);
            if entries.len() >= self.config.max_entries {
                return Decision::Capacity;
            }
        }

        entries.insert(
            fingerprint,
            Usage {
                started_at: now,
                requests: 1,
            },
        );
        Decision::Allowed {
            limit: self.config.limit,
            remaining: self.config.limit - 1,
        }
    }
}

impl UpstashLimits {
    async fn check(&self, active: &[ActiveLimit<'_>]) -> Result<Decisions, String> {
        if active.is_empty() {
            return Ok(Decisions::default());
        }
        let (body, kinds) = upstash_request(active);
        let response = self
            .client
            .post(self.url.trim_end_matches('/'))
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await
            .map_err(|error| format!("rate-limit store request failed: {error}"))?;
        if !response.status().is_success() {
            return Err(format!("rate-limit store returned {}", response.status()));
        }
        let body = response
            .json::<serde_json::Value>()
            .await
            .map_err(|error| format!("rate-limit store returned invalid JSON: {error}"))?;
        parse_upstash_response(&body, &kinds, active)
    }
}

impl Decisions {
    fn set(&mut self, kind: LimitKind, decision: Decision) {
        match kind {
            LimitKind::Minute => self.minute = Some(decision),
            LimitKind::DailyIp => self.daily_ip = Some(decision),
            LimitKind::Monthly => self.monthly = Some(decision),
            LimitKind::GlobalDaily => self.global_daily = Some(decision),
        }
    }
}

fn upstash_request(active: &[ActiveLimit<'_>]) -> (serde_json::Value, Vec<LimitKind>) {
    let mut command = Vec::with_capacity(3 + active.len() * 6);
    command.push(serde_json::Value::String("EVAL".to_string()));
    command.push(serde_json::Value::String(CHECK_LIMITS_SCRIPT.to_string()));
    command.push(serde_json::Value::String((active.len() * 2).to_string()));

    let mut fingerprints = Vec::with_capacity(active.len());
    let mut kinds = Vec::with_capacity(active.len());
    for limit in active {
        let fingerprint = hashed_fingerprint(limit.fingerprint);
        let namespace = if limit.namespace.is_empty() {
            String::new()
        } else {
            format!(":{}", limit.namespace)
        };
        command.push(serde_json::Value::String(format!(
            "{RATE_LIMIT_KEY_PREFIX}{namespace}:{}:counter:{fingerprint}",
            limit.config.name,
        )));
        command.push(serde_json::Value::String(format!(
            "{RATE_LIMIT_KEY_PREFIX}{namespace}:{}:entries",
            limit.config.name,
        )));
        fingerprints.push(fingerprint);
        kinds.push(limit.kind);
    }

    command.push(serde_json::Value::String(unix_millis().to_string()));
    for (limit, fingerprint) in active.iter().zip(fingerprints) {
        command.push(serde_json::Value::String(fingerprint));
        command.push(serde_json::Value::String(limit.config.limit.to_string()));
        command.push(serde_json::Value::String(
            limit.config.window.as_millis().to_string(),
        ));
        command.push(serde_json::Value::String(
            limit.config.max_entries.to_string(),
        ));
    }

    (serde_json::Value::Array(command), kinds)
}

fn parse_upstash_response(
    body: &serde_json::Value,
    kinds: &[LimitKind],
    active: &[ActiveLimit<'_>],
) -> Result<Decisions, String> {
    let outcomes = body
        .get("result")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "rate-limit store returned no result".to_string())?;
    if outcomes.is_empty() || outcomes.len() > active.len() || outcomes.len() > kinds.len() {
        return Err("rate-limit store returned an unexpected result count".to_string());
    }

    let mut decisions = Decisions::default();
    for ((outcome, kind), limit) in outcomes.iter().zip(kinds).zip(active) {
        let values = outcome
            .as_array()
            .filter(|values| values.len() == 2)
            .ok_or_else(|| "rate-limit store returned a malformed result".to_string())?;
        let code = value_as_i64(&values[0])
            .ok_or_else(|| "rate-limit store returned an invalid decision".to_string())?;
        let count = value_as_u64(&values[1])
            .ok_or_else(|| "rate-limit store returned an invalid count".to_string())?;
        let decision = match code {
            1 => Decision::Allowed {
                limit: limit.config.limit,
                remaining: limit.config.limit.saturating_sub(count as u32),
            },
            0 => Decision::Exhausted {
                limit: limit.config.limit,
            },
            -1 => Decision::Capacity,
            _ => return Err("rate-limit store returned an unknown decision".to_string()),
        };
        decisions.set(*kind, decision);
    }
    Ok(decisions)
}

fn hashed_fingerprint(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn hash(value: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn value_as_i64(value: &serde_json::Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
}

fn value_as_u64(value: &serde_json::Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
}

#[cfg(test)]
mod tests {
    use super::{
        parse_upstash_response, upstash_request, Config, Decision, LimitConfig, LocalLimits,
    };
    use std::time::Instant;

    fn config() -> LimitConfig {
        LimitConfig::new(Config {
            monthly_limit: 2,
            minute_limit: 2,
            daily_ip_limit: 3,
            global_daily_limit: 4,
            max_client_entries: 10,
        })
    }

    #[test]
    fn local_limits_enforce_and_reset_request_window() {
        let config = config();
        let limiter = LocalLimits::new(config);
        let active = config.active(Some("installation:one"), "address:one");
        let now = Instant::now();
        let minute = limiter.minute.as_ref().unwrap();

        assert_eq!(
            minute.check_at("address:one", now),
            Decision::Allowed {
                limit: 2,
                remaining: 1
            }
        );
        assert_eq!(
            minute.check_at("address:one", now),
            Decision::Allowed {
                limit: 2,
                remaining: 0
            }
        );
        assert_eq!(
            minute.check_at("address:one", now),
            Decision::Exhausted { limit: 2 }
        );
        assert!(matches!(
            minute.check_at("address:one", now + active[0].config.window),
            Decision::Allowed {
                limit: 2,
                remaining: 1
            }
        ));
    }

    #[test]
    fn local_limits_bound_and_prune_tracked_fingerprints() {
        let config = LimitConfig::new(Config {
            max_client_entries: 1,
            ..Config {
                monthly_limit: 0,
                minute_limit: 2,
                daily_ip_limit: 0,
                global_daily_limit: 0,
                max_client_entries: 1,
            }
        });
        let limiter = LocalLimits::new(config);
        let minute = limiter.minute.as_ref().unwrap();
        let now = Instant::now();

        assert!(matches!(
            minute.check_at("one", now),
            Decision::Allowed { .. }
        ));
        assert_eq!(minute.check_at("two", now), Decision::Capacity);
        assert!(matches!(
            minute.check_at("two", now + minute.config.window),
            Decision::Allowed { .. }
        ));
    }

    #[test]
    fn redis_request_hashes_fingerprints_and_includes_all_limits() {
        let config = config();
        let active = config.active(Some("installation:secret"), "address:192.0.2.1");
        let (request, kinds) = upstash_request(&active);
        let command = request.as_array().unwrap();
        let encoded = serde_json::to_string(&request).unwrap();

        assert_eq!(command[0], "EVAL");
        assert_eq!(command[2], "8");
        assert_eq!(kinds.len(), 4);
        assert!(!encoded.contains("installation:secret"));
        assert!(!encoded.contains("192.0.2.1"));
        assert!(encoded.contains("jst:rate-limit:minute:counter:"));
        assert!(encoded.contains("jst:rate-limit:global-daily:entries"));
    }

    #[test]
    fn redis_request_scopes_demo_counters_away_from_cli_counters() {
        let config = LimitConfig::new_scoped(
            Config {
                monthly_limit: 2,
                minute_limit: 2,
                daily_ip_limit: 3,
                global_daily_limit: 4,
                max_client_entries: 10,
            },
            "demo",
        );
        let active = config.active(Some("browser:secret"), "address:192.0.2.1");
        let (request, _) = upstash_request(&active);
        let encoded = serde_json::to_string(&request).unwrap();

        assert!(encoded.contains("jst:rate-limit:demo:minute:counter:"));
        assert!(encoded.contains("jst:rate-limit:demo:global-daily:entries"));
        assert!(!encoded.contains("jst:rate-limit:minute:counter:"));
    }

    #[test]
    fn parses_allowed_and_exhausted_redis_results() {
        let config = config();
        let active = config.active(Some("installation:one"), "address:one");
        let (_, kinds) = upstash_request(&active);
        let decisions = parse_upstash_response(
            &serde_json::json!({"result": [[1, 1], [1, "2"], [0, 2]]}),
            &kinds,
            &active,
        )
        .unwrap();

        assert_eq!(
            decisions.minute,
            Some(Decision::Allowed {
                limit: 2,
                remaining: 1
            })
        );
        assert_eq!(
            decisions.daily_ip,
            Some(Decision::Allowed {
                limit: 3,
                remaining: 1
            })
        );
        assert_eq!(decisions.monthly, Some(Decision::Exhausted { limit: 2 }));
        assert_eq!(decisions.global_daily, None);
    }

    #[test]
    fn rejects_malformed_redis_results() {
        let config = config();
        let active = config.active(Some("installation:one"), "address:one");
        let (_, kinds) = upstash_request(&active);

        assert!(parse_upstash_response(&serde_json::json!({}), &kinds, &active).is_err());
        assert!(
            parse_upstash_response(&serde_json::json!({"result": [[7, 1]]}), &kinds, &active)
                .is_err()
        );
    }
}
