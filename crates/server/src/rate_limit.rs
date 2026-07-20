use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Allowed { limit: u32, remaining: u32 },
    Exhausted { limit: u32 },
    Capacity,
}

pub struct RateLimiter {
    entries: Mutex<HashMap<u64, Usage>>,
    limit: u32,
    max_entries: usize,
    window: Duration,
}

#[derive(Clone, Copy)]
struct Usage {
    started_at: Instant,
    requests: u32,
}

impl RateLimiter {
    pub fn new(limit: u32, max_entries: usize, window: Duration) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            limit,
            max_entries,
            window,
        }
    }

    pub fn check(&self, fingerprint: &str) -> Decision {
        self.check_at(fingerprint, Instant::now())
    }

    fn check_at(&self, fingerprint: &str, now: Instant) -> Decision {
        let fingerprint = hash(fingerprint);
        let mut entries = self.entries.lock().expect("rate limiter lock poisoned");

        if let Some(usage) = entries.get_mut(&fingerprint) {
            if now.duration_since(usage.started_at) >= self.window {
                *usage = Usage {
                    started_at: now,
                    requests: 1,
                };
                return Decision::Allowed {
                    limit: self.limit,
                    remaining: self.limit - 1,
                };
            }
            if usage.requests >= self.limit {
                return Decision::Exhausted { limit: self.limit };
            }

            usage.requests += 1;
            return Decision::Allowed {
                limit: self.limit,
                remaining: self.limit - usage.requests,
            };
        }

        if entries.len() >= self.max_entries {
            entries.retain(|_, usage| now.duration_since(usage.started_at) < self.window);
            if entries.len() >= self.max_entries {
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
            limit: self.limit,
            remaining: self.limit - 1,
        }
    }
}

fn hash(value: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::{Decision, RateLimiter};
    use std::time::{Duration, Instant};

    #[test]
    fn enforces_and_resets_request_window() {
        let limiter = RateLimiter::new(2, 10, Duration::from_secs(60));
        let now = Instant::now();

        assert_eq!(
            limiter.check_at("one", now),
            Decision::Allowed {
                limit: 2,
                remaining: 1
            }
        );
        assert_eq!(
            limiter.check_at("one", now),
            Decision::Allowed {
                limit: 2,
                remaining: 0
            }
        );
        assert_eq!(
            limiter.check_at("one", now),
            Decision::Exhausted { limit: 2 }
        );
        assert_eq!(
            limiter.check_at("one", now + Duration::from_secs(60)),
            Decision::Allowed {
                limit: 2,
                remaining: 1
            }
        );
    }

    #[test]
    fn bounds_and_prunes_tracked_fingerprints() {
        let limiter = RateLimiter::new(2, 1, Duration::from_secs(60));
        let now = Instant::now();

        assert!(matches!(
            limiter.check_at("one", now),
            Decision::Allowed { .. }
        ));
        assert_eq!(limiter.check_at("two", now), Decision::Capacity);
        assert!(matches!(
            limiter.check_at("two", now + Duration::from_secs(60)),
            Decision::Allowed { .. }
        ));
    }
}
