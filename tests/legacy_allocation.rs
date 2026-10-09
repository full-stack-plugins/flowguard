use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
struct Measured;
static BIGGEST: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Measured {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        BIGGEST.fetch_max(l.size(), Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        BIGGEST.fetch_max(n, Ordering::Relaxed);
        unsafe { System.realloc(p, l, n) }
    }
}
#[global_allocator]
static ALLOC: Measured = Measured;
#[test]
fn excessive_columns_do_not_amplify_before_rejection_or_ignore() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("docs/project/02-architecture.md");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let text = format!(
        "| 阶段 | 02-architecture |\n| 阶段状态 | pending |\n{}\n",
        "|".repeat(2 * 1024 * 1024)
    );
    std::fs::write(&path, text).unwrap();
    let root = flowguard::input_limits::AllowedRoot::new(dir.path(), 16 * 1024 * 1024).unwrap();
    BIGGEST.store(0, Ordering::SeqCst);
    let result = flowguard::legacy::observe(
        &root,
        "fixture",
        "02-architecture",
        flowguard::legacy::LEGACY_REVISION,
    );
    let largest = BIGGEST.load(Ordering::SeqCst);
    eprintln!(
        "largest_single_allocation={largest}; result_ok={}",
        result.is_ok()
    );
    assert!(result.is_ok());
    assert!(
        largest <= 4 * 1024 * 1024,
        "2MiB ignored row allocated {largest} bytes before column rejection"
    );
    let text = format!(
        "| 阶段 | 02-architecture |\n| 阶段状态 | pending |\n| 批准依据 | {} |\n",
        "|".repeat(2 * 1024 * 1024)
    );
    std::fs::write(&path, text).unwrap();
    BIGGEST.store(0, Ordering::SeqCst);
    let result = flowguard::legacy::observe(
        &root,
        "fixture",
        "02-architecture",
        flowguard::legacy::LEGACY_REVISION,
    );
    let largest = BIGGEST.load(Ordering::SeqCst);
    eprintln!(
        "known-field largest_single_allocation={largest}; result_ok={}",
        result.is_ok()
    );
    assert!(result.is_err());
    assert!(largest <= 4 * 1024 * 1024);
}
