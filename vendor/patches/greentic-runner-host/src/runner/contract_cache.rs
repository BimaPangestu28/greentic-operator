use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

const DEFAULT_CONTRACT_CACHE_MAX_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractSnapshot {
    pub resolved_digest: String,
    pub component_id: String,
    pub operation_id: String,
    pub validate_output: bool,
    pub strict: bool,
    pub describe_hash: Option<String>,
    pub schema_hash: Option<String>,
}

impl ContractSnapshot {
    pub fn new(
        resolved_digest: String,
        component_id: String,
        operation_id: String,
        validate_output: bool,
        strict: bool,
    ) -> Self {
        Self {
            resolved_digest,
            component_id,
            operation_id,
            validate_output,
            strict,
            describe_hash: None,
            schema_hash: None,
        }
    }

    fn estimated_bytes(&self) -> u64 {
        let mut bytes = 128_u64;
        bytes = bytes.saturating_add(self.resolved_digest.len() as u64);
        bytes = bytes.saturating_add(self.component_id.len() as u64);
        bytes = bytes.saturating_add(self.operation_id.len() as u64);
        if let Some(value) = &self.describe_hash {
            bytes = bytes.saturating_add(value.len() as u64);
        }
        if let Some(value) = &self.schema_hash {
            bytes = bytes.saturating_add(value.len() as u64);
        }
        bytes
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContractCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub entries: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct ContractCache {
    max_bytes: u64,
    state: Arc<Mutex<ContractCacheState>>,
}

#[derive(Debug, Default)]
struct ContractCacheState {
    entries: HashMap<String, ContractCacheEntry>,
    lru: VecDeque<String>,
    total_bytes: u64,
    hits: u64,
    misses: u64,
    evictions: u64,
}

#[derive(Debug)]
struct ContractCacheEntry {
    snapshot: Arc<ContractSnapshot>,
    bytes_estimate: u64,
}

impl ContractCache {
    pub fn new(max_bytes: u64) -> Self {
        Self {
            max_bytes,
            state: Arc::new(Mutex::new(ContractCacheState::default())),
        }
    }

    pub fn from_env() -> Self {
        let max_bytes = std::env::var("GREENTIC_CONTRACT_CACHE_MAX_BYTES")
            .ok()
            .and_then(|raw| raw.trim().parse::<u64>().ok())
            .unwrap_or(DEFAULT_CONTRACT_CACHE_MAX_BYTES);
        Self::new(max_bytes)
    }

    pub fn get(&self, key: &str) -> Option<Arc<ContractSnapshot>> {
        let mut state = self.state.lock();
        if state.entries.contains_key(key) {
            state.hits = state.hits.saturating_add(1);
            let snapshot = state
                .entries
                .get(key)
                .map(|entry| Arc::clone(&entry.snapshot));
            touch_lru(&mut state.lru, key);
            return snapshot;
        }
        state.misses = state.misses.saturating_add(1);
        None
    }

    pub fn insert(&self, key: String, snapshot: Arc<ContractSnapshot>) {
        let mut state = self.state.lock();
        if let Some(existing) = state.entries.remove(&key) {
            state.total_bytes = state.total_bytes.saturating_sub(existing.bytes_estimate);
            remove_lru(&mut state.lru, &key);
        }
        let bytes_estimate = snapshot.estimated_bytes();
        state.entries.insert(
            key.clone(),
            ContractCacheEntry {
                snapshot,
                bytes_estimate,
            },
        );
        state.total_bytes = state.total_bytes.saturating_add(bytes_estimate);
        state.lru.push_front(key);
        self.evict_if_needed(&mut state);
    }

    pub fn stats(&self) -> ContractCacheStats {
        let state = self.state.lock();
        ContractCacheStats {
            hits: state.hits,
            misses: state.misses,
            evictions: state.evictions,
            entries: state.entries.len() as u64,
            total_bytes: state.total_bytes,
        }
    }

    fn evict_if_needed(&self, state: &mut ContractCacheState) {
        if self.max_bytes == 0 {
            return;
        }
        while state.total_bytes > self.max_bytes {
            let Some(candidate) = state.lru.pop_back() else {
                break;
            };
            if let Some(entry) = state.entries.remove(&candidate) {
                state.total_bytes = state.total_bytes.saturating_sub(entry.bytes_estimate);
                state.evictions = state.evictions.saturating_add(1);
            }
        }
    }
}

fn touch_lru(lru: &mut VecDeque<String>, key: &str) {
    if let Some(pos) = lru.iter().position(|item| item == key) {
        lru.remove(pos);
        lru.push_front(key.to_string());
    }
}

fn remove_lru(lru: &mut VecDeque<String>, key: &str) {
    if let Some(pos) = lru.iter().position(|item| item == key) {
        lru.remove(pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_tracks_hits_and_lru_eviction() {
        let cache = ContractCache::new(256);
        let key_a = "sha256:a::component.alpha::run".to_string();
        let key_b = "sha256:b::component.beta::run".to_string();

        cache.insert(
            key_a.clone(),
            Arc::new(ContractSnapshot::new(
                "sha256:a".to_string(),
                "component.alpha".to_string(),
                "run".to_string(),
                true,
                true,
            )),
        );
        assert!(cache.get(&key_a).is_some());
        cache.insert(
            key_b.clone(),
            Arc::new(ContractSnapshot::new(
                "sha256:b".to_string(),
                "component.beta".to_string(),
                "run".to_string(),
                true,
                true,
            )),
        );
        let stats = cache.stats();
        assert!(stats.hits >= 1);
        assert!(stats.entries >= 1);
        assert!(cache.get(&key_b).is_some());
    }
}
