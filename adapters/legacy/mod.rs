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

/// Read-only compatibility display for strictly parsed documents. Never authority.
#[derive(Debug, serde::Serialize)]
pub struct StatusView {
    pub declared: StageState,
    pub legacy_effective: StageState,
    pub source_digest: String,
    pub new_eligibility: &'static str,
}
pub fn status(root: &AllowedRoot, feature: &str, stage: &str, revision: &str) -> Result<StatusView, &'static str> {
    let (view, _) = effective(root, feature, stage, revision, &mut 0)?;
    Ok(view)
}
fn satisfied(s: StageState) -> bool {
    matches!(s, StageState::Accepted | StageState::Inherited | StageState::Skipped)
}
fn effective(root: &AllowedRoot, feature: &str, stage: &str, revision: &str, budget: &mut usize) -> Result<(StatusView, String), &'static str> {
    *budget += 1;
    if *budget > 1024 { return Err("legacy closure budget"); }
    let observation = observe(root, feature, stage, revision)?;
    let index = STAGES.iter().position(|(s,_)| *s == stage).ok_or("unknown stage")?;
    let path = if STAGES[index].1 { format!("docs/project/{stage}.md") } else { format!("docs/features/{feature}/{stage}.md") };
    let bytes = root.read(&path).map_err(|_| "legacy source unavailable")?;
    if crate::digest(&bytes) != observation.source_digest { return Err("legacy source changed during observation"); }
    // Python read_text uses universal newlines; full-byte source identity stays separate.
    let text = std::str::from_utf8(&bytes).map_err(|_| "invalid legacy encoding")?.replace("\r\n", "\n").replace('\r', "\n");
    let rows: BTreeMap<_,_> = text.lines().filter_map(|line| {
        let columns: Vec<_> = line.strip_prefix('|')?.trim_end().strip_suffix('|')?.split('|').map(str::trim).collect();
        (columns.len() == 2).then(|| (columns[0], columns[1]))
    }).collect();
    let fingerprint = rows.get("验收指纹").copied().unwrap_or("-").to_string();
    let mut state = observation.declared;
    if satisfied(state) {
        if content_hash(&text) != fingerprint { state = StageState::Invalidated; }
        else {
            let upstream = if stage == "10-release" {
                // Scope mapping is deliberately separate from new release authority.
                let features = release_features(&text)?;
                let mut values = Vec::new();
                for (owner, prior, label) in std::iter::once((feature.to_string(), "07-standards", "07-standards".to_string())).chain(features.into_iter().map(|f| (f.clone(), "09-docs", format!("{f}/09-docs")))) {
                    let (v, fp) = effective(root, &owner, prior, revision, budget)?;
                    if !satisfied(v.legacy_effective) { return Ok((StatusView { declared: observation.declared, legacy_effective: StageState::Invalidated, source_digest: observation.source_digest, new_eligibility: "not_evaluated" }, fingerprint)); }
                    values.push(format!("{label}:{fp}"));
                }
                raw_hash(values.join("\n").as_bytes())
            } else if STAGES[index].1 { "project".into() }
            else {
                let mut values = Vec::new();
                for (prior, _) in &STAGES[..index] {
                    let (v, fp) = effective(root, feature, prior, revision, budget)?;
                    if !satisfied(v.legacy_effective) { state = StageState::Invalidated; }
                    values.push(format!("{prior}:{fp}"));
                }
                raw_hash(values.join("\n").as_bytes())
            };
            if rows.get("前置指纹").copied() != Some(upstream.as_str()) { state = StageState::Invalidated; }
        }
    }
    Ok((StatusView { declared: observation.declared, legacy_effective: state, source_digest: observation.source_digest, new_eligibility: "not_evaluated" }, fingerprint))
}
fn raw_hash(bytes: &[u8]) -> String { crate::digest(bytes)[7..].into() }
fn content_hash(text: &str) -> String {
    let mut text = text.to_string();
    let start = "<!-- FLOWGUARD_EVIDENCE_START -->";
    let end = "<!-- FLOWGUARD_EVIDENCE_END -->";
    while let Some(a) = text.find(start) {
        let Some(b) = text[a+start.len()..].find(end) else { break };
        let mut finish = a + start.len() + b + end.len();
        if text[finish..].starts_with('\n') { finish += 1; }
        let begin = if text[..a].ends_with('\n') { a-1 } else { a };
        text.replace_range(begin..finish, "");
    }
    let stripped: String = text.split_inclusive('\n').map(|line| {
        if ["| 阶段状态 |", "| 批准依据 |", "| 前置指纹 |", "| 验收指纹 |"].iter().any(|p| line.starts_with(p)) {
            if line.ends_with('\n') { "\n" } else { "" }
        } else { line }
    }).collect();
    raw_hash(stripped.as_bytes())
}
fn release_features(text: &str) -> Result<Vec<String>, &'static str> {
    let mut inside = false;
    let mut features = Vec::new();
    for line in text.lines() {
        if line.starts_with("## 3. 发布内容") { inside = true; continue; }
        if !inside { continue; }
        if line == "---" || line.starts_with("## 4.") { break; }
        if !line.starts_with('|') { continue; }
        let columns: Vec<_> = line.trim().trim_matches('|').split('|').map(str::trim).collect();
        if columns.len() != 3 { return Err("ambiguous legacy release scope"); }
        let f = columns[0];
        if matches!(f, "功能 (feature)" | ":---" | "---") { continue; }
        if !crate::stage::valid_feature(f) || features.iter().any(|s| s == f) { return Err("invalid legacy release scope"); }
        features.push(f.to_string());
        if features.len() > 64 { return Err("legacy release budget"); }
    }
    if features.is_empty() { return Err("missing legacy release scope"); }
    Ok(features)
}
