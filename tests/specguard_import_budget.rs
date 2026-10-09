use flowguard::specguard_adapter::{ExpectedExport, import_fixture};
use specguard::model::Identity;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    collections::BTreeSet,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};
struct Count;
static ACTIVE: AtomicBool = AtomicBool::new(false);
static MAX: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Count {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if ACTIVE.load(Ordering::Relaxed) {
            MAX.fetch_max(l.size(), Ordering::Relaxed);
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if ACTIVE.load(Ordering::Relaxed) {
            MAX.fetch_max(n, Ordering::Relaxed);
        }
        unsafe { System.realloc(p, l, n) }
    }
}
#[global_allocator]
static ALLOCATOR: Count = Count;
#[test]
fn oversized_expected_scope_rejects_without_clone_hash_or_parse_allocation() {
    let e = ExpectedExport {
        baseline_digest: format!("sha256:{}", "a".repeat(64)),
        source_digest: format!("sha256:{}", "b".repeat(64)),
        candidate_oid: "c".repeat(40),
        scope: BTreeSet::from([Identity {
            namespace: "n".repeat(17 * 1024 * 1024),
            id: "R".into(),
        }]),
    };
    ACTIVE.store(true, Ordering::Relaxed);
    let result = import_fixture(b"{}", &e);
    ACTIVE.store(false, Ordering::Relaxed);
    assert!(result.is_err());
    assert_eq!(MAX.load(Ordering::Relaxed), 0);
}
