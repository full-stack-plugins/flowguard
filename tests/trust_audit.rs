mod common;
use common::*;
use flowguard::{audit::*, gate::*, run_store::MemoryRunStore, trust::*};
use guardengine::integration::{GuardRunEnvelope, eligibility::*};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};
struct Caller(RefCell<CallerRecord>);
impl CallerPort for Caller {
    fn resolve(&self, reference: &str) -> Result<CallerRecord, AuthorityError> {
        if reference != "token-DO-NOT-LOG" {
            return Err(AuthorityError::Untrusted);
        }
        Ok(self.0.borrow().clone())
    }
}
fn caller() -> Caller {
    Caller(RefCell::new(CallerRecord {
        tenant: "tenant-DO-NOT-LOG".into(),
        principal: "caller-DO-NOT-LOG".into(),
        validity: Validity {
            issued_at: NOW - 1,
            expires_at: NOW + 100,
            revoked: false,
        },
    }))
}
fn access(c: &Caller, now: i64) -> Access<'_> {
    Access {
        port: c,
        reference: "token-DO-NOT-LOG",
        now,
    }
}
fn tenant() -> TenantPolicy {
    TenantPolicy::freeze(
        "tenant-DO-NOT-LOG",
        "repo",
        BTreeSet::from(["caller-DO-NOT-LOG".into()]),
        BTreeSet::from(["caller-DO-NOT-LOG".into()]),
    )
    .unwrap()
}
fn retention() -> Retention {
    Retention {
        artifact_seconds: 10,
        audit_seconds: 100,
        max_entries: 32,
    }
}
struct Authority {
    producers: RefCell<BTreeMap<String, ProducerRecord>>,
    approval: RefCell<ApprovalRecord>,
}
impl AuthorityProvider for Authority {
    fn verify_producer(
        &self,
        _: &GuardRunEnvelope,
        d: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        self.producers
            .borrow()
            .get(d)
            .cloned()
            .ok_or(AuthorityError::Untrusted)
    }
    fn verify_approval(&self, r: &str) -> Result<ApprovalRecord, AuthorityError> {
        if r != "approval-DO-NOT-LOG" {
            return Err(AuthorityError::Untrusted);
        }
        Ok(self.approval.borrow().clone())
    }
}
fn register(a: &Authority, e: &GuardRunEnvelope) {
    let d = flowguard::digest(&serde_json::to_vec(e).unwrap());
    a.producers.borrow_mut().insert(
        d.clone(),
        ProducerRecord {
            principal: "fixture-producer".into(),
            producer: e.producer.clone(),
            envelope_digest: d,
            validity: Validity {
                issued_at: NOW - 1,
                expires_at: NOW + 100,
                revoked: false,
            },
        },
    );
}
fn prepared(
    store: &MemoryRunStore,
) -> (
    tempfile::TempDir,
    GateRun,
    PendingGate,
    EligibilityPolicy,
    Authority,
) {
    let (dir, b) = binding();
    let mut u = upstream(&b, guardengine::Enforcement::Review, false);
    u.envelope.approval_refs = vec!["approval-DO-NOT-LOG".into()];
    let f = frozen(&b, &u, false);
    let scope = obligation_scope(f.obligations().first().unwrap());
    let policy = common::policy(&u);
    let authority = Authority {
        producers: RefCell::new(BTreeMap::new()),
        approval: RefCell::new(ApprovalRecord {
            principal: "fixture-reviewer".into(),
            purpose: "review".into(),
            action: "commit".into(),
            binding: policy.binding.clone(),
            contract_digest: policy.contract_digest.clone(),
            validity: Validity {
                issued_at: NOW - 1,
                expires_at: NOW + 100,
                revoked: false,
            },
        }),
    };
    register(&authority, &u.envelope);
    let prepare = || {
        prepare_gate(
            &b,
            &f,
            BTreeMap::from([(scope.clone(), policy.clone())]),
            GateRequest {
                run_id: "audit-run".into(),
                action: "commit".into(),
                started_at: TIME.into(),
            },
        )
        .unwrap()
    };
    let pending = prepare();
    store.advance(&pending, 0).unwrap();
    store.reserve(&pending, "audit-request", 1).unwrap();
    let run = pending
        .evaluate(
            &[SpecialistEvidence {
                scope: &scope,
                envelope: &u.envelope,
                artifacts: u.artifacts(),
            }],
            &authority,
            NOW,
            TIME,
        )
        .unwrap();
    assert_eq!(run.envelope().decision, Some(guardengine::Decision::Allow));
    register(&authority, run.envelope());
    store.append(&run).unwrap();
    store.publish("audit-run", 1).unwrap();
    let own = EligibilityPolicy {
        binding: b.binding().clone(),
        producer: run.envelope().producer.clone(),
        required_scopes: run.envelope().coverage.required_scopes.clone(),
        contract_digest: run
            .envelope()
            .artifacts
            .contract
            .as_ref()
            .unwrap()
            .digest
            .clone(),
        action: "commit".into(),
        producer_principals: BTreeSet::from(["fixture-producer".into()]),
        approval_principals: BTreeMap::new(),
    };
    (dir, run, prepare(), own, authority)
}
#[test]
fn tenant_access_audit_minimization_retention_and_history_are_real() {
    let store = MemoryRunStore::default();
    let (_dir, run, query, policy, authority) = prepared(&store);
    let before = store.history(&query).unwrap();
    let caller = caller();
    let audit = LocalAuditStore::new(tenant(), retention(), &store).unwrap();
    audit.retain(run, access(&caller, NOW)).unwrap();
    assert!(
        audit
            .consume("audit-run", &policy, &authority, access(&caller, NOW))
            .unwrap()
            .eligible
    );
    let records = audit.records(access(&caller, NOW)).unwrap();
    assert_eq!(records.len(), 2);
    let first = serde_json::to_vec(&records).unwrap();
    let text = String::from_utf8(first.clone()).unwrap();
    for secret in [
        "token-DO-NOT-LOG",
        "caller-DO-NOT-LOG",
        "tenant-DO-NOT-LOG",
        "approval-DO-NOT-LOG",
        "fixture-producer",
        "fixture-reviewer",
    ] {
        assert!(!text.contains(secret));
    }
    caller.0.borrow_mut().tenant = "another-tenant".into();
    assert!(audit.records(access(&caller, NOW)).is_err());
    assert!(
        audit
            .consume("audit-run", &policy, &authority, access(&caller, NOW))
            .is_err()
    );
    caller.0.borrow_mut().tenant = "tenant-DO-NOT-LOG".into();
    assert!(
        audit
            .consume("audit-run", &policy, &authority, access(&caller, NOW + 10))
            .is_err()
    );
    assert!(
        audit
            .consume("audit-run", &policy, &authority, access(&caller, NOW + 9))
            .is_err()
    );
    let later = audit.records(access(&caller, NOW + 10)).unwrap();
    assert_eq!(first, serde_json::to_vec(&later[..2]).unwrap());
    assert_eq!(before, store.history(&query).unwrap());
}
#[test]
fn every_consume_refreshes_producer_approval_and_actual_artifact_availability() {
    let store = MemoryRunStore::default();
    let (_dir, run, query, policy, authority) = prepared(&store);
    let original = store.history(&query).unwrap();
    let caller = caller();
    let audit = LocalAuditStore::new(tenant(), retention(), &store).unwrap();
    audit.retain(run, access(&caller, NOW)).unwrap();
    assert!(
        audit
            .consume("audit-run", &policy, &authority, access(&caller, NOW))
            .unwrap()
            .eligible
    );
    authority.approval.borrow_mut().validity.revoked = true;
    assert!(
        audit
            .consume("audit-run", &policy, &authority, access(&caller, NOW))
            .is_err()
    );
    authority.approval.borrow_mut().validity.revoked = false;
    let old = authority.producers.borrow().clone();
    authority.producers.borrow_mut().clear();
    assert!(
        audit
            .consume("audit-run", &policy, &authority, access(&caller, NOW))
            .is_err()
    );
    *authority.producers.borrow_mut() = old;
    for record in authority.producers.borrow_mut().values_mut() {
        record.principal = "forged-producer".into();
    }
    assert!(
        audit
            .consume("audit-run", &policy, &authority, access(&caller, NOW))
            .is_err()
    );
    for record in authority.producers.borrow_mut().values_mut() {
        record.principal = "fixture-producer".into();
    }
    assert!(
        audit
            .consume("audit-run", &policy, &authority, access(&caller, NOW))
            .unwrap()
            .eligible
    );
    audit.remove("audit-run", access(&caller, NOW)).unwrap();
    assert!(
        audit
            .consume("audit-run", &policy, &authority, access(&caller, NOW))
            .is_err()
    );
    assert_eq!(original, store.history(&query).unwrap());
}
#[test]
fn artifact_removal_during_authority_callback_rejects_without_deadlock() {
    let store = MemoryRunStore::default();
    let (_dir, run, _, policy, authority) = prepared(&store);
    let caller = caller();
    let audit = LocalAuditStore::new(tenant(), retention(), &store).unwrap();
    audit.retain(run, access(&caller, NOW)).unwrap();
    struct Remove<'a> {
        audit: &'a LocalAuditStore<'a>,
        caller: &'a Caller,
        inner: &'a Authority,
        removed: std::cell::Cell<bool>,
    }
    impl AuthorityProvider for Remove<'_> {
        fn verify_producer(
            &self,
            e: &GuardRunEnvelope,
            d: &str,
        ) -> Result<ProducerRecord, AuthorityError> {
            if !self.removed.replace(true) {
                self.audit
                    .remove("audit-run", access(self.caller, NOW))
                    .unwrap();
            }
            self.inner.verify_producer(e, d)
        }
        fn verify_approval(&self, r: &str) -> Result<ApprovalRecord, AuthorityError> {
            self.inner.verify_approval(r)
        }
    }
    let racing = Remove {
        audit: &audit,
        caller: &caller,
        inner: &authority,
        removed: std::cell::Cell::new(false),
    };
    assert!(
        audit
            .consume("audit-run", &policy, &racing, access(&caller, NOW))
            .is_err()
    );
    assert!(racing.removed.get());
}

#[test]
fn final_caller_refresh_cannot_reopen_a_withdrawn_publication() {
    let store = MemoryRunStore::default();
    let (_dir, run, query, policy, authority) = prepared(&store);
    let caller = caller();
    let audit = LocalAuditStore::new(tenant(), retention(), &store).unwrap();
    audit.retain(run, access(&caller, NOW)).unwrap();
    struct Withdraw<'a> {
        inner: &'a Caller,
        store: &'a MemoryRunStore,
        query: &'a PendingGate,
        calls: std::cell::Cell<usize>,
    }
    impl CallerPort for Withdraw<'_> {
        fn resolve(&self, r: &str) -> Result<CallerRecord, AuthorityError> {
            let n = self.calls.get() + 1;
            self.calls.set(n);
            if n == 2 {
                self.store.advance(self.query, 1).unwrap();
            }
            self.inner.resolve(r)
        }
    }
    let racing = Withdraw {
        inner: &caller,
        store: &store,
        query: &query,
        calls: std::cell::Cell::new(0),
    };
    assert!(
        audit
            .consume(
                "audit-run",
                &policy,
                &authority,
                Access {
                    port: &racing,
                    reference: "token-DO-NOT-LOG",
                    now: NOW
                }
            )
            .is_err()
    );
    assert_eq!(racing.calls.get(), 2);
}

#[test]
fn explicit_roles_and_bounded_audit_capacity_never_fall_back_to_unlogged_access() {
    let store = MemoryRunStore::default();
    let (_dir, run, _, policy, authority) = prepared(&store);
    let caller = caller();
    assert!(
        LocalAuditStore::new(
            tenant(),
            Retention {
                max_entries: 1025,
                ..retention()
            },
            &store
        )
        .is_err()
    );
    let tenant = TenantPolicy::freeze(
        "tenant-DO-NOT-LOG",
        "repo",
        BTreeSet::from(["caller-DO-NOT-LOG".into()]),
        BTreeSet::from(["reader".into()]),
    )
    .unwrap();
    let audit = LocalAuditStore::new(
        tenant,
        Retention {
            max_entries: 1,
            ..retention()
        },
        &store,
    )
    .unwrap();
    audit.retain(run, access(&caller, NOW)).unwrap();
    assert!(audit.records(access(&caller, NOW)).is_err());
    assert!(
        audit
            .consume("audit-run", &policy, &authority, access(&caller, NOW))
            .is_err()
    );
    assert!(audit.remove("audit-run", access(&caller, NOW)).is_err());
    caller.0.borrow_mut().principal = "reader".into();
    assert_eq!(audit.records(access(&caller, NOW)).unwrap().len(), 1);
    assert_eq!(
        audit
            .consume("audit-run", &policy, &authority, access(&caller, NOW))
            .err(),
        Some(AuditError::Access)
    );
    caller.0.borrow_mut().validity.expires_at = NOW + 200;
    assert!(
        audit
            .records(access(&caller, NOW + 100))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn audit_visibility_deadline_overflow_refuses_unreadable_success() {
    let store = MemoryRunStore::default();
    let (_dir, run, _, policy, authority) = prepared(&store);
    let caller = caller();
    let audit = LocalAuditStore::new(
        tenant(),
        Retention {
            audit_seconds: i64::MAX - NOW,
            ..retention()
        },
        &store,
    )
    .unwrap();
    audit.retain(run, access(&caller, NOW)).unwrap();
    assert!(
        audit
            .consume("audit-run", &policy, &authority, access(&caller, NOW + 1))
            .is_err()
    );
    assert_eq!(audit.records(access(&caller, NOW + 1)).unwrap().len(), 1);
}
