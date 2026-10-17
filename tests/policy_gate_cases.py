"""独立验收表：显式期望，不从 Rego 结果反推正确答案。"""
from copy import deepcopy


def cases():
    subject, contract, policy, plan = ["sha256:" + digit * 64 for digit in "1234"]
    base = {
        "schemaVersion": "guard.policy-input/v1",
        "binding": {"repositoryId": "repo", "taskId": "task", "subjectDigest": subject,
                    "contractDigest": contract, "policyDigest": policy, "planDigest": plan,
                    "action": "BRANCH_CREATE"},
        "facts": {"schemaVersion": 1, "mode": "cooperative", "action": "BRANCH_CREATE",
                  "subjectDigest": subject, "contractDigest": contract, "nowEpochSeconds": 100,
                  "obligations": [{"obligationId": "o", "ruleId": "r", "verifierId": "v",
                                   "target": "src/a", "required": True, "mode": "ENFORCE",
                                   "nonWaivable": False}],
                  "results": [{"resultId": "result", "obligationId": "o", "ruleId": "r",
                               "verifierId": "v", "target": "src/a", "subjectDigest": subject,
                               "contractDigest": contract, "policyDigest": policy, "planDigest": plan,
                               "execution": "SUCCEEDED", "coverage": "COMPLETE", "verdict": "SATISFIED",
                               "findings": []}],
                  "stages": [], "approvals": [], "exceptions": [],
                  "authorityContext": {"verifiedApprovalIds": [], "verifiedResultIds": ["result"],
                                       "verifiedContractDigest": contract, "verifiedSubjectDigest": subject,
                                       "enforcementVerified": False}}
    }
    table = []

    def add(name, value, decision, missing=0, findings=0):
        table.append({"name": name, "input": deepcopy(value), "expectedDecision": decision,
                      "expectedMissing": missing, "expectedFindings": findings})

    add("complete", base, "ALLOW")
    missing = deepcopy(base)
    missing["facts"]["results"] = []
    add("required-test-not-run", missing, "INCOMPLETE", 1)
    positive = deepcopy(base)
    positive["facts"]["results"][0].update(verdict="VIOLATED", findings=[{"findingId": "f", "message": "certain violation"}])
    add("known-violation", positive, "DENY", findings=1)
    partial = deepcopy(positive)
    partial["facts"]["results"][0].update(execution="FAILED", coverage="PARTIAL")
    add("partial-positive-retained", partial, "DENY", 1, 1)
    for mode, decision in [("REVIEW", "REVIEW_REQUIRED"), ("ADVISE", "ALLOW")]:
        value = deepcopy(positive)
        value["facts"]["obligations"][0]["mode"] = mode
        add("rule-" + mode.lower(), value, decision, findings=1)
    approval = {"id": "approval", "kind": "exception", "actor": "reviewer", "requestedBy": "coder",
                "action": "BRANCH_CREATE", "subjectDigest": subject, "contractDigest": contract,
                "scope": {"obligationId": "o", "ruleId": "r", "target": "src/a", "findingId": "f"},
                "notBefore": 90, "expiresAt": 200, "revoked": False}
    waived = deepcopy(positive)
    waived["facts"].update(approvals=[approval], exceptions=[{
        "id": "e", "obligationId": "o", "ruleId": "r", "target": "src/a", "findingId": "f",
        "approvalId": "approval", "reason": "temporary migration"}])
    waived["facts"]["authorityContext"]["verifiedApprovalIds"] = ["approval"]
    add("exact-exception", waived, "ALLOW_WITH_EXCEPTIONS", findings=1)
    failed_waived = deepcopy(waived)
    failed_waived["facts"]["results"][0]["execution"] = "FAILED"
    add("actual-failure-not-normal-domain-completion", failed_waived, "INCOMPLETE", 1, 1)
    failed_complete = deepcopy(positive)
    failed_complete["facts"]["results"][0]["execution"] = "FAILED"
    add("actual-failure-with-complete-coverage-keeps-positive", failed_complete, "DENY", 1, 1)
    for name, field, value in [("expired", "expiresAt", 100), ("revoked", "revoked", True),
                               ("self-approved", "actor", "coder"), ("future", "notBefore", 101)]:
        item = deepcopy(waived)
        item["facts"]["approvals"][0][field] = value
        add(name, item, "DENY", findings=1)
    nonwaivable = deepcopy(waived)
    nonwaivable["facts"]["obligations"][0].update(nonWaivable=True, mode="ADVISE")
    add("nonwaivable-even-advisory", nonwaivable, "DENY", findings=1)
    incomplete_waiver = deepcopy(waived)
    incomplete_waiver["facts"]["results"][0]["coverage"] = "PARTIAL"
    add("exception-not-a-test-result", incomplete_waiver, "INCOMPLETE", 1, 1)
    untrusted = deepcopy(base)
    untrusted["facts"]["authorityContext"]["verifiedResultIds"] = []
    add("unverified-result", untrusted, "INCOMPLETE", 1)
    enforced = deepcopy(base)
    enforced["facts"]["mode"] = "enforced"
    add("enforced-host-not-proven", enforced, "INCOMPLETE")
    for field in ["subjectDigest", "contractDigest", "policyDigest", "planDigest"]:
        value = deepcopy(base)
        value["facts"]["results"][0][field] = "sha256:" + "9" * 64
        add("wrong-" + field, value, "INCOMPLETE", 1)
    for mode in ["advisory", "cooperative", "enforced"]:
        value = deepcopy(positive)
        value["facts"]["mode"] = mode
        add("operating-" + mode, value, "DENY", findings=1)
    commit = deepcopy(base)
    commit["binding"]["action"] = commit["facts"]["action"] = "COMMIT"
    commit["facts"]["stages"] = [{"stageId": f"{i:02}", "state": "accepted", "approvalId": ""} for i in range(1, 10)]
    add("accepted-document-not-authority", commit, "REVIEW_REQUIRED")
    commit["facts"]["results"] = []
    add("accepted-document-not-tests", commit, "INCOMPLETE", 1)
    add("malformed-unknown-wrapper", {}, "INCOMPLETE")
    empty = deepcopy(base)
    empty["facts"].update(obligations=[], results=[])
    add("empty-plan-not-vacuous-success", empty, "INCOMPLETE")
    return table
