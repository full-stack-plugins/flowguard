//! Immutable audit observations, never live qualification capabilities.
use crate::{
    context::ValidatedBinding,
    dependencies::{GraphLimits, StageGraph, StageRecord, build_graph},
    gate::{GateDecision, GateRun},
    obligations::FrozenObligations,
};
use guardengine::integration::{
    CoverageStatus, GuardRunEnvelope, RunBinding, RunStatus, load_envelope_json,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
const VERSION: &str = "flowguard.accepted-stage/v1alpha1";
const PROFILE: &str = "local-controller-observed";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    observed_state: crate::stage_transition::StageState,
    stage: StageRecord,
    graph: Vec<StageRecord>,
    graph_digest: String,
    plan_digest: String,
    context_digest: String,
    candidate_scope_digest: String,
    work_digest: String,
    binding: RunBinding,
    frozen_obligations: serde_json::Value,
    required_scopes: Vec<String>,
    dependencies: BTreeMap<String, String>,
    gate: GateDecision,
    envelope: GuardRunEnvelope,
    envelope_digest: String,
    approval_reference: String,
    approval_action: String,
    approval_purpose: String,
    checked_at: i64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    api_version: String,
    kind: String,
    profile: String,
    payload: Payload,
    digest: String,
}
/// Structurally checked historical audit bytes, not approval or a live receipt.
/// ```compile_fail
/// let _: flowguard::accepted_stage::AcceptedStageSnapshot = serde_json::from_str("{}").unwrap();
/// ```
pub struct AcceptedStageSnapshot(Wire);
fn encoded(v: &impl Serialize) -> Result<Vec<u8>, &'static str> {
    serde_json::to_vec(v).map_err(|_| "encoding")
}
fn hash(v: &impl Serialize) -> Result<String, &'static str> {
    Ok(crate::digest(&encoded(v)?))
}
pub(crate) fn admit(v: &impl Serialize, left: &mut usize) -> Result<(), &'static str> {
    struct Sink<'a>(&'a mut usize);
    impl std::io::Write for Sink<'_> {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            *self.0 = self
                .0
                .checked_sub(b.len())
                .ok_or_else(|| std::io::Error::other("snapshot budget"))?;
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Sink(left), v).map_err(|_| "snapshot budget")
}
pub(crate) struct Export<'a> {
    pub graph: &'a StageGraph,
    pub binding: &'a ValidatedBinding,
    pub frozen: &'a FrozenObligations,
    pub stage: &'a str,
    pub graph_digest: &'a str,
    pub plan_digest: &'a str,
    pub context: &'a str,
    pub work: &'a str,
    pub required: &'a [String],
    pub dependencies: &'a BTreeMap<String, String>,
    pub run: &'a GateRun,
    pub approval: &'a str,
    pub action: &'a str,
    pub now: i64,
}
impl Export<'_> {
    pub(crate) fn check(&self) -> Result<(), &'static str> {
        let mut left = 512 * 1024;
        if self.graph.nodes().len() > 64 || self.dependencies.len() > 64 {
            return Err("snapshot budget");
        }
        self.binding.check_budget().map_err(|_| "binding budget")?;
        for bytes in [
            self.stage,
            self.graph_digest,
            self.plan_digest,
            self.context,
            self.work,
            self.approval,
            self.action,
        ] {
            if bytes.len() > 256 || bytes.trim().is_empty() {
                return Err("identity budget");
            }
        }
        admit(self.graph, &mut left)?;
        admit(self.frozen, &mut left)?;
        admit(self.run.envelope(), &mut left)?;
        admit(&self.run.domain(), &mut left)?;
        admit(&self.required, &mut left)?;
        admit(self.dependencies, &mut left)?;
        if hash(self.graph)? != self.graph_digest
            || self.binding.domain_digest() != self.context
            || self.frozen.context_digest() != self.context
            || self.frozen.graph_digest() != self.graph_digest
            || !self.graph.nodes().contains_key(self.stage)
            || self.run.work_digest() != self.work
        {
            return Err("export binding mismatch");
        }
        let gate = self.run.domain().ok_or("missing gate")?;
        if gate.frozen_obligations_digest != self.frozen.digest()
            || gate.binding_digest != self.context
            || self.run.envelope().binding != *self.binding.binding()
        {
            return Err("export evidence mismatch");
        }
        Ok(())
    }
    pub(crate) fn finish(self) -> Result<AcceptedStageSnapshot, &'static str> {
        let payload = Payload {
            observed_state: crate::stage_transition::StageState::Accepted,
            stage: self.graph.nodes()[self.stage].clone(),
            graph: self.graph.nodes().values().cloned().collect(),
            graph_digest: self.graph_digest.into(),
            plan_digest: self.plan_digest.into(),
            context_digest: self.context.into(),
            candidate_scope_digest: self.binding.candidate_scope_digest(),
            work_digest: self.work.into(),
            binding: self.binding.binding().clone(),
            frozen_obligations: serde_json::to_value(self.frozen).map_err(|_| "frozen encoding")?,
            required_scopes: self.required.to_vec(),
            dependencies: self.dependencies.clone(),
            gate: self.run.domain().ok_or("missing gate")?.clone(),
            envelope: self.run.envelope().clone(),
            envelope_digest: hash(self.run.envelope())?,
            approval_reference: self.approval.into(),
            approval_action: self.action.into(),
            approval_purpose: "stage.accept".into(),
            checked_at: self.now,
        };
        validate(&payload)?;
        let digest = hash(&(VERSION, "StageRecord", PROFILE, &payload))?;
        Ok(AcceptedStageSnapshot(Wire {
            api_version: VERSION.into(),
            kind: "StageRecord".into(),
            profile: PROFILE.into(),
            payload,
            digest,
        }))
    }
}
fn validate(p: &Payload) -> Result<(), &'static str> {
    if p.observed_state != crate::stage_transition::StageState::Accepted {
        return Err("not an accepted observation");
    }
    let mut left = 768 * 1024;
    admit(p, &mut left)?;
    if p.graph.len() > 64 || p.required_scopes.len() > 256 || p.dependencies.len() > 64 {
        return Err("snapshot counts");
    }
    for d in [
        &p.graph_digest,
        &p.plan_digest,
        &p.context_digest,
        &p.candidate_scope_digest,
        &p.work_digest,
        &p.envelope_digest,
    ] {
        if !crate::valid_digest(d) {
            return Err("snapshot digest");
        }
    }
    if p.approval_reference.trim().is_empty()
        || p.approval_reference.len() > 256
        || p.approval_purpose != "stage.accept"
    {
        return Err("approval reference");
    }
    crate::admission::binding(&p.binding).map_err(|_| "binding budget")?;
    let graph = build_graph(
        p.graph.clone(),
        GraphLimits {
            max_nodes: 64,
            max_edges: 4096,
            max_depth: 64,
        },
    )
    .map_err(|_| "graph invalid")?;
    if graph.nodes().values().ne(p.graph.iter())
        || graph.nodes().get(&p.stage.id) != Some(&p.stage)
        || hash(&graph)? != p.graph_digest
    {
        return Err("stage graph mismatch");
    }
    if p.stage.dependencies.iter().ne(p.dependencies.keys())
        || p.dependencies.values().any(|v| !crate::valid_digest(v))
    {
        return Err("dependency mismatch");
    }
    let action = format!(
        "stage.accept:{}",
        hash(&(
            "flowguard.stage-intent/v1alpha1",
            &p.graph_digest,
            &p.stage.id,
            &crate::stage_qualification::StageMode::Accept,
            &p.dependencies
        ))?
    );
    if p.approval_action != action || p.gate.action != action {
        return Err("action mismatch");
    }
    if p.context_digest
        != hash(&(
            "flowguard.binding/v1",
            &p.binding,
            &p.candidate_scope_digest,
        ))?
    {
        return Err("context mismatch");
    }
    let frozen = FrozenObligations::from_json(&encoded(&p.frozen_obligations)?)?;
    if frozen.context_digest() != p.context_digest
        || frozen.graph_digest() != p.graph_digest
        || !frozen.missing_stages().is_empty()
        || p.gate.frozen_obligations_digest != frozen.digest()
    {
        return Err("frozen mismatch");
    }
    let anchors = [
        format!("flowguard.frozen:{}", frozen.digest()),
        format!(
            "flowguard.action:{}",
            crate::digest(p.approval_action.as_bytes())
        ),
        format!("git.scope:{}", p.candidate_scope_digest),
    ];
    if anchors.iter().any(|s| !p.required_scopes.contains(s))
        || frozen.obligations().iter().any(|o| {
            !graph.nodes().contains_key(&o.stage_id)
                || !p
                    .required_scopes
                    .contains(&crate::gate::obligation_scope(o))
                || !p
                    .required_scopes
                    .contains(&format!("flowguard.stage:{}", o.stage_id))
        })
    {
        return Err("required obligation mismatch");
    }
    let envelope = load_envelope_json(
        &encoded(&p.envelope)?,
        guardengine::integration::EvidenceProfile::EngineBacked,
    )
    .map_err(|_| "envelope invalid")?;
    let gate = crate::gate::load_gate_decision(&encoded(&p.gate)?)?;
    let domain_digest = hash(&gate)?;
    if envelope.artifacts.domain.len() != 1
        || envelope.artifacts.domain[0].digest != domain_digest
        || envelope.artifacts.domain[0].uri
            != format!("artifact://flowguard/{}", &domain_digest[7..])
        || envelope.artifacts.domain[0].media_type != "application/json"
    {
        return Err("domain artifact mismatch");
    }
    if p.envelope_digest != hash(&envelope)?
        || envelope.binding != p.binding
        || envelope.run_status != RunStatus::Completed
        || envelope.coverage.status != CoverageStatus::Complete
        || gate.binding_digest != p.context_digest
        || envelope.artifacts.report.as_ref() != Some(&gate.report)
        || envelope.decision.as_ref() != Some(&gate.decision)
        || envelope.coverage.required_scopes != p.required_scopes
    {
        return Err("gate mismatch");
    }
    if p.required_scopes.is_empty()
        || p.required_scopes.windows(2).any(|w| w[0] >= w[1])
        || p.required_scopes.iter().any(|s| s.trim().is_empty())
    {
        return Err("required scopes");
    }
    Ok(())
}
impl AcceptedStageSnapshot {
    pub fn to_json(&self) -> Result<Vec<u8>, &'static str> {
        encoded(&self.0)
    }
    pub fn digest(&self) -> &str {
        &self.0.digest
    }
    pub fn from_json(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > 1024 * 1024 {
            return Err("snapshot byte budget");
        }
        unique_json(bytes)?;
        let wire: Wire = serde_json::from_slice(bytes).map_err(|_| "snapshot schema")?;
        if wire.api_version != VERSION || wire.kind != "StageRecord" || wire.profile != PROFILE {
            return Err("snapshot version/profile");
        }
        validate(&wire.payload)?;
        if wire.digest != hash(&(VERSION, "StageRecord", PROFILE, &wire.payload))? {
            return Err("snapshot tamper");
        }
        Ok(Self(wire))
    }
}
// Full recursive duplicate-key admission before Value/set normalization.
fn unique_json(bytes: &[u8]) -> Result<(), &'static str> {
    struct Unique;
    impl<'de> Deserialize<'de> for Unique {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct Visitor;
            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = Unique;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("unique JSON")
                }
                fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<Unique, E> {
                    Ok(Unique)
                }
                fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<Unique, E> {
                    Ok(Unique)
                }
                fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<Unique, E> {
                    Ok(Unique)
                }
                fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<Unique, E> {
                    Ok(Unique)
                }
                fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<Unique, E> {
                    Ok(Unique)
                }
                fn visit_unit<E: serde::de::Error>(self) -> Result<Unique, E> {
                    Ok(Unique)
                }
                fn visit_seq<A: serde::de::SeqAccess<'de>>(
                    self,
                    mut a: A,
                ) -> Result<Unique, A::Error> {
                    while a.next_element::<Unique>()?.is_some() {}
                    Ok(Unique)
                }
                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut a: A,
                ) -> Result<Unique, A::Error> {
                    let mut keys = std::collections::BTreeSet::new();
                    while let Some(k) = a.next_key::<String>()? {
                        if !keys.insert(k) {
                            return Err(serde::de::Error::custom("duplicate key"));
                        }
                        a.next_value::<Unique>()?;
                    }
                    Ok(Unique)
                }
            }
            d.deserialize_any(Visitor)
        }
    }
    serde_json::from_slice::<Unique>(bytes)
        .map(|_| ())
        .map_err(|_| "ambiguous JSON")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rehashed_snapshot_cannot_replace_gate_domain_under_original_envelope() {
        let mut snapshot = AcceptedStageSnapshot::from_json(include_bytes!(
            "../fixtures/schema/accepted-stage-valid.json"
        ))
        .unwrap();
        let report = snapshot.0.payload.gate.report.clone();
        snapshot.0.payload.gate.upstream_reports.push(report);
        snapshot.0.digest = hash(&(VERSION, "StageRecord", PROFILE, &snapshot.0.payload)).unwrap();
        assert!(AcceptedStageSnapshot::from_json(&snapshot.to_json().unwrap()).is_err());
    }
}
