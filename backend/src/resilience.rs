use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Minimal Closed -> Open -> Half-Open circuit breaker for wrapping flaky external calls
/// (Redis/NATS publishes here). Hand-rolled rather than pulling in a crate: the whole
/// state machine is ~30 lines and a dependency buys nothing extra at this scale.
pub struct CircuitBreaker {
    failure_count: AtomicU32,
    opened_at_epoch_secs: AtomicU64,
    failure_threshold: u32,
    cooldown_seconds: u64,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, cooldown_seconds: u64) -> Self {
        Self {
            failure_count: AtomicU32::new(0),
            opened_at_epoch_secs: AtomicU64::new(0),
            failure_threshold,
            cooldown_seconds,
        }
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Returns true if a call should be allowed through right now.
    pub fn allow_request(&self) -> bool {
        let opened_at = self.opened_at_epoch_secs.load(Ordering::Relaxed);
        if opened_at == 0 {
            return true; // Closed
        }
        // Open: check whether the cooldown has elapsed (-> Half-Open, allow one probe through).
        Self::now_secs().saturating_sub(opened_at) >= self.cooldown_seconds
    }

    pub fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        self.opened_at_epoch_secs.store(0, Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        if failures >= self.failure_threshold {
            self.opened_at_epoch_secs
                .store(Self::now_secs(), Ordering::Relaxed);
        }
    }

    pub fn is_open(&self) -> bool {
        !self.allow_request()
    }
}
