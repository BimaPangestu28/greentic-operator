use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug)]
pub struct OperatorMetrics {
    pub resolve_attempts: AtomicU64,
    pub resolve_errors: AtomicU64,
    pub invoke_attempts: AtomicU64,
    pub invoke_errors: AtomicU64,
    pub cbor_decode_errors: AtomicU64,
}

#[derive(Clone, Debug)]
pub struct OperatorMetricsSnapshot {
    pub resolve_attempts: u64,
    pub resolve_errors: u64,
    pub invoke_attempts: u64,
    pub invoke_errors: u64,
    pub cbor_decode_errors: u64,
}

impl Default for OperatorMetrics {
    fn default() -> Self {
        Self {
            resolve_attempts: AtomicU64::new(0),
            resolve_errors: AtomicU64::new(0),
            invoke_attempts: AtomicU64::new(0),
            invoke_errors: AtomicU64::new(0),
            cbor_decode_errors: AtomicU64::new(0),
        }
    }
}

impl OperatorMetrics {
    pub fn snapshot(&self) -> OperatorMetricsSnapshot {
        OperatorMetricsSnapshot {
            resolve_attempts: self.resolve_attempts.load(Ordering::Relaxed),
            resolve_errors: self.resolve_errors.load(Ordering::Relaxed),
            invoke_attempts: self.invoke_attempts.load(Ordering::Relaxed),
            invoke_errors: self.invoke_errors.load(Ordering::Relaxed),
            cbor_decode_errors: self.cbor_decode_errors.load(Ordering::Relaxed),
        }
    }
}
