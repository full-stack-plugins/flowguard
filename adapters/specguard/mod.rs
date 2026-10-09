//! Explicit fixture-only consumer of the SpecGuard native versioned export.
//! This type cannot satisfy a production gate; GE-TRUST is not integrated.
use specguard::{model::Identity, obligations::TestObligation};
use std::collections::BTreeSet;
#[derive(Debug)]
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
