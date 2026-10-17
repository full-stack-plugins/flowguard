# FlowGuard 拥有流程组合语义。输入 authorityContext 必须由独立调用端核验，不能来自候选文本。
package flowguard.gate

import rego.v1

closed(value, keys) if {
	is_object(value)
	object.keys(value) == keys
}

identifier(value) if {
	is_string(value)
	count(value) > 0
	count(value) <= 4096
}

digest(value) if {
	is_string(value)
	regex.match("^sha256:[0-9a-f]{64}$", value)
}

integer(value) if {
	is_number(value)
	value == floor(value)
	value >= 0
	value <= 9007199254740991
}

strings(values) if {
	is_array(values)
	every value in values { identifier(value) }
	count(values) <= 4096
}

valid_binding if {
	closed(input, {"schemaVersion", "binding", "facts"})
	input.schemaVersion == "guard.policy-input/v1"
	b := input.binding
	closed(b, {"repositoryId", "taskId", "subjectDigest", "contractDigest", "policyDigest", "planDigest", "action"})
	every _, value in b { identifier(value) }
	every k in {"subjectDigest", "contractDigest", "policyDigest", "planDigest"} { digest(b[k]) }
	b.action in {"BRANCH_CREATE", "BRANCH_SWITCH", "COMMIT", "PUSH", "MERGE", "RELEASE", "DEPLOY"}
}

valid_obligation(o) if {
	closed(o, {"obligationId", "ruleId", "verifierId", "target", "required", "mode", "nonWaivable"})
	every k in {"obligationId", "ruleId", "verifierId", "target"} { identifier(o[k]) }
	is_boolean(o.required)
	is_boolean(o.nonWaivable)
	o.mode in {"ENFORCE", "REVIEW", "ADVISE"}
}

valid_finding(f) if {
	closed(f, {"findingId", "message"})
	identifier(f.findingId)
	identifier(f.message)
}

valid_result(r) if {
	closed(r, {"resultId", "obligationId", "ruleId", "verifierId", "target", "subjectDigest", "contractDigest", "policyDigest", "planDigest", "execution", "coverage", "verdict", "findings"})
	every k in {"resultId", "obligationId", "ruleId", "verifierId", "target", "subjectDigest", "contractDigest"} { identifier(r[k]) }
	every k in {"subjectDigest", "contractDigest", "policyDigest", "planDigest"} { digest(r[k]) }
	r.execution in {"SUCCEEDED", "FAILED", "NOT_RUN", "TIMED_OUT", "CANCELLED"}
	r.coverage in {"COMPLETE", "PARTIAL", "NONE"}
	r.verdict in {"SATISFIED", "VIOLATED", "UNKNOWN", "NOT_APPLICABLE"}
	is_array(r.findings)
	count(r.findings) <= 4096
	every f in r.findings { valid_finding(f) }
	count({f.findingId | some f in r.findings}) == count(r.findings)
}

valid_stage(s) if {
	closed(s, {"stageId", "state", "approvalId"})
	s.stageId in {"01", "02", "03", "04", "05", "06", "07", "08", "09", "10"}
	s.state in {"pending", "in_progress", "pending_acceptance", "accepted", "inherited", "skipped", "invalidated"}
	is_string(s.approvalId)
}

valid_scope(a) if {
	a.kind == "stage"
	closed(a.scope, {"stageId", "state"})
	a.scope.stageId in {"01", "02", "03", "04", "05", "06", "07", "08", "09", "10"}
	a.scope.state in {"accepted", "inherited", "skipped"}
	a.subjectDigest == null
}

valid_scope(a) if {
	a.kind == "exception"
	closed(a.scope, {"obligationId", "ruleId", "target", "findingId"})
	every _, value in a.scope { identifier(value) }
	identifier(a.subjectDigest)
}

valid_scope(a) if {
	a.kind == "action"
	closed(a.scope, {"action"})
	a.scope.action == a.action
	identifier(a.subjectDigest)
}

valid_approval(a) if {
	closed(a, {"id", "kind", "actor", "requestedBy", "action", "subjectDigest", "contractDigest", "scope", "notBefore", "expiresAt", "revoked"})
	every k in {"id", "actor", "requestedBy", "action", "contractDigest"} { identifier(a[k]) }
	valid_scope(a)
	integer(a.notBefore)
	integer(a.expiresAt)
	a.expiresAt > a.notBefore
	is_boolean(a.revoked)
}

valid_exception(e) if {
	closed(e, {"id", "obligationId", "ruleId", "target", "findingId", "approvalId", "reason"})
	every _, value in e { identifier(value) }
}

valid_authority if {
	a := input.facts.authorityContext
	closed(a, {"verifiedApprovalIds", "verifiedResultIds", "verifiedContractDigest", "verifiedSubjectDigest", "enforcementVerified"})
	strings(a.verifiedApprovalIds)
	strings(a.verifiedResultIds)
	a.verifiedContractDigest == input.binding.contractDigest
	a.verifiedSubjectDigest == input.binding.subjectDigest
	is_boolean(a.enforcementVerified)
}

rule_conflict if {
	some a in input.facts.obligations
	some b in input.facts.obligations
	a.ruleId == b.ruleId
	[a.mode, a.nonWaivable, a.verifierId] != [b.mode, b.nonWaivable, b.verifierId]
}

valid_input if {
	valid_binding
	f := input.facts
	closed(f, {"schemaVersion", "mode", "action", "subjectDigest", "contractDigest", "nowEpochSeconds", "obligations", "results", "stages", "approvals", "exceptions", "authorityContext"})
	f.schemaVersion == 1
	f.mode in {"advisory", "cooperative", "enforced"}
	f.action == input.binding.action
	f.subjectDigest == input.binding.subjectDigest
	f.contractDigest == input.binding.contractDigest
	integer(f.nowEpochSeconds)
	valid_authority
	every name in {"obligations", "results", "stages", "approvals", "exceptions"} { is_array(f[name]); count(f[name]) <= 4096}
	every o in f.obligations { valid_obligation(o) }
	count(f.obligations) > 0
	not rule_conflict
	every r in f.results { some o in f.obligations; valid_obligation(o); r.obligationId == o.obligationId}
	every r in f.results { valid_result(r) }
	every s in f.stages { valid_stage(s) }
	every a in f.approvals { valid_approval(a) }
	every e in f.exceptions { valid_exception(e) }
	count({o.obligationId | some o in f.obligations}) == count(f.obligations)
	count({r.resultId | some r in f.results}) == count(f.results)
	count({s.stageId | some s in f.stages}) == count(f.stages)
	count({a.id | some a in f.approvals}) == count(f.approvals)
	count({e.id | some e in f.exceptions}) == count(f.exceptions)
}

# 投影只消费已验证、当前候选精确匹配的领域结果，不采用 accepted 或自行声明 trusted。
matching(o, r) if {
	valid_binding
	valid_authority
	valid_obligation(o)
	valid_result(r)
	r.resultId in input.facts.authorityContext.verifiedResultIds
	r.obligationId == o.obligationId
	r.ruleId == o.ruleId
	r.verifierId == o.verifierId
	r.target == o.target
	r.subjectDigest == input.binding.subjectDigest
	r.contractDigest == input.binding.contractDigest
	r.policyDigest == input.binding.policyDigest
	r.planDigest == input.binding.planDigest
}

usable(o) if {
	matches := [r | some r in input.facts.results; matching(o, r)]
	count(matches) == 1
	r := matches[0]
	r.execution == "SUCCEEDED"
	r.coverage == "COMPLETE"
	r.verdict in {"SATISFIED", "VIOLATED"}
	r.verdict == "SATISFIED"
	count(r.findings) == 0
}

usable(o) if {
	matches := [r | some r in input.facts.results; matching(o, r)]
	count(matches) == 1
	r := matches[0]
	r.execution == "SUCCEEDED"
	r.coverage == "COMPLETE"
	r.verdict == "VIOLATED"
	count(r.findings) > 0
}

approval_active(a) if {
	valid_approval(a)
	valid_authority
	a.id in input.facts.authorityContext.verifiedApprovalIds
	a.actor != a.requestedBy
	a.action == input.binding.action
	a.contractDigest == input.binding.contractDigest
	not a.revoked
	a.notBefore <= input.facts.nowEpochSeconds
	a.expiresAt > input.facts.nowEpochSeconds
}

waived(o, finding) if {
	not o.nonWaivable
	some e in input.facts.exceptions
	valid_exception(e)
	e.obligationId == o.obligationId
	e.ruleId == o.ruleId
	e.target == o.target
	e.findingId == finding.findingId
	some a in input.facts.approvals
	a.id == e.approvalId
	a.kind == "exception"
	a.subjectDigest == input.binding.subjectDigest
	a.scope == {"obligationId": o.obligationId, "ruleId": o.ruleId, "target": o.target, "findingId": finding.findingId}
	approval_active(a)
}

violation_set contains v if {
	some o in input.facts.obligations
	some r in input.facts.results
	matching(o, r)
	some f in r.findings
	v := {"obligationId": o.obligationId, "ruleId": o.ruleId, "target": o.target, "findingId": f.findingId, "message": f.message, "mode": o.mode, "nonWaivable": o.nonWaivable, "waived": false}
	not waived(o, f)
}

violation_set contains v if {
	some o in input.facts.obligations
	some r in input.facts.results
	matching(o, r)
	some f in r.findings
	waived(o, f)
	v := {"obligationId": o.obligationId, "ruleId": o.ruleId, "target": o.target, "findingId": f.findingId, "message": f.message, "mode": o.mode, "nonWaivable": o.nonWaivable, "waived": true}
}

missing_set contains o.obligationId if {
	some o in input.facts.obligations
	valid_obligation(o)
	o.required
	not usable(o)
}

required_stages := {"01", "02", "03", "04", "05", "06", "07", "08", "09"} if input.binding.action in {"COMMIT", "PUSH", "MERGE"}

else := {"01", "02", "03", "04", "05", "06", "07", "08", "09", "10"} if input.binding.action in {"DEPLOY", "RELEASE"}

else := set()

stage_present(id) if {
	some s in input.facts.stages
	valid_stage(s)
	s.stageId == id
	s.state in {"accepted", "inherited", "skipped"}
}

stage_approved(id) if {
	some s in input.facts.stages
	valid_stage(s)
	s.stageId == id
	s.state in {"accepted", "inherited", "skipped"}
	some a in input.facts.approvals
	a.id == s.approvalId
	a.kind == "stage"
	a.scope == {"stageId": id, "state": s.state}
	approval_active(a)
}

action_approved if {
	some a in input.facts.approvals
	a.kind == "action"
	a.subjectDigest == input.binding.subjectDigest
	a.scope == {"action": input.binding.action}
	approval_active(a)
}

reason_set contains "INVALID_INPUT" if not valid_input
reason_set contains "MISSING_TECHNICAL_EVIDENCE" if count(missing_set) > 0

reason_set contains "ENFORCEMENT_NOT_VERIFIED" if {
	input.facts.mode == "enforced"
	not input.facts.authorityContext.enforcementVerified
}

reason_set contains sprintf("MISSING_STAGE:%s", [id]) if {
	some id in required_stages
	not stage_present(id)
}

reason_set contains sprintf("STAGE_APPROVAL_REQUIRED:%s", [id]) if {
	some id in required_stages
	stage_present(id)
	not stage_approved(id)
}

reason_set contains "ACTION_APPROVAL_REQUIRED" if {
	input.binding.action in {"MERGE", "RELEASE", "DEPLOY"}
	not action_approved
}

reason_set contains "ACTIVE_ENFORCEMENT_VIOLATION" if {
	some v in violation_set
	not v.waived
	v.mode == "ENFORCE"
}

reason_set contains "NON_WAIVABLE_VIOLATION" if {
	some v in violation_set
	v.nonWaivable
}

reason_set contains "REVIEW_REQUIRED" if {
	some v in violation_set
	not v.waived
	v.mode == "REVIEW"
}

reason_set contains "ADVISORY_FINDINGS" if {
	some v in violation_set
	not v.waived
	v.mode == "ADVISE"
}

reason_set contains "SCOPED_EXCEPTION_APPLIED" if {
	some v in violation_set
	v.waived
}

hard_denial if "ACTIVE_ENFORCEMENT_VIOLATION" in reason_set
hard_denial if "NON_WAIVABLE_VIOLATION" in reason_set
incomplete if "INVALID_INPUT" in reason_set
incomplete if "MISSING_TECHNICAL_EVIDENCE" in reason_set
incomplete if "ENFORCEMENT_NOT_VERIFIED" in reason_set

incomplete if {
	some id in required_stages
	not stage_present(id)
}

review if "REVIEW_REQUIRED" in reason_set

review if {
	some id in required_stages
	stage_present(id)
	not stage_approved(id)
}

review if "ACTION_APPROVAL_REQUIRED" in reason_set

has_exception if "SCOPED_EXCEPTION_APPLIED" in reason_set

outcome := "DENY" if hard_denial

else := "INCOMPLETE" if incomplete

else := "REVIEW_REQUIRED" if review

else := "ALLOW_WITH_EXCEPTIONS" if has_exception

else := "ALLOW"

decision := {"schemaVersion": 1, "decision": outcome, "reasons": sort(reason_set), "requiredActions": sort(reason_set), "missingObligations": sort(missing_set), "activeViolations": sort(violation_set)}
