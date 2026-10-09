use crate::input_limits::AllowedRoot;
use serde::{Deserialize, Serialize};
pub const STAGES: [(&str, bool); 10] = [
    ("01-requirements", false),
    ("02-architecture", true),
    ("03-solution", false),
    ("04-testcases", false),
    ("05-hld", false),
    ("06-lld", false),
    ("07-standards", true),
    ("08-review", false),
    ("09-docs", false),
    ("10-release", true),
];
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    Read,
    Missing,
    Rejected,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRecord {
    pub stage: String,
    pub owner: String,
    pub path: String,
    pub digest: Option<String>,
    pub status: SourceStatus,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceInventory {
    pub version: String,
    pub sources: Vec<SourceRecord>,
}
impl SourceInventory {
    pub fn complete(&self) -> bool {
        if self.version != "flowguard.docs/v1" || self.sources.len() != STAGES.len() {
            return false;
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut feature_owner: Option<&str> = None;
        for source in &self.sources {
            let Some((_, project)) = STAGES.iter().find(|(stage, _)| *stage == source.stage) else {
                return false;
            };
            if !seen.insert(source.stage.as_str())
                || source.status != SourceStatus::Read
                || !source.digest.as_deref().is_some_and(crate::valid_digest)
            {
                return false;
            }
            let expected_path = if *project {
                if source.owner != "project" {
                    return false;
                }
                format!("docs/project/{}.md", source.stage)
            } else {
                if !valid_feature(&source.owner)
                    || feature_owner.is_some_and(|owner| owner != source.owner)
                {
                    return false;
                }
                feature_owner = Some(&source.owner);
                format!("docs/features/{}/{}.md", source.owner, source.stage)
            };
            if source.path != expected_path {
                return false;
            }
        }
        true
    }
}
pub fn discover(
    root: &AllowedRoot,
    feature: &str,
    version: &str,
) -> Result<SourceInventory, &'static str> {
    if version != "flowguard.docs/v1" {
        return Err("unsupported source version");
    }
    if !valid_feature(feature) {
        return Err("invalid feature identity");
    }
    let sources = STAGES
        .iter()
        .map(|(stage, project)| {
            let path = if *project {
                format!("docs/project/{stage}.md")
            } else {
                format!("docs/features/{feature}/{stage}.md")
            };
            let (status, digest) = match root.read(&path) {
                Ok(bytes) => (SourceStatus::Read, Some(crate::digest(&bytes))),
                Err(crate::input_limits::ReadError::Missing) => (SourceStatus::Missing, None),
                Err(_) => (SourceStatus::Rejected, None),
            };
            SourceRecord {
                stage: stage.to_string(),
                owner: if *project {
                    "project".into()
                } else {
                    feature.into()
                },
                path,
                digest,
                status,
            }
        })
        .collect();
    Ok(SourceInventory {
        version: version.into(),
        sources,
    })
}

fn valid_feature(feature: &str) -> bool {
    !feature.is_empty()
        && !feature.split('-').any(|part| {
            part.is_empty()
                || !part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}
