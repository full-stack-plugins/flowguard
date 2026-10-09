//! Read-only queue observations from a protected controller port.
//! The only implemented authority profile is an explicitly local fixture.
use crate::context::{InvocationInput, ValidatedBinding, bind};
use gitguard::{Repository, candidate::CandidateSnapshot};
use serde::Serialize;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerProfile {
    Fixture,
    Unsupported,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueError {
    Invalid,
    Budget,
    Authority,
    ClockRollback,
    Replay,
    Expired,
    Candidate,
}
pub trait QueueClock {
    fn now(&self) -> Result<i64, QueueError>;
}
/// Implementations own their current event registry independently of candidate input.
/// Returning matching text or a digest does not authenticate a production issuer.
pub trait ControllerEventPort {
    fn profile(&self) -> ControllerProfile;
    fn current(&self, reference: &str) -> Result<ControllerRecord, QueueError>;
}
#[derive(Clone, Serialize)]
pub struct EventMember {
    pub oid: String,
    pub requirement_ids: Vec<String>,
}
#[derive(Clone, Serialize)]
pub struct ControllerRecord {
    pub version: String,
    pub principal: String,
    pub queue_id: String,
    pub repo_id: String,
    pub task_id: String,
    pub worktree_id: String,
    pub reference: String,
    pub sequence: u64,
    pub issued_at: i64,
    pub expires_at: i64,
    pub revoked: bool,
    pub candidate_oid: String,
    pub base_oid: String,
    pub merge_group_id: String,
    pub members: Vec<EventMember>,
}
pub struct QueueConfig {
    pub principal: String,
    pub queue_id: String,
    pub repo_id: String,
    pub task_id: String,
    pub worktree_id: String,
    pub policy_digest: String,
    pub allowed_paths: Vec<Vec<u8>>,
}
pub struct ProtectedQueue {
    config: QueueConfig,
}
pub struct QueueSession {
    config: QueueConfig,
    last_time: Option<i64>,
    high_water: Option<(u64, String, bool)>,
}
/// Advisory observation only. Revalidate against the current port before consumption.
/// ```compile_fail
/// let _: flowguard::merge_queue::QueueObservation = serde_json::from_str("{}").unwrap();
/// ```
pub struct QueueObservation {
    binding: ValidatedBinding,
    reference: String,
    sequence: u64,
    event_digest: String,
}
impl QueueObservation {
    pub fn binding(&self) -> &ValidatedBinding {
        &self.binding
    }
}
fn text(value: &str, bytes: &mut usize) -> Result<(), QueueError> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(QueueError::Invalid);
    }
    *bytes = bytes.checked_add(value.len()).ok_or(QueueError::Budget)?;
    if value.len() > 256 || *bytes > 64 * 1024 {
        return Err(QueueError::Budget);
    }
    Ok(())
}
impl ProtectedQueue {
    pub fn freeze(config: QueueConfig) -> Result<Self, QueueError> {
        let mut total = 0;
        for value in [
            &config.principal,
            &config.queue_id,
            &config.repo_id,
            &config.task_id,
            &config.worktree_id,
            &config.policy_digest,
        ] {
            text(value, &mut total)?;
        }
        if config.allowed_paths.is_empty() || config.allowed_paths.len() > 256 {
            return Err(QueueError::Budget);
        }
        for path in &config.allowed_paths {
            total = total.checked_add(path.len()).ok_or(QueueError::Budget)?;
            if path.len() > 4096 || total > 64 * 1024 {
                return Err(QueueError::Budget);
            }
        }
        if config.allowed_paths.windows(2).any(|w| w[0] >= w[1]) {
            return Err(QueueError::Invalid);
        }
        // Reuse the native policy/path validator; do not duplicate GitGuard semantics.
        gitguard::scope::TaskScope::advisory(
            &config.task_id,
            vec!["queue-policy-validation".into()],
            config.allowed_paths.clone(),
            &config.policy_digest,
            None,
        )
        .map_err(|_| QueueError::Invalid)?;
        Ok(Self { config })
    }
    pub fn session(self) -> QueueSession {
        QueueSession {
            config: self.config,
            last_time: None,
            high_water: None,
        }
    }
}
impl QueueSession {
    fn time(&mut self, clock: &dyn QueueClock) -> Result<i64, QueueError> {
        let now = clock.now()?;
        if now < 0 || self.last_time.is_some_and(|last| now < last) {
            return Err(QueueError::ClockRollback);
        }
        self.last_time = Some(now);
        Ok(now)
    }
    pub fn observe(
        &mut self,
        reference: &str,
        repo: &Repository,
        candidate: &CandidateSnapshot,
        port: &dyn ControllerEventPort,
        clock: &dyn QueueClock,
    ) -> Result<QueueObservation, QueueError> {
        let now = self.time(clock)?;
        text(reference, &mut 0)?;
        if port.profile() != ControllerProfile::Fixture {
            return Err(QueueError::Authority);
        }
        let record = port.current(reference)?;
        let mut total = 0;
        for value in [
            &record.version,
            &record.principal,
            &record.queue_id,
            &record.repo_id,
            &record.task_id,
            &record.worktree_id,
            &record.reference,
            &record.candidate_oid,
            &record.base_oid,
            &record.merge_group_id,
        ] {
            text(value, &mut total)?;
        }
        if record.members.is_empty() || record.members.len() > 64 {
            return Err(QueueError::Budget);
        }
        let mut requirements = std::collections::BTreeSet::new();
        let mut members = std::collections::BTreeSet::new();
        let mut count = 0usize;
        for member in &record.members {
            text(&member.oid, &mut total)?;
            count = count
                .checked_add(member.requirement_ids.len())
                .ok_or(QueueError::Budget)?;
            if count > 4096 {
                return Err(QueueError::Budget);
            }
            if member.requirement_ids.is_empty()
                || !members.insert(member.oid.as_str())
                || member.requirement_ids.windows(2).any(|w| w[0] >= w[1])
            {
                return Err(QueueError::Invalid);
            }
            for id in &member.requirement_ids {
                text(id, &mut total)?;
                requirements.insert(id.as_str());
            }
        }
        let config = &self.config;
        if record.version != "flowguard.queue-event/v1"
            || record.principal != config.principal
            || record.queue_id != config.queue_id
            || record.repo_id != config.repo_id
            || record.task_id != config.task_id
            || record.worktree_id != config.worktree_id
            || record.reference != reference
            || record.sequence == 0
        {
            return Err(QueueError::Authority);
        }
        let event_digest =
            crate::digest(&serde_json::to_vec(&record).map_err(|_| QueueError::Invalid)?);
        if let Some((sequence, digest, conflicted)) = &mut self.high_water {
            if record.sequence < *sequence {
                return Err(QueueError::Replay);
            }
            if record.sequence == *sequence {
                *conflicted |= event_digest != *digest;
                if *conflicted {
                    return Err(QueueError::Replay);
                }
            }
        }
        // A correctly scoped new event supersedes old events even if expired or its
        // candidate cannot be read. Failure must not revive an older queue attempt.
        self.high_water = Some((record.sequence, event_digest.clone(), false));
        if record.revoked
            || record.issued_at < 0
            || record.issued_at > now
            || record.expires_at <= now
        {
            return Err(QueueError::Expired);
        }
        crate::admission::candidate(candidate).map_err(|_| QueueError::Budget)?;
        if candidate.repo_id() != config.repo_id
            || candidate.task_id() != config.task_id
            || candidate.worktree_id() != config.worktree_id
            || candidate.policy_digest() != config.policy_digest
            || candidate.allowed_paths() != config.allowed_paths
            || candidate.candidate_oid() != record.candidate_oid
            || candidate.base_oid() != record.base_oid
            || candidate.merge_group_id() != Some(record.merge_group_id.as_str())
            || !candidate
                .members()
                .iter()
                .map(String::as_str)
                .eq(record.members.iter().map(|m| m.oid.as_str()))
            || !candidate
                .requirement_ids()
                .iter()
                .map(String::as_str)
                .eq(requirements.iter().copied())
        {
            return Err(QueueError::Candidate);
        }
        let binding = bind(
            &InvocationInput {
                repo_candidates: vec![config.repo_id.clone()],
                task_candidates: vec![config.task_id.clone()],
                worktree_id: config.worktree_id.clone(),
                requirement_ids: requirements.into_iter().map(str::to_owned).collect(),
                candidate_oid: record.candidate_oid.clone(),
                base_oid: record.base_oid.clone(),
            },
            repo,
            candidate,
        )
        .map_err(|_| QueueError::Candidate)?;
        if self.time(clock)? >= record.expires_at {
            return Err(QueueError::Expired);
        }
        Ok(QueueObservation {
            binding,
            reference: reference.into(),
            sequence: record.sequence,
            event_digest,
        })
    }
    pub fn revalidate(
        &mut self,
        old: &QueueObservation,
        repo: &Repository,
        candidate: &CandidateSnapshot,
        port: &dyn ControllerEventPort,
        clock: &dyn QueueClock,
    ) -> Result<ValidatedBinding, QueueError> {
        let current = self.observe(&old.reference, repo, candidate, port, clock)?;
        if current.sequence != old.sequence
            || current.event_digest != old.event_digest
            || current.binding.domain_digest() != old.binding.domain_digest()
        {
            return Err(QueueError::Replay);
        }
        Ok(current.binding)
    }
}
