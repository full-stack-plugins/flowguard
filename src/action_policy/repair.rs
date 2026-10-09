//! Local authenticated test-repair scope; never a file writer or delivery grant.
use super::{Action, ActionPolicy};
use crate::{
    context::ValidatedBinding, obligations::FrozenObligations,
    stage_qualification::ProtectedStagePlan,
};
use guardengine::integration::eligibility::{
    AuthorityProvider, EligibilityPolicy, validate_approval_record,
};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
};
const PURPOSE: &str = "workflow.write-tests";
pub struct RepairContext<'a> {
    pub stage_plan: &'a ProtectedStagePlan,
    pub binding: &'a ValidatedBinding,
    pub frozen: &'a FrozenObligations,
    pub policy: &'a ActionPolicy,
}
pub struct RepairRequest<'a> {
    pub action: Action,
    pub paths: &'a BTreeSet<String>,
    pub approval_reference: &'a str,
    pub now: i64,
}
#[derive(Debug, PartialEq, Eq)]
pub enum RepairError {
    Budget,
    Binding,
    Scope,
    Action,
    Approval,
    Clock,
}
pub struct ProtectedRepairPlan {
    digest: String,
    stage_plan: String,
    context: String,
    frozen: String,
    applicability: ActionPolicy,
    paths: BTreeSet<String>,
    policy: EligibilityPolicy,
    frozen_at: i64,
}
/// A local observation with no serialization or write/execution capability.
/// ```compile_fail
/// let _: flowguard::action_policy::RepairReceipt<'_> = serde_json::from_str("{}").unwrap();
/// ```
pub struct RepairReceipt<'a> {
    plan: &'a ProtectedRepairPlan,
    paths: &'a BTreeSet<String>,
    approval: &'a str,
    last_checked: Cell<i64>,
}
fn path_valid(p: &str) -> bool {
    p.len() <= 1024
        && p.starts_with("tests/")
        && p.split('/').all(|s| {
            !s.is_empty()
                && s != "."
                && s != ".."
                && s != ".git"
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        })
        && [".rs", ".py", ".ts", ".js", ".java"]
            .iter()
            .any(|ext| p.ends_with(ext))
}
fn paths_admission(paths: &BTreeSet<String>) -> Result<(), RepairError> {
    if paths.is_empty() || paths.len() > 64 || paths.iter().any(|p| p.len() > 1024) {
        return Err(RepairError::Budget);
    }
    if paths.iter().any(|p| !path_valid(p)) {
        return Err(RepairError::Scope);
    }
    Ok(())
}
fn context_admission(c: &RepairContext<'_>) -> Result<(), RepairError> {
    if c.policy.required_stages.is_empty()
        || c.policy.required_stages.len() > 64
        || c.policy
            .required_stages
            .iter()
            .any(|s| s.len() > 256 || s.trim().is_empty())
    {
        return Err(RepairError::Budget);
    }
    c.binding.check_budget().map_err(|_| RepairError::Budget)?;
    let mut left = 65536;
    crate::accepted_stage::admit(&c.policy.required_stages, &mut left)
        .map_err(|_| RepairError::Budget)?;
    crate::accepted_stage::admit(c.frozen, &mut left).map_err(|_| RepairError::Budget)?;
    crate::accepted_stage::admit(c.stage_plan.approval_policy(), &mut left)
        .map_err(|_| RepairError::Budget)?;
    Ok(())
}
impl ProtectedRepairPlan {
    pub fn freeze(
        c: &RepairContext<'_>,
        paths: &BTreeSet<String>,
        now: i64,
    ) -> Result<Self, RepairError> {
        context_admission(c)?;
        paths_admission(paths)?;
        let mut left = 65536;
        crate::accepted_stage::admit(paths, &mut left).map_err(|_| RepairError::Budget)?;
        let binding = c.binding.domain_digest();
        let base = c.stage_plan.approval_policy();
        let stages: BTreeSet<_> = c
            .frozen
            .obligations()
            .iter()
            .map(|o| o.stage_id.clone())
            .collect();
        if stages != c.policy.required_stages
            || c.stage_plan.context_digest() != binding
            || c.frozen.context_digest() != binding
            || c.stage_plan.graph_digest() != c.frozen.graph_digest()
            || base.binding != *c.binding.binding()
            || !base
                .required_scopes
                .contains(&format!("flowguard.frozen:{}", c.frozen.digest()))
            || !base
                .required_scopes
                .contains(&format!("git.scope:{}", c.binding.candidate_scope_digest()))
        {
            return Err(RepairError::Binding);
        }
        let approvers = base
            .approval_principals
            .get("stage.accept")
            .ok_or(RepairError::Action)?;
        let digest = crate::digest(
            &serde_json::to_vec(&(
                "flowguard.test-repair/v1alpha1",
                c.stage_plan.digest(),
                &binding,
                c.frozen.digest(),
                &c.policy.required_stages,
                c.policy.allow_test_repair,
                paths,
                now,
                base,
            ))
            .map_err(|_| RepairError::Binding)?,
        );
        let mut policy = base.clone();
        policy.action = format!("write-tests:{digest}");
        policy.approval_principals = BTreeMap::from([(PURPOSE.into(), approvers.clone())]);
        Ok(Self {
            digest,
            stage_plan: c.stage_plan.digest().into(),
            context: binding,
            frozen: c.frozen.digest().into(),
            applicability: c.policy.clone(),
            paths: paths.clone(),
            policy,
            frozen_at: now,
        })
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn approval_policy(&self) -> &EligibilityPolicy {
        &self.policy
    }
    fn validate(
        &self,
        c: &RepairContext<'_>,
        paths: &BTreeSet<String>,
        reference: &str,
        now: i64,
        provider: &dyn AuthorityProvider,
    ) -> Result<(), RepairError> {
        context_admission(c)?;
        paths_admission(paths)?;
        if reference.len() > 256
            || reference.trim().is_empty()
            || reference.chars().any(char::is_control)
        {
            return Err(RepairError::Budget);
        }
        if now < self.frozen_at {
            return Err(RepairError::Clock);
        }
        if self.stage_plan != c.stage_plan.digest()
            || self.context != c.binding.domain_digest()
            || self.frozen != c.frozen.digest()
            || self.applicability.required_stages != c.policy.required_stages
            || self.applicability.allow_test_repair != c.policy.allow_test_repair
        {
            return Err(RepairError::Binding);
        }
        if !self.applicability.allow_test_repair {
            return Err(RepairError::Action);
        }
        if !paths.is_subset(&self.paths) {
            return Err(RepairError::Scope);
        }
        let record = provider
            .verify_approval(reference)
            .map_err(|_| RepairError::Approval)?;
        validate_approval_record(&record, &self.policy, PURPOSE, now)
            .map_err(|_| RepairError::Approval)
    }
    pub fn authorize<'a>(
        &'a self,
        c: &RepairContext<'_>,
        request: &RepairRequest<'a>,
        provider: &dyn AuthorityProvider,
    ) -> Result<RepairReceipt<'a>, RepairError> {
        if super::requirements(request.action, c.policy) != super::ActionAssessment::ApprovalMissing
        {
            return Err(RepairError::Action);
        }
        self.validate(
            c,
            request.paths,
            request.approval_reference,
            request.now,
            provider,
        )?;
        Ok(RepairReceipt {
            plan: self,
            paths: request.paths,
            approval: request.approval_reference,
            last_checked: Cell::new(request.now),
        })
    }
}
impl RepairReceipt<'_> {
    pub fn paths(&self) -> &BTreeSet<String> {
        self.paths
    }
    pub fn refresh(
        &self,
        current: &ProtectedRepairPlan,
        c: &RepairContext<'_>,
        provider: &dyn AuthorityProvider,
        now: i64,
    ) -> Result<(), RepairError> {
        if current.digest != self.plan.digest {
            return Err(RepairError::Binding);
        }
        if now < self.last_checked.get() {
            return Err(RepairError::Clock);
        }
        self.plan
            .validate(c, self.paths, self.approval, now, provider)?;
        self.last_checked.set(now);
        Ok(())
    }
}
