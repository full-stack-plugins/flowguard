//! Explicit process-local attempt storage. No persistence or authority grants.
use crate::gate::PendingGate;
use guardengine::integration::RunBinding;
use std::{collections::BTreeMap, sync::Mutex};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    Conflict,
    Stale,
    Missing,
    Invalid,
    Capacity,
    Poisoned,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptHandle {
    pub run_id: String,
    pub generation: u64,
}
#[derive(Clone)]
pub(crate) struct WorkIdentity {
    pub target: String,
    pub digest: String,
    pub run_id: String,
    pub binding: RunBinding,
    pub required: Vec<String>,
}
struct Attempt {
    work: WorkIdentity,
    handle: AttemptHandle,
    bytes: Option<Vec<u8>>,
}
struct Head {
    generation: u64,
    digest: String,
    published: Option<String>,
}
#[derive(Default)]
struct State {
    heads: BTreeMap<String, Head>,
    attempts: BTreeMap<String, Attempt>,
    keys: BTreeMap<(String, String), String>,
    bytes: usize,
}
#[derive(Default)]
pub struct MemoryRunStore {
    state: Mutex<State>,
}
impl MemoryRunStore {
    pub fn advance(&self, gate: &PendingGate, expected: u64) -> Result<u64, StoreError> {
        let work = gate.store_identity();
        let mut state = self.state.lock().map_err(|_| StoreError::Poisoned)?;
        if state.heads.get(&work.target).map_or(0, |h| h.generation) != expected {
            return Err(StoreError::Stale);
        }
        if !state.heads.contains_key(&work.target) && state.heads.len() >= 1024 {
            return Err(StoreError::Capacity);
        }
        let generation = expected.checked_add(1).ok_or(StoreError::Capacity)?;
        state.heads.insert(
            work.target,
            Head {
                generation,
                digest: work.digest,
                published: None,
            },
        );
        Ok(generation)
    }
    pub fn reserve(
        &self,
        gate: &PendingGate,
        key: &str,
        generation: u64,
    ) -> Result<AttemptHandle, StoreError> {
        if key.trim().is_empty() || key.len() > 128 {
            return Err(StoreError::Invalid);
        }
        let work = gate.store_identity();
        let mut state = self.state.lock().map_err(|_| StoreError::Poisoned)?;
        let index = (work.target.clone(), key.into());
        if let Some(id) = state.keys.get(&index) {
            let old = state.attempts.get(id).ok_or(StoreError::Missing)?;
            return if old.work.digest == work.digest && old.handle.generation == generation {
                Ok(old.handle.clone())
            } else {
                Err(StoreError::Conflict)
            };
        }
        if state.attempts.contains_key(&work.run_id) {
            return Err(StoreError::Conflict);
        }
        let head = state.heads.get(&work.target).ok_or(StoreError::Missing)?;
        if head.generation != generation || head.digest != work.digest {
            return Err(StoreError::Stale);
        }
        if state.attempts.len() >= 1024 {
            return Err(StoreError::Capacity);
        }
        let handle = AttemptHandle {
            run_id: work.run_id.clone(),
            generation,
        };
        state.keys.insert(index, work.run_id.clone());
        state.attempts.insert(
            work.run_id.clone(),
            Attempt {
                work,
                handle: handle.clone(),
                bytes: None,
            },
        );
        Ok(handle)
    }
}
impl MemoryRunStore {
    /// Records structurally validated bytes. It does not authenticate evidence or grant eligibility.
    pub fn append(&self, run_id: &str, bytes: &[u8]) -> Result<(), StoreError> {
        use guardengine::integration::{EvidenceProfile, load_envelope_json};
        if bytes.len() > 1024 * 1024 {
            return Err(StoreError::Capacity);
        }
        let envelope = load_envelope_json(bytes, EvidenceProfile::EngineBacked)
            .map_err(|_| StoreError::Invalid)?;
        let mut state = self.state.lock().map_err(|_| StoreError::Poisoned)?;
        let attempt = state.attempts.get(run_id).ok_or(StoreError::Missing)?;
        if envelope.run_id != run_id
            || envelope.binding != attempt.work.binding
            || envelope.coverage.required_scopes != attempt.work.required
            || envelope.producer.guard != "flowguard"
            || envelope.producer.analyzer_id != "flowguard.stage-gates"
            || envelope.producer.version != env!("CARGO_PKG_VERSION")
            || envelope.producer.analyzer_version != env!("CARGO_PKG_VERSION")
        {
            return Err(StoreError::Invalid);
        }
        if let Some(old) = &attempt.bytes {
            return if old == bytes {
                Ok(())
            } else {
                Err(StoreError::Conflict)
            };
        }
        if state.bytes.saturating_add(bytes.len()) > 32 * 1024 * 1024 {
            return Err(StoreError::Capacity);
        }
        state.bytes += bytes.len();
        state
            .attempts
            .get_mut(run_id)
            .ok_or(StoreError::Missing)?
            .bytes = Some(bytes.to_vec());
        Ok(())
    }
    /// Single assignment within a generation; a retry must advance to replace a publication.
    pub fn publish(&self, run_id: &str, expected: u64) -> Result<(), StoreError> {
        let mut state = self.state.lock().map_err(|_| StoreError::Poisoned)?;
        let attempt = state.attempts.get(run_id).ok_or(StoreError::Missing)?;
        if attempt.bytes.is_none() {
            return Err(StoreError::Missing);
        }
        let target = attempt.work.target.clone();
        let digest = attempt.work.digest.clone();
        let generation = attempt.handle.generation;
        let head = state.heads.get_mut(&target).ok_or(StoreError::Missing)?;
        if head.generation != expected || generation != expected || head.digest != digest {
            return Err(StoreError::Stale);
        }
        if head.published.as_deref().is_some_and(|id| id != run_id) {
            return Err(StoreError::Conflict);
        }
        head.published = Some(run_id.into());
        Ok(())
    }
    pub fn current(&self, gate: &PendingGate) -> Result<Option<Vec<u8>>, StoreError> {
        let work = gate.store_identity();
        let state = self.state.lock().map_err(|_| StoreError::Poisoned)?;
        let Some(head) = state.heads.get(&work.target) else {
            return Ok(None);
        };
        if head.digest != work.digest {
            return Ok(None);
        }
        Ok(head
            .published
            .as_ref()
            .and_then(|id| state.attempts.get(id))
            .and_then(|a| a.bytes.clone()))
    }
    /// Detached immutable bytes, deterministically ordered by run ID; no eligibility cache.
    pub fn history(&self, gate: &PendingGate) -> Result<Vec<Vec<u8>>, StoreError> {
        let work = gate.store_identity();
        let state = self.state.lock().map_err(|_| StoreError::Poisoned)?;
        Ok(state
            .attempts
            .values()
            .filter(|a| a.work.target == work.target)
            .filter_map(|a| a.bytes.clone())
            .collect())
    }
}
