use flowguard::baseline::{BaselineRef, ReleaseRef, resolve_baseline, validate_release};
use std::collections::{BTreeMap, BTreeSet};
fn baseline() -> BaselineRef {
    BaselineRef {
        version: "flowguard.workflow/v1".into(),
        repo_id: "repo".into(),
        stage: "02-architecture".into(),
        revision: "a".repeat(40),
        content_digest: flowguard::digest(b"parent"),
        policy_digest: flowguard::digest(b"policy"),
        approval_ref: "controller:record-1".into(),
        requirements: BTreeSet::from(["A".into(), "B".into()]),
    }
}
#[test]
fn inheritance_is_exact_independent_and_never_copies_accepted() {
    let b = baseline();
    let a = resolve_baseline(
        &b,
        "repo",
        "A",
        &b.revision,
        &b.content_digest,
        &b.policy_digest,
    )
    .unwrap();
    let other = resolve_baseline(
        &b,
        "repo",
        "B",
        &b.revision,
        &b.content_digest,
        &b.policy_digest,
    )
    .unwrap();
    assert_ne!(a, other);
    assert_eq!(a.parent_digest(), other.parent_digest());
    for (repo, req, rev, digest, policy) in [
        (
            "wrong",
            "A",
            b.revision.as_str(),
            b.content_digest.as_str(),
            b.policy_digest.as_str(),
        ),
        (
            "repo",
            "C",
            &b.revision,
            &b.content_digest,
            &b.policy_digest,
        ),
        ("repo", "A", "latest", &b.content_digest, &b.policy_digest),
        ("repo", "A", &b.revision, "changed", &b.policy_digest),
        ("repo", "A", &b.revision, &b.content_digest, "changed"),
    ] {
        assert!(resolve_baseline(&b, repo, req, rev, digest, policy).is_err());
    }
    let mut fake = b.clone();
    fake.approval_ref.clear();
    assert!(
        resolve_baseline(
            &fake,
            "repo",
            "A",
            &b.revision,
            &b.content_digest,
            &b.policy_digest
        )
        .is_err()
    );
}
#[test]
fn release_uses_exact_feature_09_set() {
    let refs = BTreeMap::from([
        ("A".into(), flowguard::digest(b"09A")),
        ("B".into(), flowguard::digest(b"09B")),
    ]);
    let release = ReleaseRef {
        version: "flowguard.workflow/v1".into(),
        features: refs.clone(),
    };
    assert!(validate_release(&release, &refs).is_ok());
    let mut missing = refs.clone();
    missing.remove("B");
    assert!(validate_release(&release, &missing).is_err());
    let mut changed = refs;
    changed.insert("A".into(), flowguard::digest(b"changed"));
    assert!(validate_release(&release, &changed).is_err());
}
