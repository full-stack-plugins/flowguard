//! Controller-pinned local profile; not an identity provider.
use crate::{dependencies::StageGraph, input_limits::AllowedRoot, obligations::FrozenObligations};
#[derive(Debug)]
pub struct PolicySnapshot {
    raw_digest: String,
    frozen: FrozenObligations,
}
impl PolicySnapshot {
    pub fn raw_digest(&self) -> &str {
        &self.raw_digest
    }
    pub fn frozen(&self) -> &FrozenObligations {
        &self.frozen
    }
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: String,
    context_digest: String,
    baseline_digest: String,
    graph_digest: String,
    obligations: Vec<crate::obligations::EvidenceObligation>,
}
pub fn load_and_freeze(
    root: &AllowedRoot,
    relative: &str,
    expected_raw_digest: &str,
    context_digest: &str,
    baseline_digest: &str,
    graph: &StageGraph,
) -> Result<PolicySnapshot, &'static str> {
    let bytes = root.read(relative).map_err(|_| "policy read rejected")?;
    if bytes.len() > 1024 * 1024 {
        return Err("policy byte budget");
    }
    let raw_digest = crate::digest(&bytes);
    if !crate::valid_digest(expected_raw_digest) || raw_digest != expected_raw_digest {
        return Err("policy pin mismatch");
    }
    let document: Document =
        serde_json::from_slice(&bytes).map_err(|_| "policy schema rejected")?;
    if document.version != "flowguard.policy/v1alpha1"
        || document.context_digest != context_digest
        || document.baseline_digest != baseline_digest
        || document.graph_digest
            != crate::digest(&serde_json::to_vec(graph).map_err(|_| "graph encoding")?)
    {
        return Err("policy binding mismatch");
    }
    let unique: std::collections::BTreeSet<_> = document.obligations.iter().collect();
    if unique.len() != document.obligations.len() {
        return Err("duplicate obligation");
    }
    let frozen =
        crate::obligations::freeze(graph, document.obligations, context_digest, baseline_digest)?;
    Ok(PolicySnapshot { raw_digest, frozen })
}
