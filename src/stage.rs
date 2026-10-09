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
        self.sources.len() == 10 && self.sources.iter().all(|s| s.status == SourceStatus::Read)
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
    if feature.is_empty()
        || feature.split('-').any(|part| {
            part.is_empty()
                || !part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
    {
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
