//! Explicit fixture-only consumer of the SpecGuard native versioned export.
//! This type cannot satisfy a production gate; GE-TRUST is not integrated.
use specguard::{model::Identity, obligations::TestObligation};
use std::collections::BTreeSet;
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExpectedExport {
    pub baseline_digest: String,
    pub source_digest: String,
    pub candidate_oid: String,
    pub scope: BTreeSet<Identity>,
}
#[derive(Debug)]
pub struct ImportedFixtureObligations {
    obligations: Vec<TestObligation>,
}
impl ImportedFixtureObligations {
    pub fn obligations(&self) -> &[TestObligation] {
        &self.obligations
    }
}
pub fn import_fixture(
    bytes: &[u8],
    expected: &ExpectedExport,
) -> Result<ImportedFixtureObligations, &'static str> {
    expected_budget(expected)?;
    if bytes.len() > 1024 * 1024 {
        return Err("export budget");
    }
    let set: specguard::obligations::ObligationSet =
        serde_json::from_slice(bytes).map_err(|_| "invalid SpecGuard schema")?;
    if set.authentication_profile != "fixture-only" {
        return Err("production authentication unsupported");
    }
    if !set.complete
        || set.scope.is_empty()
        || set.scope != expected.scope
        || set.baseline_digest != expected.baseline_digest
        || set.source_digest != expected.source_digest
        || set.candidate_oid != expected.candidate_oid
        || !crate::valid_digest(&set.baseline_digest)
        || !crate::valid_digest(&set.source_digest)
        || !matches!(set.candidate_oid.len(), 40 | 64)
        || !set
            .candidate_oid
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("SpecGuard binding/scope mismatch");
    }
    if set.sources.is_empty()
        || set
            .sources
            .iter()
            .any(|s| s.status != specguard::model::Terminal::Complete)
        || set.obligations.is_empty()
        || set.obligations.len() > 8192
    {
        return Err("incomplete SpecGuard export");
    }
    if set.sources.len() > 4096
        || set.scope.len() > 256
        || set.sources.len().saturating_mul(set.obligations.len()) > 1_048_576
        || set.sources.iter().any(|s| !source_path(&s.path))
        || set.obligations.iter().any(|o| {
            o.links.len() > 256
                || !source_path(&o.source.path)
                || o.id.len() > 128
                || !identity_valid(&o.requirement)
                || !identity_valid(&o.acceptance)
        })
    {
        return Err("SpecGuard structural budget/source");
    }
    let mut ids = BTreeSet::new();
    let mut covered = BTreeSet::new();
    for o in &set.obligations {
        if !ids.insert(o.id.clone())
            || !set.scope.contains(&o.requirement)
            || o.source.line == 0
            || o.source.path.trim().is_empty()
            || !crate::valid_digest(&o.text_digest)
            || [
                &o.requirement.namespace,
                &o.requirement.id,
                &o.acceptance.namespace,
                &o.acceptance.id,
            ]
            .iter()
            .any(|s| s.trim().is_empty())
        {
            return Err("invalid SpecGuard obligation");
        }
        let expected_id = format!(
            "obligation:{}",
            specguard::model::digest(&(&o.requirement, &o.acceptance))
        );
        if o.id != expected_id || !set.sources.iter().any(|s| s.path == o.source.path) {
            return Err("unbound SpecGuard obligation");
        }
        covered.insert(o.requirement.clone());
    }
    if covered != set.scope {
        return Err("missing requirement obligation");
    }
    Ok(ImportedFixtureObligations {
        obligations: set.obligations,
    })
}

fn bounded(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.len() <= max && !s.contains('\0')
}
fn identity_valid(i: &Identity) -> bool {
    bounded(&i.namespace, 256) && bounded(&i.id, 256)
}
fn source_path(s: &str) -> bool {
    bounded(s, 4096)
        && !s.contains('\\')
        && s.split('/')
            .all(|p| !p.is_empty() && !matches!(p, "." | ".." | ".git"))
}
fn expected_budget(e: &ExpectedExport) -> Result<usize, &'static str> {
    if e.scope.is_empty()
        || e.scope.len() > 256
        || !crate::valid_digest(&e.baseline_digest)
        || !crate::valid_digest(&e.source_digest)
        || !matches!(e.candidate_oid.len(), 40 | 64)
        || !e
            .candidate_oid
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || e.scope.iter().any(|i| !identity_valid(i))
    {
        return Err("invalid expected export/budget");
    }
    let bytes = e
        .scope
        .iter()
        .map(|i| i.namespace.len() + i.id.len())
        .sum::<usize>()
        + e.baseline_digest.len()
        + e.source_digest.len()
        + e.candidate_oid.len();
    if bytes > 65536 {
        return Err("expected export byte budget");
    }
    Ok(bytes)
}
/// Protected controller gate declaration; it is not a successful SG observation.
#[derive(Clone, Debug, serde::Serialize)]
pub struct FixtureEvidenceRequirement {
    pub stage_id: String,
    pub rules_digest: String,
    pub analyzer_version: String,
}
/// Prepared expectations must come from a protected controller, independently of export bytes.
/// This fixture-only object authenticates neither that controller nor baseline approval.
/// ```compile_fail
/// let _: flowguard::specguard_adapter::PreparedFixtureImport = serde_json::from_str("{}").unwrap();
/// ```
pub struct PreparedFixtureImport {
    expected: ExpectedExport,
    raw_digest: String,
    current_digest: String,
    import_digest: String,
    requirement: FixtureEvidenceRequirement,
}
pub struct BoundFixtureImport {
    imported: ImportedFixtureObligations,
    current_digest: String,
    requirement: crate::obligations::EvidenceObligation,
}
impl PreparedFixtureImport {
    pub fn prepare(
        expected: &ExpectedExport,
        raw_digest: &str,
        current: &crate::context::ValidatedBinding,
        mapping: &std::collections::BTreeMap<Identity, String>,
        requirement: &FixtureEvidenceRequirement,
    ) -> Result<Self, &'static str> {
        let bytes = expected_budget(expected)?;
        current
            .check_budget()
            .map_err(|_| "current binding budget")?;
        if !crate::valid_digest(raw_digest)
            || mapping.len() > 256
            || !bounded(&requirement.stage_id, 128)
            || !bounded(&requirement.analyzer_version, 128)
            || !crate::valid_digest(&requirement.rules_digest)
            || mapping
                .iter()
                .any(|(k, v)| !identity_valid(k) || !bounded(v, 256))
        {
            return Err("prepared import metadata budget");
        }
        let expanded = bytes
            + mapping
                .iter()
                .map(|(k, v)| k.namespace.len() + k.id.len() + v.len())
                .sum::<usize>();
        if expanded > 131072 {
            return Err("prepared import expansion budget");
        }
        // Small bounded borrowed sets only, before any controller-input clones/hash.
        let mapped: BTreeSet<_> = mapping.values().map(String::as_str).collect();
        if !mapping.keys().eq(expected.scope.iter())
            || mapped.len() != mapping.len()
            || !mapped.iter().copied().eq(current
                .binding()
                .requirement_ids
                .iter()
                .map(String::as_str))
        {
            return Err("historical/current requirement mapping differs");
        }
        let current_digest = current.domain_digest();
        let mapping: Vec<_> = mapping.iter().collect(); // Identity is an object, not a JSON map key.
        let bytes = serde_json::to_vec(&(
            "flowguard.specguard-fixture-import/v1alpha1",
            expected,
            raw_digest,
            &current_digest,
            mapping,
            requirement,
        ))
        .map_err(|_| "import encoding")?;
        let import_digest = crate::digest(&bytes);
        Ok(Self {
            expected: expected.clone(),
            raw_digest: raw_digest.into(),
            current_digest,
            import_digest,
            requirement: requirement.clone(),
        })
    }
    pub fn consume(
        &self,
        bytes: &[u8],
        current: &crate::context::ValidatedBinding,
    ) -> Result<BoundFixtureImport, &'static str> {
        current
            .check_budget()
            .map_err(|_| "current binding budget")?;
        if bytes.len() > 1024 * 1024
            || self.current_digest != current.domain_digest()
            || crate::digest(bytes) != self.raw_digest
        {
            return Err("export raw bytes/current binding changed");
        }
        let imported = import_fixture(bytes, &self.expected)?;
        if imported.obligations.len() > 1024 {
            return Err("gate coverage budget");
        }
        let coverage = imported
            .obligations
            .iter()
            .map(|o| format!("flowguard.specguard-import:{}:{}", self.import_digest, o.id))
            .collect();
        Ok(BoundFixtureImport {
            imported,
            current_digest: self.current_digest.clone(),
            requirement: crate::obligations::EvidenceObligation {
                stage_id: self.requirement.stage_id.clone(),
                guard: "specguard".into(),
                coverage,
                rules_digest: self.requirement.rules_digest.clone(),
                analyzer_version: self.requirement.analyzer_version.clone(),
            },
        })
    }
}
impl BoundFixtureImport {
    pub fn obligations(&self) -> &[TestObligation] {
        self.imported.obligations()
    }
    /// Adds required evidence only. Never returns provider success or approval authority.
    pub fn evidence_obligation(
        &self,
        current: &crate::context::ValidatedBinding,
    ) -> Result<crate::obligations::EvidenceObligation, &'static str> {
        current
            .check_budget()
            .map_err(|_| "current binding budget")?;
        if self.current_digest != current.domain_digest() {
            return Err("current workflow binding changed");
        }
        Ok(self.requirement.clone())
    }
}
