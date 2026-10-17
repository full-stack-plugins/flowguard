package flowguard.gate_test

import rego.v1

base := {
	"schemaVersion": "guard.policy-input/v1",
	"binding": {"repositoryId": "repo", "taskId": "task", "subjectDigest": "sha256:1111111111111111111111111111111111111111111111111111111111111111", "contractDigest": "sha256:2222222222222222222222222222222222222222222222222222222222222222", "policyDigest": "sha256:3333333333333333333333333333333333333333333333333333333333333333", "planDigest": "sha256:4444444444444444444444444444444444444444444444444444444444444444", "action": "BRANCH_CREATE"},
	"facts": {
		"schemaVersion": 1, "mode": "cooperative", "action": "BRANCH_CREATE", "subjectDigest": "sha256:1111111111111111111111111111111111111111111111111111111111111111", "contractDigest": "sha256:2222222222222222222222222222222222222222222222222222222222222222", "nowEpochSeconds": 100,
		"obligations": [{"obligationId": "o", "ruleId": "r", "verifierId": "v", "target": "src/a", "required": true, "mode": "ENFORCE", "nonWaivable": false}],
		"results": [{"resultId": "result", "obligationId": "o", "ruleId": "r", "verifierId": "v", "target": "src/a", "subjectDigest": "sha256:1111111111111111111111111111111111111111111111111111111111111111", "contractDigest": "sha256:2222222222222222222222222222222222222222222222222222222222222222", "policyDigest": "sha256:3333333333333333333333333333333333333333333333333333333333333333", "planDigest": "sha256:4444444444444444444444444444444444444444444444444444444444444444", "execution": "SUCCEEDED", "coverage": "COMPLETE", "verdict": "SATISFIED", "findings": []}],
		"stages": [], "approvals": [], "exceptions": [],
		"authorityContext": {"verifiedApprovalIds": [], "verifiedResultIds": ["result"], "verifiedContractDigest": "sha256:2222222222222222222222222222222222222222222222222222222222222222", "verifiedSubjectDigest": "sha256:1111111111111111111111111111111111111111111111111111111111111111", "enforcementVerified": false},
	},
}

test_complete_requires_real_policy if {
	result := data.flowguard.gate.decision with input as base
	result.decision == "ALLOW"
}

test_missing_required_not_accepted_text if {
	facts := object.union(base.facts, {"results": []})
	result := data.flowguard.gate.decision with input as object.union(base, {"facts": facts})
	result.decision == "INCOMPLETE"
	count(result.missingObligations) == 1
}

finding := {"findingId": "f", "message": "confirmed forbidden dependency"}
violation := object.union(base.facts.results[0], {"verdict": "VIOLATED", "findings": [finding]})

evaluate(facts) := result if result := data.flowguard.gate.decision with input as object.union(base, {"facts": facts})

exception := {"id": "e", "obligationId": "o", "ruleId": "r", "target": "src/a", "findingId": "f", "approvalId": "approval", "reason": "temporary migration"}
approval := {"id": "approval", "kind": "exception", "actor": "independent-reviewer", "requestedBy": "coding-agent", "action": "BRANCH_CREATE", "subjectDigest": "sha256:1111111111111111111111111111111111111111111111111111111111111111", "contractDigest": "sha256:2222222222222222222222222222222222222222222222222222222222222222", "scope": {"obligationId": "o", "ruleId": "r", "target": "src/a", "findingId": "f"}, "notBefore": 90, "expiresAt": 200, "revoked": false}
exception_facts := object.union(base.facts, {"results": [violation], "exceptions": [exception], "approvals": [approval], "authorityContext": object.union(base.facts.authorityContext, {"verifiedApprovalIds": ["approval"]})})

test_partial_positive_keeps_violation_and_missing if {
	result := evaluate(object.union(base.facts, {"results": [object.union(violation, {"coverage": "PARTIAL", "execution": "FAILED"})]}))
	result.decision == "DENY"
	count(result.activeViolations) == 1
	result.missingObligations == ["o"]
	"MISSING_TECHNICAL_EVIDENCE" in result.reasons
}

test_unknown_cannot_pass if {
	result := evaluate(object.union(base.facts, {"results": [object.union(base.facts.results[0], {"verdict": "UNKNOWN"})]}))
	result.decision == "INCOMPLETE"
}

test_not_applicable_without_explicit_applicability_proof_cannot_pass if {
	result := evaluate(object.union(base.facts, {"results": [object.union(base.facts.results[0], {"verdict": "NOT_APPLICABLE"})]}))
	result.decision == "INCOMPLETE"
}

test_failed_empty_result_cannot_pass if {
	every execution in ["FAILED", "NOT_RUN", "TIMED_OUT", "CANCELLED"] {
		result := evaluate(object.union(base.facts, {"results": [object.union(base.facts.results[0], {"execution": execution})]}))
		result.decision == "INCOMPLETE"
	}
}

test_verified_reference_is_required if {
	result := evaluate(object.union(base.facts, {"authorityContext": object.union(base.facts.authorityContext, {"verifiedResultIds": []})}))
	result.decision == "INCOMPLETE"
}

test_wrong_subject_rule_verifier_target_or_contract_cannot_cover if {
	every field in ["subjectDigest", "contractDigest", "ruleId", "verifierId", "target", "obligationId"] {
		result := evaluate(object.union(base.facts, {"results": [object.union(base.facts.results[0], {field: "wrong"})]}))
		result.decision == "INCOMPLETE"
	}
}

test_review_and_advice_are_approved_rule_dispositions if {
	review := evaluate(object.union(base.facts, {"results": [violation], "obligations": [object.union(base.facts.obligations[0], {"mode": "REVIEW"})]}))
	review.decision == "REVIEW_REQUIRED"
	advice := evaluate(object.union(base.facts, {"results": [violation], "obligations": [object.union(base.facts.obligations[0], {"mode": "ADVISE"})]}))
	advice.decision == "ALLOW"
	count(advice.activeViolations) == 1
}

test_operating_modes_do_not_hide_enforce_violations if {
	every mode in ["advisory", "cooperative", "enforced"] {
		result := evaluate(object.union(base.facts, {"mode": mode, "results": [violation]}))
		result.decision == "DENY"
		count(result.activeViolations) == 1
	}
}

test_enforced_claim_requires_independent_host_evidence if {
	result := evaluate(object.union(base.facts, {"mode": "enforced"}))
	result.decision == "INCOMPLETE"
	"ENFORCEMENT_NOT_VERIFIED" in result.reasons
	allowed := evaluate(object.union(base.facts, {"mode": "enforced", "authorityContext": object.union(base.facts.authorityContext, {"enforcementVerified": true})}))
	allowed.decision == "ALLOW"
}

test_exact_scoped_exception_preserves_original_finding if {
	result := evaluate(exception_facts)
	result.decision == "ALLOW_WITH_EXCEPTIONS"
	count(result.activeViolations) == 1
	result.activeViolations[0].findingId == "f"
	result.activeViolations[0].waived
}

test_nonwaivable_overrides_disposition_and_approval if {
	every mode in ["ENFORCE", "REVIEW", "ADVISE"] {
		result := evaluate(object.union(exception_facts, {"obligations": [object.union(base.facts.obligations[0], {"mode": mode, "nonWaivable": true})]}))
		result.decision == "DENY"
		not result.activeViolations[0].waived
	}
}

test_expired_revoked_future_and_self_approval_do_not_waive if {
	every change in [{"expiresAt": 100}, {"revoked": true}, {"notBefore": 101}, {"actor": "coding-agent"}, {"subjectDigest": "wrong"}, {"contractDigest": "wrong"}, {"action": "RELEASE"}] {
		result := evaluate(object.union(exception_facts, {"approvals": [object.union(approval, change)]}))
		result.decision == "DENY"
		not result.activeViolations[0].waived
	}
}

test_unverified_exception_approval_cannot_waive if {
	result := evaluate(object.union(exception_facts, {"authorityContext": base.facts.authorityContext}))
	result.decision == "DENY"
}

test_exception_must_match_each_exact_scope_dimension if {
	every field in ["obligationId", "ruleId", "target", "findingId"] {
		result := evaluate(object.union(exception_facts, {"exceptions": [object.union(exception, {field: "other"})]}))
		result.decision == "DENY"
	}
}

test_exception_does_not_cover_missing_technical_execution if {
	result := evaluate(object.union(exception_facts, {"results": [object.union(violation, {"coverage": "PARTIAL"})]}))
	result.decision == "INCOMPLETE"
	result.activeViolations[0].waived
	result.missingObligations == ["o"]
}

test_duplicate_current_results_cannot_claim_complete if {
	result := evaluate(object.union(base.facts, {"results": [base.facts.results[0], object.union(base.facts.results[0], {"resultId": "result2"})], "authorityContext": object.union(base.facts.authorityContext, {"verifiedResultIds": ["result", "result2"]})}))
	result.decision == "INCOMPLETE"
}

test_invalid_shape_is_closed if {
	every change in [{"approved": true}, {"schemaVersion": 2}, {"mode": "best-effort"}, {"nowEpochSeconds": 1.5}, {"nowEpochSeconds": -1}, {"action": "RELEASE"}] {
		result := evaluate(object.union(base.facts, change))
		result.decision == "INCOMPLETE"
	}
}

test_duplicate_rules_and_unknown_nested_fields_block if {
	result := evaluate(object.union(base.facts, {"obligations": [base.facts.obligations[0], base.facts.obligations[0]]}))
	result.decision == "INCOMPLETE"
	nested := evaluate(object.union(base.facts, {"results": [object.union(base.facts.results[0], {"trusted": true})]}))
	nested.decision == "INCOMPLETE"
}

test_authority_mismatch_cannot_activate_report if {
	result := evaluate(object.union(base.facts, {"results": [violation], "authorityContext": object.union(base.facts.authorityContext, {"verifiedSubjectDigest": "wrong"})}))
	result.decision == "INCOMPLETE"
	count(result.activeViolations) == 0
}

test_invalid_other_report_does_not_erase_confirmed_positive if {
	result := evaluate(object.union(base.facts, {"results": [violation, {"invalid": true}]}))
	result.decision == "DENY"
	count(result.activeViolations) == 1
	"INVALID_INPUT" in result.reasons
}

commit_binding := object.union(base.binding, {"action": "COMMIT"})
commit_stages := [{"stageId": id, "state": "accepted", "approvalId": concat("", ["stage-", id])} | some id in ["01", "02", "03", "04", "05", "06", "07", "08", "09"]]
commit_approvals := [object.union(object.remove(approval, {"scope"}), {"id": concat("", ["stage-", id]), "kind": "stage", "action": "COMMIT", "subjectDigest": null, "scope": {"stageId": id, "state": "accepted"}}) | some id in ["01", "02", "03", "04", "05", "06", "07", "08", "09"]]
commit_facts := object.union(base.facts, {"action": "COMMIT", "stages": commit_stages, "approvals": commit_approvals, "authorityContext": object.union(base.facts.authorityContext, {"verifiedApprovalIds": [a.id | some a in commit_approvals]})})

test_accepted_stage_text_cannot_replace_independent_approvals if {
	facts := object.union(commit_facts, {"approvals": []})
	result := data.flowguard.gate.decision with input as object.union(base, {"binding": commit_binding, "facts": facts})
	result.decision == "REVIEW_REQUIRED"
	count(result.reasons) == 9
}

test_approved_stages_never_replace_technical_evidence if {
	facts := object.union(commit_facts, {"results": []})
	result := data.flowguard.gate.decision with input as object.union(base, {"binding": commit_binding, "facts": facts})
	result.decision == "INCOMPLETE"
	result.missingObligations == ["o"]
}

test_all_commit_stages_approved_and_technical_complete if {
	result := data.flowguard.gate.decision with input as object.union(base, {"binding": commit_binding, "facts": commit_facts})
	result.decision == "ALLOW"
}

test_stage_approval_scope_reusable_across_candidate_body_changes if {
	binding := object.union(commit_binding, {"subjectDigest": "sha256:5555555555555555555555555555555555555555555555555555555555555555"})
	result_now := object.union(base.facts.results[0], {"subjectDigest": "sha256:5555555555555555555555555555555555555555555555555555555555555555"})
	facts := object.union(commit_facts, {"subjectDigest": "sha256:5555555555555555555555555555555555555555555555555555555555555555", "results": [result_now], "authorityContext": object.union(commit_facts.authorityContext, {"verifiedSubjectDigest": "sha256:5555555555555555555555555555555555555555555555555555555555555555"})})
	result := data.flowguard.gate.decision with input as object.union(base, {"binding": binding, "facts": facts})
	result.decision == "ALLOW"
}

test_missing_or_invalidated_stage_blocks if {
	every state in ["pending", "in_progress", "pending_acceptance", "invalidated"] {
		stages := array.concat([object.union(commit_stages[0], {"state": state})], array.slice(commit_stages, 1, 9))
		result := data.flowguard.gate.decision with input as object.union(base, {"binding": commit_binding, "facts": object.union(commit_facts, {"stages": stages})})
		result.decision == "INCOMPLETE"
	}
}

test_release_requires_tenth_stage_and_candidate_acceptance if {
	b := object.union(base.binding, {"action": "RELEASE"})
	f := object.union(commit_facts, {"action": "RELEASE"})
	result := data.flowguard.gate.decision with input as object.union(base, {"binding": b, "facts": f})
	result.decision == "INCOMPLETE"
	"MISSING_STAGE:10" in result.reasons
	"ACTION_APPROVAL_REQUIRED" in result.reasons
}

test_empty_obligation_set_cannot_vacuously_pass if {
	result := evaluate(object.union(base.facts, {"obligations": [], "results": []}))
	result.decision == "INCOMPLETE"
}

test_result_from_other_policy_or_plan_is_not_current if {
	every field in ["policyDigest", "planDigest"] {
		result := evaluate(object.union(base.facts, {"results": [object.union(base.facts.results[0], {field: concat("", ["sha256:", "9999999999999999999999999999999999999999999999999999999999999999"])})]}))
		result.decision == "INCOMPLETE"
	}
}

test_optional_missing_is_not_required_missing if {
	result := evaluate(object.union(base.facts, {"obligations": [object.union(base.facts.obligations[0], {"required": false})], "results": []}))
	result.decision == "ALLOW"
	count(result.missingObligations) == 0
}

test_review_technical_gap_still_incomplete if {
	every mode in ["REVIEW", "ADVISE"] {
		result := evaluate(object.union(base.facts, {"obligations": [object.union(base.facts.obligations[0], {"mode": mode})], "results": []}))
		result.decision == "INCOMPLETE"
	}
}

test_exception_cannot_approve_second_finding if {
	second := {"findingId": "f2", "message": "second confirmed finding"}
	result := evaluate(object.union(exception_facts, {"results": [object.union(violation, {"findings": [finding, second]})]}))
	result.decision == "DENY"
	count(result.activeViolations) == 2
	count({v | some v in result.activeViolations; v.waived}) == 1
}

test_inherited_and_skipped_require_corresponding_scope if {
	every state in ["inherited", "skipped"] {
		stages := [object.union(s, {"state": state}) | some s in commit_stages]
		bad := data.flowguard.gate.decision with input as object.union(base, {"binding": commit_binding, "facts": object.union(commit_facts, {"stages": stages})})
		bad.decision == "REVIEW_REQUIRED"
		approvals := [object.union(a, {"scope": {"state": state}}) | some a in commit_approvals]
		good := data.flowguard.gate.decision with input as object.union(base, {"binding": commit_binding, "facts": object.union(commit_facts, {"stages": stages, "approvals": approvals})})
		good.decision == "ALLOW"
	}
}

test_release_approved_all_stages_and_action if {
	b := object.union(base.binding, {"action": "RELEASE"})
	ids := ["01", "02", "03", "04", "05", "06", "07", "08", "09", "10"]
	stages := [{"stageId": id, "state": "accepted", "approvalId": concat("", ["stage-", id])} | some id in ids]
	approvals := [object.union(object.remove(approval, {"scope"}), {"id": concat("", ["stage-", id]), "kind": "stage", "action": "RELEASE", "subjectDigest": null, "scope": {"stageId": id, "state": "accepted"}}) | some id in ids]
	action := object.union(object.remove(approval, {"scope"}), {"id": "release-approval", "kind": "action", "action": "RELEASE", "scope": {"action": "RELEASE"}})
	all := array.concat(approvals, [action])
	facts := object.union(base.facts, {"action": "RELEASE", "stages": stages, "approvals": all, "authorityContext": object.union(base.facts.authorityContext, {"verifiedApprovalIds": [a.id | some a in all]})})
	result := data.flowguard.gate.decision with input as object.union(base, {"binding": b, "facts": facts})
	result.decision == "ALLOW"
	old := object.union(action, {"subjectDigest": "wrong"})
	old_approvals := array.concat(approvals, [old])
	stale := data.flowguard.gate.decision with input as object.union(base, {"binding": b, "facts": object.union(facts, {"approvals": old_approvals})})
	stale.decision == "REVIEW_REQUIRED"
}

test_malformed_wrapper_returns_defined_incomplete if {
	every value in [null, [], {}, {"schemaVersion": "guard.policy-input/v2"}] {
		result := data.flowguard.gate.decision with input as value
		result.decision == "INCOMPLETE"
	}
}

test_same_rule_identity_cannot_change_disposition_between_targets if {
	first := object.union(base.facts.obligations[0], {"required": false})
	second := object.union(first, {"obligationId": "second", "target": "src/b", "mode": "ADVISE"})
	result := evaluate(object.union(base.facts, {"obligations": [first, second], "results": []}))
	result.decision == "INCOMPLETE"
	"INVALID_INPUT" in result.reasons
}

# SUCCEEDED 是独立领域核验后的“验证完成”，不是 OS exit0；不在本测试伪造 raw→facts 适配器。
test_failed_execution_complete_coverage_still_missing_even_with_exception if {
	failed := object.union(violation, {"execution": "FAILED"})
	result := evaluate(object.union(exception_facts, {"results": [failed]}))
	result.decision == "INCOMPLETE"
	result.missingObligations == ["o"]
	count(result.activeViolations) == 1
	result.activeViolations[0].waived
}

test_failed_execution_does_not_erase_confirmed_positive if {
	failed := object.union(violation, {"execution": "FAILED"})
	result := evaluate(object.union(base.facts, {"results": [failed]}))
	result.decision == "DENY"
	result.missingObligations == ["o"]
	count(result.activeViolations) == 1
}
