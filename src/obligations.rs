use crate::dependencies::StageGraph;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceObligation {
    pub stage_id: String,
    pub guard: String,
    pub coverage: BTreeSet<String>,
    pub rules_digest: String,
    pub analyzer_version: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    version: String,
    context_digest: String,
    baseline_digest: String,
    graph_digest: String,
    obligations: BTreeSet<EvidenceObligation>,
    missing_stages: BTreeSet<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FrozenObligations {
    #[serde(flatten)]
    payload: Payload,
    digest: String,
}
impl FrozenObligations {
    pub fn obligations(&self) -> &BTreeSet<EvidenceObligation> {
        &self.payload.obligations
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn missing_stages(&self) -> &BTreeSet<String> {
        &self.payload.missing_stages
    }
    pub fn missing_providers(&self, present: &BTreeSet<String>) -> BTreeSet<String> {
        self.payload
            .obligations
            .iter()
            .map(|o| o.guard.clone())
            .filter(|g| !present.contains(g))
            .collect()
    }
    pub fn from_json(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > 1024 * 1024 {
            return Err("payload budget");
        }
        // Decode directly into a closed struct so duplicate keys cannot disappear
        // during Value normalization before the canonical digest comparison.
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            version: String,
            context_digest: String,
            baseline_digest: String,
            graph_digest: String,
            obligations: BTreeSet<EvidenceObligation>,
            missing_stages: BTreeSet<String>,
            digest: String,
        }
        let wire: Wire = serde_json::from_slice(bytes).map_err(|_| "invalid payload schema")?;
        let digest = wire.digest;
        let payload = Payload {
            version: wire.version,
            context_digest: wire.context_digest,
            baseline_digest: wire.baseline_digest,
            graph_digest: wire.graph_digest,
            obligations: wire.obligations,
            missing_stages: wire.missing_stages,
        };
        validate(&payload)?;
        if crate::digest(&serde_json::to_vec(&payload).map_err(|_| "encoding")?) != digest {
            return Err("payload digest mismatch");
        }
        Ok(Self { payload, digest })
    }
}
fn validate(payload: &Payload) -> Result<(), &'static str> {
    if payload.version != "flowguard.workflow/v1"
        || !crate::valid_digest(&payload.context_digest)
        || !crate::valid_digest(&payload.baseline_digest)
        || !crate::valid_digest(&payload.graph_digest)
        || payload.obligations.is_empty()
        || payload.obligations.len() > 8192
    {
        return Err("invalid frozen context");
    }
    for o in &payload.obligations {
        if o.stage_id.trim().is_empty()
            || o.guard.trim().is_empty()
            || o.analyzer_version.trim().is_empty()
            || o.coverage.is_empty()
            || o.coverage.len() > 1024
            || o.coverage.iter().any(|c| c.trim().is_empty())
            || !crate::valid_digest(&o.rules_digest)
        {
            return Err("invalid obligation");
        }
    }
    if payload
        .missing_stages
        .iter()
        .any(|s| !payload.obligations.iter().any(|o| &o.stage_id == s))
    {
        return Err("unknown missing stage");
    }
    Ok(())
}
/// The caller supplies protected obligations, BEFORE collecting provider results.
/// Digests bind local fixtures; this function does not authenticate their source.
pub fn freeze(
    graph: &StageGraph,
    obligations: Vec<EvidenceObligation>,
    context_digest: &str,
    baseline_digest: &str,
) -> Result<FrozenObligations, &'static str> {
    if obligations.len() > 8192 {
        return Err("obligation budget");
    }
    let missing_stages = obligations
        .iter()
        .filter(|o| !graph.nodes().contains_key(&o.stage_id))
        .map(|o| o.stage_id.clone())
        .collect();
    let payload = Payload {
        version: "flowguard.workflow/v1".into(),
        context_digest: context_digest.into(),
        baseline_digest: baseline_digest.into(),
        graph_digest: crate::digest(&serde_json::to_vec(graph).map_err(|_| "graph encoding")?),
        obligations: obligations.into_iter().collect(),
        missing_stages,
    };
    validate(&payload)?;
    let digest = crate::digest(&serde_json::to_vec(&payload).map_err(|_| "encoding")?);
    Ok(FrozenObligations { payload, digest })
}
