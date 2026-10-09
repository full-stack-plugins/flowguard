//! Controller-owned dependency invalidation for the process-local MemoryRunStore.
//! Snapshots and deltas only withdraw publications. They never establish eligibility.
use crate::{
    dependencies::StageGraph,
    gate::PendingGate,
    run_store::{MemoryRunStore, StoreError, WorkIdentity},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
const MAX_NODES: usize = 256;
const MAX_BYTES: usize = 16 * 1024 * 1024;
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct ContextDigests {
    pub source: String,
    pub rules: String,
    pub analyzer: String,
    pub config: String,
    pub coverage: String,
    pub baseline: String,
    pub dependencies: String,
}
/// Protected-controller observations, not caller self-authentication or a cached approval.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Freshness {
    pub observed_at: i64,
    pub expires_at: i64,
    pub revoked: bool,
    pub artifacts_available: bool,
}
impl Freshness {
    fn invalid(self, now: i64) -> bool {
        self.revoked || !self.artifacts_available || now >= self.expires_at
    }
}
pub struct NodeInput<'a> {
    pub node_id: &'a str,
    pub gate: &'a PendingGate,
    pub generation: u64,
    pub context: &'a ContextDigests,
    pub freshness: Freshness,
}
#[derive(Debug, PartialEq, Eq)]
pub enum InvalidationError {
    Budget,
    Invalid,
    ClockRollback,
    Unregistered,
    Store(StoreError),
}
impl From<StoreError> for InvalidationError {
    fn from(v: StoreError) -> Self {
        Self::Store(v)
    }
}
struct FrozenNode {
    work: WorkIdentity,
    generation: u64,
    context: ContextDigests,
    freshness: Freshness,
}
/// Exact graph-node/work mapping prepared independently of completed gate reports.
pub struct FrozenInputs {
    graph: StageGraph,
    nodes: BTreeMap<String, FrozenNode>,
    observed_at: i64,
}
/// Opaque old work/generation batch: callers cannot shrink its computed closure.
/// ```compile_fail
/// let forged: flowguard::invalidation::Invalidation = serde_json::from_str("{}").unwrap();
/// ```
pub struct Invalidation {
    affected: BTreeSet<String>,
    old_heads: Vec<(WorkIdentity, u64)>,
}
fn bounded(value: &impl Serialize, limit: usize) -> Result<usize, InvalidationError> {
    struct Sink(usize);
    impl std::io::Write for Sink {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_sub(b.len())
                .ok_or(std::io::ErrorKind::InvalidInput)?;
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut sink = Sink(limit);
    serde_json::to_writer(&mut sink, value).map_err(|_| InvalidationError::Budget)?;
    Ok(limit - sink.0)
}
impl FrozenInputs {
    pub fn freeze(
        graph: &StageGraph,
        inputs: &[NodeInput<'_>],
        observed_at: i64,
    ) -> Result<Self, InvalidationError> {
        if graph.nodes().len() > MAX_NODES || inputs.len() != graph.nodes().len() {
            return Err(InvalidationError::Budget);
        }
        if observed_at < 0 {
            return Err(InvalidationError::Invalid);
        }
        let mut bytes = bounded(graph, 512 * 1024)?;
        let mut seen = BTreeSet::new();
        for i in inputs {
            if i.node_id.len() > 256
                || !graph.nodes().contains_key(i.node_id)
                || !seen.insert(i.node_id)
                || i.freshness.observed_at < 0
                || i.freshness.observed_at > observed_at
                || i.freshness.expires_at < 0
            {
                return Err(InvalidationError::Invalid);
            }
            let c = i.context;
            if [
                &c.source,
                &c.rules,
                &c.analyzer,
                &c.config,
                &c.coverage,
                &c.baseline,
                &c.dependencies,
            ]
            .iter()
            .any(|v| !crate::valid_digest(v))
            {
                return Err(InvalidationError::Invalid);
            }
            bytes = bytes.checked_add(bounded(i.context, 1024)? + i.node_id.len()).ok_or(InvalidationError::Budget)?;
            bytes = bytes
                .checked_add(
                    i.gate
                        .invalidation_budget_bytes()
                        .map_err(|_| InvalidationError::Budget)?,
                )
                .ok_or(InvalidationError::Budget)?;
            if bytes > MAX_BYTES {
                return Err(InvalidationError::Budget);
            }
        }
        // All caller-sized graph/work input has passed borrowed budgets before owned identities.
        let mut nodes = BTreeMap::new();
        let mut targets = BTreeSet::new();
        for i in inputs {
            let work = i.gate.store_identity();
            if !targets.insert(work.target.clone()) {
                return Err(InvalidationError::Invalid);
            }
            nodes.insert(
                i.node_id.into(),
                FrozenNode {
                    work,
                    generation: i.generation,
                    context: i.context.clone(),
                    freshness: i.freshness,
                },
            );
        }
        Ok(Self {
            graph: graph.clone(),
            nodes,
            observed_at,
        })
    }
    pub fn changes(&self, current: &Self, now: i64) -> Result<Invalidation, InvalidationError> {
        if current.observed_at < self.observed_at || now < current.observed_at {
            return Err(InvalidationError::ClockRollback);
        }
        let mut affected = BTreeSet::new();
        for id in self.nodes.keys().chain(current.nodes.keys()) {
            let changed = match (self.nodes.get(id), current.nodes.get(id)) {
                (Some(a), Some(b)) => {
                    a.work.digest != b.work.digest
                        || a.work.run_id != b.work.run_id
                        || a.generation != b.generation
                        || a.context != b.context
                        || a.freshness != b.freshness
                        || a.freshness.invalid(now)
                        || b.freshness.invalid(now)
                        || self.graph.nodes().get(id) != current.graph.nodes().get(id)
                }
                _ => true,
            };
            if changed {
                affected.insert(id.clone());
            }
        }
        // The union can contain cycles even though each graph is a DAG; monotone finite sets
        // compute its conservative closure without recursion or executing graph nodes.
        loop {
            let before = affected.len();
            for (id, node) in self.graph.nodes().iter().chain(current.graph.nodes()) {
                if node.dependencies.iter().any(|d| affected.contains(d)) {
                    affected.insert(id.clone());
                }
            }
            if before == affected.len() {
                break;
            }
        }
        let mut old_heads = Vec::new();
        for id in &affected {
            if let Some(old) = self.nodes.get(id) {
                if old.generation == 0 {
                    return Err(InvalidationError::Unregistered);
                }
                old_heads.push((old.work.clone(), old.generation));
            }
        }
        Ok(Invalidation {
            affected,
            old_heads,
        })
    }
}
impl Invalidation {
    pub fn affected(&self) -> &BTreeSet<String> {
        &self.affected
    }
    /// All expected old heads are checked under one store mutex before any head is changed.
    /// Unaffected work and all immutable history bytes remain intact; no durable-store claim.
    pub fn apply(&self, store: &MemoryRunStore) -> Result<(), InvalidationError> {
        Ok(store.invalidate_batch(&self.old_heads)?)
    }
}
