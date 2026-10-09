//! Borrowed local input budgets before Git validation, cloning or digest buffers.
use gitguard::candidate::CandidateSnapshot;
use guardengine::integration::RunBinding;
const TOTAL: usize = 64 * 1024;
const REQUIREMENTS: usize = 4096;
fn text<'a>(values: impl IntoIterator<Item = &'a str>, total: &mut usize) -> Result<(), ()> {
    for value in values {
        if value.len() > 256 {
            return Err(());
        }
        *total = total.checked_add(value.len()).ok_or(())?;
        if *total > TOTAL {
            return Err(());
        }
    }
    Ok(())
}
pub(crate) fn candidate(c: &CandidateSnapshot) -> Result<(), ()> {
    if c.requirement_ids().len() > REQUIREMENTS
        || c.allowed_paths().len() > 256
        || c.members().len() > 64
    {
        return Err(());
    }
    let mut total = 0;
    text(
        [
            c.repo_id(),
            c.task_id(),
            c.worktree_id(),
            c.candidate_oid(),
            c.base_oid(),
            c.source_snapshot_digest(),
            c.policy_digest(),
        ]
        .into_iter()
        .chain(c.merge_group_id())
        .chain(c.baseline_digest())
        .chain(c.requirement_ids().iter().map(String::as_str))
        .chain(c.members().iter().map(String::as_str)),
        &mut total,
    )?;
    if c.members().iter().any(|m| m.len() > 64) {
        return Err(());
    }
    for path in c.allowed_paths() {
        if path.len() > 4096 {
            return Err(());
        }
        total = total.checked_add(path.len()).ok_or(())?;
        if total > TOTAL {
            return Err(());
        }
    }
    // GG keeps schema/object-format strings private. Stream borrowed serialization
    // into a counting sink, never a Vec, to bound those too. This bounds escaped
    // JSON/numeric path expansion before any actual candidate hashing/validation.
    struct Sink(usize);
    impl std::io::Write for Sink {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.0 {
                return Err(std::io::ErrorKind::InvalidInput.into());
            }
            self.0 -= bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Sink(512 * 1024), c).map_err(|_| ())
}
pub(crate) fn invocation(i: &crate::context::InvocationInput) -> Result<(), ()> {
    if i.repo_candidates.len() > 1
        || i.task_candidates.len() > 1
        || i.requirement_ids.len() > REQUIREMENTS
    {
        return Err(());
    }
    text(
        [
            i.worktree_id.as_str(),
            i.candidate_oid.as_str(),
            i.base_oid.as_str(),
        ]
        .into_iter()
        .chain(i.repo_candidates.iter().map(String::as_str))
        .chain(i.task_candidates.iter().map(String::as_str))
        .chain(i.requirement_ids.iter().map(String::as_str)),
        &mut 0,
    )
}
pub(crate) fn binding(b: &RunBinding) -> Result<(), ()> {
    if b.requirement_ids.len() > REQUIREMENTS {
        return Err(());
    }
    text(
        [
            b.repo_id.as_str(),
            b.task_id.as_str(),
            b.worktree_id.as_str(),
            b.candidate_oid.as_str(),
            b.base_oid.as_str(),
            b.source_snapshot_digest.as_str(),
        ]
        .into_iter()
        .chain(b.merge_group_id.as_deref())
        .chain(b.baseline_digest.as_deref())
        .chain(b.requirement_ids.iter().map(String::as_str)),
        &mut 0,
    )
}
