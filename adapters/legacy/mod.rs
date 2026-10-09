//! Fixed-revision declarations, not effective legacy state or authorization.
use crate::{input_limits::AllowedRoot, stage::STAGES, stage_transition::StageState};
use std::collections::BTreeMap;
pub const LEGACY_REVISION: &str = "13b52b054c31f614dc272b18c195b8fa929aa595";
#[derive(Debug, PartialEq, Eq)]
pub struct Observation {
    pub declared: StageState,
    pub source_digest: String,
}
impl Observation {
    /// No rule-owner transfer or legacy-to-new authority migration is implemented.
    pub fn migration_supported(&self) -> bool {
        false
    }
}
pub fn observe(
    root: &AllowedRoot,
    feature: &str,
    stage: &str,
    revision: &str,
) -> Result<Observation, &'static str> {
    if revision != LEGACY_REVISION {
        return Err("unsupported legacy revision");
    }
    if !crate::stage::valid_feature(feature) {
        return Err("invalid feature");
    }
    let (_, project) = STAGES
        .iter()
        .find(|(id, _)| *id == stage)
        .ok_or("unknown stage")?;
    let path = if *project {
        format!("docs/project/{stage}.md")
    } else {
        format!("docs/features/{feature}/{stage}.md")
    };
    let bytes = root.read(&path).map_err(|_| "legacy source unavailable")?;
    let text = std::str::from_utf8(&bytes).map_err(|_| "invalid legacy encoding")?;
    let mut rows = BTreeMap::new();
    let mut fence: Option<(u8, usize)> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let marker = trimmed.as_bytes().first().copied();
        let width = trimmed.bytes().take_while(|b| Some(*b) == marker).count();
        if let Some((character, minimum)) = fence {
            if marker == Some(character) && width >= minimum && trimmed[width..].trim().is_empty() {
                fence = None;
            }
            continue;
        }
        if matches!(marker, Some(b'`' | b'~')) && width >= 3 {
            fence = Some((marker.unwrap(), width));
            continue;
        }
        let Some(inner) = line
            .strip_prefix('|')
            .and_then(|s| s.trim_end().strip_suffix('|'))
        else {
            continue;
        };
        let mut columns = inner.split('|').map(str::trim);
        let key = columns.next();
        if !matches!(
            key,
            Some(
                "任务"
                    | "父任务"
                    | "阶段"
                    | "阶段状态"
                    | "规格事实源"
                    | "原生产物"
                    | "批准依据"
                    | "前置指纹"
                    | "验收指纹"
            )
        ) {
            continue;
        }
        let value = columns.next().ok_or("ambiguous legacy row")?;
        if columns.next().is_some() {
            return Err("ambiguous legacy row");
        }
        if rows.insert(key.unwrap(), value).is_some() {
            return Err("duplicate legacy field");
        }
    }
    if rows.get("阶段").copied() != Some(stage) {
        return Err("legacy stage mismatch");
    }
    // Text is a declaration only: never validate legacy approval/fingerprint rules here.
    let declared = match rows.get("阶段状态").copied() {
        Some("pending") => StageState::Pending,
        Some("in_progress") => StageState::InProgress,
        Some("pending_acceptance") => StageState::PendingAcceptance,
        Some("accepted") => StageState::Accepted,
        Some("inherited") => StageState::Inherited,
        Some("skipped") => StageState::Skipped,
        Some("invalidated") => StageState::Invalidated,
        _ => return Err("unknown legacy state"),
    };
    Ok(Observation {
        declared,
        source_digest: crate::digest(&bytes),
    })
}
