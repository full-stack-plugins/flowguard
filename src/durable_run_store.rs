//! Owner-private Linux durable attempt history. Storage never establishes authority.
//! Descriptor/lock/sync protocol adapted from reviewed GitGuard df2f756; state replay is FlowGuard's.
use crate::{
    gate::PendingGate,
    run_store::{AttemptHandle, MemoryRunStore, WorkIdentity},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};
const VERSION: &str = "flowguard.attempt-log/v1alpha1";
const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_EVENTS: usize = 10000;
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum Event {
    Advance {
        work: WorkIdentity,
        expected: u64,
    },
    Reserve {
        work: WorkIdentity,
        key: String,
        generation: u64,
    },
    Append {
        run_id: String,
        envelope: String,
    },
    Publish {
        run_id: String,
        expected: u64,
    },
}
enum EventResult {
    Generation(u64),
    Attempt(AttemptHandle),
    Unit,
}
impl Event {
    fn apply(&self, store: &MemoryRunStore) -> Result<EventResult, String> {
        match self {
            Self::Advance { work, expected } => {
                validate_work(work)?;
                store
                    .advance_work(work.clone(), *expected)
                    .map(EventResult::Generation)
            }
            Self::Reserve {
                work,
                key,
                generation,
            } => {
                validate_work(work)?;
                store
                    .reserve_work(work.clone(), key, *generation)
                    .map(EventResult::Attempt)
            }
            Self::Append { run_id, envelope } => store
                .append(run_id, envelope.as_bytes())
                .map(|_| EventResult::Unit),
            Self::Publish { run_id, expected } => {
                store.publish(run_id, *expected).map(|_| EventResult::Unit)
            }
        }
        .map_err(|_| "attempt transition rejected".into())
    }
}
fn validate_work(work: &WorkIdentity) -> Result<(), String> {
    use guardengine::integration::*;
    if !crate::valid_digest(&work.target)
        || !crate::valid_digest(&work.digest)
        || work.required.is_empty()
        || work.required.len() > 64
    {
        return Err("invalid persisted identity".into());
    }
    prepare_attempt(InvocationDraft {
        run_id: work.run_id.clone(),
        producer: Some(Producer {
            guard: "flowguard".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            analyzer_id: "flowguard.stage-gates".into(),
            analyzer_version: env!("CARGO_PKG_VERSION").into(),
        }),
        binding: Some(work.binding.clone()),
        coverage: Some(Coverage {
            status: CoverageStatus::Partial,
            required_scopes: work.required.clone(),
            observed_scopes: vec![],
            missing_scopes: work.required.clone(),
        }),
        profile: Some(EvidenceProfile::EngineBacked),
        started_at: "2026-10-09T00:00:00Z".into(),
    })
    .map_err(|_| "invalid persisted binding".to_string())?;
    Ok(())
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    api_version: String,
    store_id: String,
    events: Vec<Event>,
    checksum: String,
}
impl Snapshot {
    fn checksum(&self) -> Result<String, String> {
        serde_json::to_vec(&(&self.api_version, &self.store_id, &self.events))
            .map(|b| crate::digest(&b))
            .map_err(|_| "snapshot serialization failed".into())
    }
}
struct FileStore {
    directory: File,
    identity: String,
}
struct Lock(File);
impl Lock {
    fn acquire(file: File) -> Result<Self, String> {
        // flock is tied to this freshly opened descriptor, shared by all backend processes.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err("history busy or lock unavailable; retry observation".into());
        }
        Ok(Self(file))
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}
impl FileStore {
    fn directory(root: &Path) -> Result<File, String> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(root)
            .map_err(|_| "private history directory unavailable")?;
        let meta = file
            .metadata()
            .map_err(|_| "history metadata unavailable")?;
        if !meta.is_dir() || meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
            return Err("history directory must be owner-private".into());
        }
        // Linux local filesystems only; reject known network/pseudo filesystems and unknown types.
        let mut stat = std::mem::MaybeUninit::<libc::statfs>::uninit();
        if unsafe { libc::fstatfs(file.as_raw_fd(), stat.as_mut_ptr()) } != 0 {
            return Err("filesystem capability unavailable".into());
        }
        let kind = unsafe { stat.assume_init() }.f_type;
        if ![0xef53, 0x58465342, 0x9123683e, 0x794c7630].contains(&kind) {
            return Err("unsupported history filesystem".into());
        }
        Ok(file)
    }
    fn path(&self, name: &str) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", self.directory.as_raw_fd())).join(name)
    }
    fn checked(file: &File) -> Result<(), String> {
        let m = file
            .metadata()
            .map_err(|_| "history metadata unavailable")?;
        if !m.is_file()
            || m.uid() != unsafe { libc::geteuid() }
            || m.mode() & 0o077 != 0
            || m.nlink() != 1
        {
            return Err("unsafe history file".into());
        }
        Ok(())
    }
    fn lock(&self) -> Result<Lock, String> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(self.path("run.lock"))
            .map_err(|_| "history lock missing")?;
        Self::checked(&file)?;
        let mut id = String::new();
        (&mut file)
            .take(72)
            .read_to_string(&mut id)
            .map_err(|_| "invalid history identity")?;
        if id != self.identity {
            return Err("history identity changed".into());
        }
        Lock::acquire(file)
    }
    fn create(root: &Path) -> Result<Self, String> {
        let directory = Self::directory(root)?;
        let mut store = Self {
            directory,
            identity: String::new(),
        };
        if std::fs::read_dir(store.path("."))
            .map_err(|_| "history directory unavailable")?
            .next()
            .is_some()
        {
            return Err("history initialization requires an empty directory".into());
        }
        let nonce = tempfile::NamedTempFile::new_in(store.path("."))
            .map_err(|_| "history identity unavailable")?;
        store.identity = crate::digest(nonce.path().as_os_str().as_encoded_bytes());
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(store.path("run.lock"))
            .map_err(|_| "history already initialized or unavailable")?;
        let _lock = Lock::acquire(file.try_clone().map_err(|_| "lock unavailable")?)?;
        file.write_all(store.identity.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|_| "history initialization outcome uncertain")?;
        store
            .directory
            .sync_all()
            .map_err(|_| "history initialization outcome uncertain")?;
        let snapshot = Snapshot {
            api_version: VERSION.into(),
            store_id: store.identity.clone(),
            events: vec![],
            checksum: String::new(),
        };
        store.save(snapshot)?;
        Ok(store)
    }
    fn open(root: &Path) -> Result<Self, String> {
        let mut store = Self {
            directory: Self::directory(root)?,
            identity: String::new(),
        };
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(store.path("run.lock"))
            .map_err(|_| "history not initialized")?;
        Self::checked(&file)?;
        (&mut file)
            .take(72)
            .read_to_string(&mut store.identity)
            .map_err(|_| "invalid history identity")?;
        if !crate::valid_digest(&store.identity) {
            return Err("invalid history identity".into());
        }
        let _lock = store.lock()?;
        store.load()?;
        Ok(store)
    }
    fn load(&self) -> Result<(Snapshot, MemoryRunStore), String> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(self.path("state.json"))
            .map_err(|_| "history state missing; recovery required")?;
        Self::checked(&file)?;
        let mut bytes = vec![];
        file.take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "history state unavailable")?;
        if bytes.len() > MAX_BYTES {
            return Err("history capacity exceeded".into());
        }
        let snapshot: Snapshot =
            serde_json::from_slice(&bytes).map_err(|_| "history corrupt; recovery required")?;
        if snapshot.api_version != VERSION
            || snapshot.store_id != self.identity
            || snapshot.events.len() > MAX_EVENTS
            || snapshot.checksum != snapshot.checksum()?
        {
            return Err("history integrity failure; recovery required".into());
        }
        let store = MemoryRunStore::default();
        for event in &snapshot.events {
            event.apply(&store)?;
        }
        Ok((snapshot, store))
    }
    fn save(&self, mut snapshot: Snapshot) -> Result<(), String> {
        snapshot.checksum = snapshot.checksum()?;
        let bytes = serde_json::to_vec(&snapshot).map_err(|_| "history serialization failed")?;
        if bytes.len() > MAX_BYTES || snapshot.events.len() > MAX_EVENTS {
            return Err("history capacity exceeded".into());
        }
        let mut tmp = tempfile::NamedTempFile::new_in(self.path("."))
            .map_err(|_| "history staging unavailable")?;
        tmp.write_all(&bytes)
            .and_then(|_| tmp.as_file().sync_all())
            .map_err(|_| "history staging failed")?;
        #[cfg(test)]
        if std::env::var("FG_DURABLE_TEST_CRASH_POINT").as_deref() == Ok("before-rename") {
            std::process::exit(71);
        }
        tmp.persist(self.path("state.json"))
            .map_err(|_| "history publication outcome uncertain; reopen and inspect")?;
        #[cfg(test)]
        if std::env::var("FG_DURABLE_TEST_CRASH_POINT").as_deref() == Ok("after-rename") {
            std::process::exit(72);
        }
        self.directory
            .sync_all()
            .map_err(|_| "history publication outcome uncertain; reopen and inspect")?;
        Ok(())
    }
    fn mutate(&self, event: Event) -> Result<EventResult, String> {
        if serde_json::to_vec(&event)
            .map_err(|_| "event serialization failed")?
            .len()
            > 3 * 1024 * 1024
        {
            return Err("event too large".into());
        }
        let _lock = self.lock()?;
        let (mut snapshot, store) = self.load()?;
        let result = event.apply(&store)?;
        snapshot.events.push(event);
        self.save(snapshot)?;
        Ok(result)
    }
}
/// Explicit opt-in storage; no persistent writes occur in ordinary gate checks.
pub struct DurableRunStore {
    store: FileStore,
}
impl DurableRunStore {
    pub fn create(root: &Path) -> Result<Self, String> {
        FileStore::create(root).map(|store| Self { store })
    }
    pub fn open(root: &Path) -> Result<Self, String> {
        FileStore::open(root).map(|store| Self { store })
    }
    pub fn advance(&self, gate: &PendingGate, expected: u64) -> Result<u64, String> {
        match self.store.mutate(Event::Advance {
            work: gate.store_identity(),
            expected,
        })? {
            EventResult::Generation(g) => Ok(g),
            _ => Err("invalid advance result".into()),
        }
    }
    /// Reobserve a current generation after an uncertain write; this is not authorization.
    pub fn generation(&self, gate: &PendingGate) -> Result<Option<u64>, String> {
        let _lock = self.store.lock()?;
        self.store
            .load()?
            .1
            .generation_work(&gate.store_identity())
            .map_err(|_| "generation unavailable".into())
    }
    pub fn reserve(
        &self,
        gate: &PendingGate,
        key: &str,
        generation: u64,
    ) -> Result<AttemptHandle, String> {
        match self.store.mutate(Event::Reserve {
            work: gate.store_identity(),
            key: key.into(),
            generation,
        })? {
            EventResult::Attempt(a) => Ok(a),
            _ => Err("invalid reserve result".into()),
        }
    }
    pub fn append(&self, run_id: &str, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() > 1024 * 1024 {
            return Err("envelope budget".into());
        }
        let envelope = std::str::from_utf8(bytes)
            .map_err(|_| "invalid envelope encoding")?
            .to_owned();
        self.store
            .mutate(Event::Append {
                run_id: run_id.into(),
                envelope,
            })
            .map(|_| ())
    }
    pub fn publish(&self, run_id: &str, expected: u64) -> Result<(), String> {
        self.store
            .mutate(Event::Publish {
                run_id: run_id.into(),
                expected,
            })
            .map(|_| ())
    }
    pub fn current(&self, gate: &PendingGate) -> Result<Option<Vec<u8>>, String> {
        let _lock = self.store.lock()?;
        self.store
            .load()?
            .1
            .current(gate)
            .map_err(|_| "current unavailable".into())
    }
    pub fn history(&self, gate: &PendingGate) -> Result<Vec<Vec<u8>>, String> {
        let _lock = self.store.lock()?;
        self.store
            .load()?
            .1
            .history(gate)
            .map_err(|_| "history unavailable".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    fn work(run: &str) -> WorkIdentity {
        let envelope = guardengine::integration::load_envelope_json(
            include_bytes!("../fixtures/gate_mapping/envelope.json"),
            guardengine::integration::EvidenceProfile::EngineBacked,
        )
        .unwrap();
        WorkIdentity {
            target: crate::digest(b"fixture-target"),
            digest: crate::digest(b"fixture-work"),
            run_id: run.into(),
            binding: envelope.binding,
            required: envelope.coverage.required_scopes,
        }
    }
    fn advance(store: &FileStore, expected: u64, run: &str) -> Result<u64, String> {
        match store.mutate(Event::Advance {
            work: work(run),
            expected,
        })? {
            EventResult::Generation(g) => Ok(g),
            _ => Err("wrong result".into()),
        }
    }
    #[test]
    fn process_worker() {
        let Ok(root) = std::env::var("FG_DURABLE_TEST_ROOT") else {
            return;
        };
        let run = std::env::var("FG_DURABLE_TEST_RUN").unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let store = loop {
            match FileStore::open(Path::new(&root)) {
                Ok(store) => break store,
                Err(e) if e.starts_with("history busy") && std::time::Instant::now() < deadline => {
                    std::thread::sleep(std::time::Duration::from_millis(5))
                }
                Err(e) => panic!("{e}"),
            }
        };
        if let Ok(gate) = std::env::var("FG_DURABLE_TEST_GATE") {
            std::fs::write(format!("{gate}.{run}.ready"), b"ready").unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while !Path::new(&gate).exists() {
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        let won = advance(&store, 0, &run).is_ok();
        if let Ok(output) = std::env::var("FG_DURABLE_TEST_OUTPUT") {
            std::fs::write(output, if won { "won" } else { "lost" }).unwrap();
        }
    }
    fn worker(root: &Path, run: &str) -> std::process::Command {
        let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
        cmd.args([
            "--exact",
            "durable_run_store::tests::process_worker",
            "--nocapture",
        ])
        .env("FG_DURABLE_TEST_ROOT", root)
        .env("FG_DURABLE_TEST_RUN", run)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
        cmd
    }
    #[test]
    fn independent_processes_have_exactly_one_cas_winner() {
        let dir = tempfile::Builder::new()
            .prefix(".fg-durable-test-")
            .tempdir_in(env!("CARGO_MANIFEST_DIR"))
            .unwrap();
        let root = dir.path().join("store");
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        FileStore::create(&root).unwrap();
        let gate = dir.path().join("gate");
        let mut children = vec![];
        for run in ["a", "b"] {
            children.push(
                worker(&root, run)
                    .env("FG_DURABLE_TEST_GATE", &gate)
                    .env("FG_DURABLE_TEST_OUTPUT", dir.path().join(run))
                    .spawn()
                    .unwrap(),
            );
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !["a", "b"]
            .iter()
            .all(|run| dir.path().join(format!("gate.{run}.ready")).exists())
        {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        std::fs::write(&gate, b"go").unwrap();
        for child in children {
            let output = child.wait_with_output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        assert_eq!(
            ["a", "b"]
                .iter()
                .filter(|r| std::fs::read(dir.path().join(r)).unwrap() == b"won")
                .count(),
            1
        );
        let reopened = FileStore::open(&root).unwrap();
        assert!(advance(&reopened, 0, "stale").is_err());
        assert_eq!(advance(&reopened, 1, "next").unwrap(), 2);
    }
    #[test]
    fn process_crash_around_atomic_replace_preserves_whole_old_or_new_state() {
        for (point, exit, expected) in [("before-rename", 71, 0), ("after-rename", 72, 1)] {
            let dir = tempfile::Builder::new()
                .prefix(".fg-durable-test-")
                .tempdir_in(env!("CARGO_MANIFEST_DIR"))
                .unwrap();
            FileStore::create(dir.path()).unwrap();
            let status = worker(dir.path(), "interrupted")
                .env("FG_DURABLE_TEST_CRASH_POINT", point)
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(exit));
            let reopened = FileStore::open(dir.path()).unwrap();
            assert_eq!(advance(&reopened, expected, "next").unwrap(), expected + 1);
        }
    }

    #[test]
    fn corruption_and_recomputed_checksum_invalid_cas_fail_closed() {
        let dir = tempfile::tempdir_in(env!("CARGO_MANIFEST_DIR")).unwrap();
        let store = FileStore::create(dir.path()).unwrap();
        advance(&store, 0, "one").unwrap();
        let path = dir.path().join("state.json");
        let original = std::fs::read(&path).unwrap();
        for case in 0..3 {
            let mut snapshot: Snapshot = serde_json::from_slice(&original).unwrap();
            match case {
                0 => snapshot.api_version = "unknown".into(),
                1 => {
                    if let Event::Advance { expected, .. } = &mut snapshot.events[0] {
                        *expected = 9;
                    }
                }
                _ => snapshot.checksum = "bad".into(),
            }
            if case != 2 {
                snapshot.checksum = snapshot.checksum().unwrap();
            }
            std::fs::write(&path, serde_json::to_vec(&snapshot).unwrap()).unwrap();
            assert!(FileStore::open(dir.path()).is_err());
        }
        std::fs::remove_file(&path).unwrap();
        assert!(FileStore::open(dir.path()).is_err());
        std::os::unix::fs::symlink("/dev/null", &path).unwrap();
        assert!(FileStore::open(dir.path()).is_err());
    }
}
