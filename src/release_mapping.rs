//! Explicit local release mapping contract. Structural validation is not approval
//! or migration of a real legacy release; consume the existing ReleasePlan with
//! freshly qualified stages before any new release eligibility decision.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseMappingRow {
    pub feature_id: String,
    pub requirement_id: String,
    pub immutable_09: String,
}
fn rows(input: &[ReleaseMappingRow]) -> Result<BTreeMap<(&str, &str), &str>, &'static str> {
    if input.is_empty() || input.len() > 256 {
        return Err("release mapping budget or empty");
    }
    let mut out = BTreeMap::new();
    let mut feature_refs = BTreeMap::new();
    for row in input {
        if !crate::stage::valid_feature(&row.feature_id)
            || row.feature_id.len() > 256
            || row.requirement_id.trim().is_empty()
            || row.requirement_id.len() > 256
            || !crate::valid_digest(&row.immutable_09)
        {
            return Err("invalid release mapping identity");
        }
        if out
            .insert(
                (row.feature_id.as_str(), row.requirement_id.as_str()),
                row.immutable_09.as_str(),
            )
            .is_some()
        {
            return Err("ambiguous release mapping");
        }
        if feature_refs
            .insert(row.feature_id.as_str(), row.immutable_09.as_str())
            .is_some_and(|old| old != row.immutable_09)
        {
            return Err("ambiguous immutable09 reference");
        }
    }
    Ok(out)
}
/// The expected closure is supplied independently by the local caller. Neither
/// input is trusted approval data. Exact equality is necessary, never sufficient.
pub fn validate(
    mapping: &[ReleaseMappingRow],
    expected: &[ReleaseMappingRow],
) -> Result<(), &'static str> {
    if rows(mapping)? != rows(expected)? {
        return Err("missing or mismatched release mapping");
    }
    Ok(())
}
