use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

const WINDOW_SECONDS: u64 = 60;
const MAX_TRACKED_KEYS: usize = 10_000;

#[derive(Debug, Clone, Copy)]
struct Window {
    start: u64,
    count: u32,
}

#[derive(Clone, Default)]
pub struct RateLimiter {
    windows: Arc<Mutex<HashMap<String, Window>>>,
}

impl RateLimiter {
    #[must_use = "ignoring the answer silently disables rate limiting"]
    pub fn check(&self, key: &str, limit: u32, now_secs: u64) -> bool {
        if limit == 0 {
            return true;
        }

        let mut windows = self
            .windows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if windows.len() > MAX_TRACKED_KEYS {
            windows.retain(|_, window| now_secs.saturating_sub(window.start) < 2 * WINDOW_SECONDS);
        }

        let window = windows.entry(key.to_owned()).or_insert(Window {
            start: now_secs,
            count: 0,
        });

        if now_secs.saturating_sub(window.start) >= WINDOW_SECONDS {
            *window = Window {
                start: now_secs,
                count: 0,
            };
        }

        if window.count >= limit {
            return false;
        }

        window.count += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_the_limit_then_blocks() {
        let limiter = RateLimiter::default();

        assert!(limiter.check("1.2.3.4", 2, 1_000));
        assert!(limiter.check("1.2.3.4", 2, 1_001));
        assert!(!limiter.check("1.2.3.4", 2, 1_002));
        assert!(limiter.check("5.6.7.8", 2, 1_002));
    }

    #[test]
    fn resets_after_the_window() {
        let limiter = RateLimiter::default();

        assert!(limiter.check("key", 1, 1_000));
        assert!(!limiter.check("key", 1, 1_030));
        assert!(limiter.check("key", 1, 1_061));
    }
}
