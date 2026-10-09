//! Controller-frozen source scopes; byte identity never authenticates a producer.
use crate::{context::ValidatedBinding, gate::obligation_scope, obligations::FrozenObligations};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
const VERSION: &str = "flowguard.specialist-sources/v1alpha1";
const CONTEXT_VERSION: &str = "flowguard.specialist-sources/v1alpha2";
fn present<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(d).map(Some)
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaselineScope {
    Absent,
    FrozenWorkflowBaseline,
}
/// Independent controller configuration, never inferred from returned evidence.
pub struct ScopedSource {
    pub source_snapshot_digest: String,
    pub baseline: BaselineScope,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pin {
    scope: String,
    source_snapshot_digest: String,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    baseline: Option<BaselineScope>,
}
#[derive(Debug, Clone, Serialize)]
struct Payload {
    version: String,
    context_digest: String,
    frozen_digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    workflow_baseline_digest: Option<String>,
    pins: Vec<Pin>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ScopedSources {
    #[serde(flatten)]
    payload: Payload,
    digest: String,
}
impl ScopedSources {
    /// Input is independently protected controller configuration, frozen before
    /// collecting evidence. Never populate this map from an envelope being checked.
    pub fn freeze(
        binding: &ValidatedBinding,
        frozen: &FrozenObligations,
        sources: BTreeMap<String, String>,
    ) -> Result<Self, &'static str> {
        if sources.is_empty()
            || sources.len() > 64
            || sources
                .iter()
                .any(|(k, v)| k.len() > 256 || !crate::valid_digest(v))
        {
            return Err("invalid source pin budget");
        }
        let payload = Payload {
            version: VERSION.into(),
            context_digest: binding.domain_digest(),
            frozen_digest: frozen.digest().into(),
            workflow_baseline_digest: None,
            pins: sources
                .into_iter()
                .map(|(scope, source_snapshot_digest)| Pin {
                    scope,
                    source_snapshot_digest,
                    baseline: None,
                })
                .collect(),
        };
        let digest = crate::digest(&serde_json::to_vec(&payload).map_err(|_| "profile encoding")?);
        let result = Self { payload, digest };
        result.validate_for(binding, frozen)?;
        Ok(result)
    }
    /// Additive profile: every producer explicitly declares whether it uses the
    /// independently frozen workflow baseline. No arbitrary baseline override.
    pub fn freeze_contexts(
        binding: &ValidatedBinding,
        frozen: &FrozenObligations,
        sources: BTreeMap<String, ScopedSource>,
    ) -> Result<Self, &'static str> {
        if sources.is_empty()
            || sources.len() > 64
            || sources
                .iter()
                .any(|(k, v)| k.len() > 256 || !crate::valid_digest(&v.source_snapshot_digest))
        {
            return Err("invalid source pin budget");
        }
        let payload = Payload {
            version: CONTEXT_VERSION.into(),
            context_digest: binding.domain_digest(),
            frozen_digest: frozen.digest().into(),
            workflow_baseline_digest: Some(frozen.baseline_digest().into()),
            pins: sources
                .into_iter()
                .map(|(scope, source)| Pin {
                    scope,
                    source_snapshot_digest: source.source_snapshot_digest,
                    baseline: Some(source.baseline),
                })
                .collect(),
        };
        let digest = crate::digest(&serde_json::to_vec(&payload).map_err(|_| "profile encoding")?);
        let result = Self { payload, digest };
        result.validate_for(binding, frozen)?;
        Ok(result)
    }
    pub fn from_json(
        bytes: &[u8],
        binding: &ValidatedBinding,
        frozen: &FrozenObligations,
    ) -> Result<Self, &'static str> {
        if bytes.len() > 65536 {
            return Err("source profile byte budget");
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            version: String,
            context_digest: String,
            frozen_digest: String,
            #[serde(default, deserialize_with = "present")]
            workflow_baseline_digest: Option<String>,
            pins: Vec<Pin>,
            digest: String,
        }
        let w: Wire = serde_json::from_slice(bytes).map_err(|_| "invalid source profile")?;
        let result = Self {
            payload: Payload {
                version: w.version,
                context_digest: w.context_digest,
                frozen_digest: w.frozen_digest,
                workflow_baseline_digest: w.workflow_baseline_digest,
                pins: w.pins,
            },
            digest: w.digest,
        };
        result.validate_for(binding, frozen)?;
        Ok(result)
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub(crate) fn validate_for(
        &self,
        binding: &ValidatedBinding,
        frozen: &FrozenObligations,
    ) -> Result<(), &'static str> {
        let p = &self.payload;
        let version_valid = match p.version.as_str() {
            VERSION => {
                p.workflow_baseline_digest.is_none()
                    && p.pins.iter().all(|pin| pin.baseline.is_none())
            }
            CONTEXT_VERSION => {
                p.workflow_baseline_digest.as_deref() == Some(frozen.baseline_digest())
                    && p.pins.iter().all(|pin| pin.baseline.is_some())
            }
            _ => false,
        };
        if !version_valid
            || p.context_digest != binding.domain_digest()
            || p.frozen_digest != frozen.digest()
            || frozen.context_digest() != p.context_digest
            || p.pins.is_empty()
            || p.pins.len() > 64
            || p.pins
                .iter()
                .any(|p| p.scope.len() > 256 || !crate::valid_digest(&p.source_snapshot_digest))
            || !p.pins.windows(2).all(|p| p[0].scope < p[1].scope)
        {
            return Err("source profile context mismatch");
        }
        let mut required: Vec<_> = frozen.obligations().iter().map(obligation_scope).collect();
        required.sort();
        if !required
            .iter()
            .map(String::as_str)
            .eq(p.pins.iter().map(|p| p.scope.as_str()))
            || crate::digest(&serde_json::to_vec(p).map_err(|_| "profile encoding")?) != self.digest
        {
            return Err("source profile pins mismatch");
        }
        Ok(())
    }
    pub(crate) fn expected_binding(
        &self,
        controller: &guardengine::integration::RunBinding,
        scope: &str,
    ) -> Result<guardengine::integration::RunBinding, &'static str> {
        let pin = self
            .payload
            .pins
            .iter()
            .find(|p| p.scope == scope)
            .ok_or("missing specialist source pin")?;
        let mut expected = controller.clone();
        expected.source_snapshot_digest = pin.source_snapshot_digest.clone();
        match pin.baseline {
            None => {}
            Some(BaselineScope::Absent) => expected.baseline_digest = None,
            Some(BaselineScope::FrozenWorkflowBaseline) => {
                expected.baseline_digest = Some(
                    self.payload
                        .workflow_baseline_digest
                        .as_ref()
                        .ok_or("missing frozen baseline")?
                        .clone(),
                );
            }
        }
        Ok(expected)
    }
}
