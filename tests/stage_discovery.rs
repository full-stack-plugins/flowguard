use flowguard::{
    input_limits::AllowedRoot,
    stage::{STAGES, SourceStatus, discover},
};
use std::{fs, path::PathBuf};
fn root(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("fg-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}
#[test]
fn discovers_all_ten_paths_hashes_and_ownership_without_writes() {
    let p = root("ten");
    for (id, project) in STAGES {
        let path = p.join(if project {
            format!("docs/project/{id}.md")
        } else {
            format!("docs/features/a/{id}.md")
        });
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "native document\n").unwrap();
    }
    fn tree(path: &std::path::Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
        let mut result = std::collections::BTreeMap::new();
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                result.extend(tree(&path));
            } else {
                result.insert(path.clone(), fs::read(path).unwrap());
            }
        }
        result
    }
    let before = tree(&p);
    let allowed = AllowedRoot::new(&p, 1024).unwrap();
    let first = discover(&allowed, "a", "flowguard.docs/v1").unwrap();
    assert_eq!(first.sources.len(), 10);
    for (source, (id, project)) in first.sources.iter().zip(STAGES) {
        assert_eq!(source.stage, id);
        assert_eq!(source.owner, if project { "project" } else { "a" });
        assert_eq!(source.status, SourceStatus::Read);
        assert_eq!(source.digest.as_ref().unwrap().len(), 71);
    }
    assert_eq!(first, discover(&allowed, "a", "flowguard.docs/v1").unwrap());
    assert!(!p.join(".flowguard").exists());
    assert_eq!(before, tree(&p));
    fs::remove_file(p.join("docs/project/07-standards.md")).unwrap();
    let missing = discover(&allowed, "a", "flowguard.docs/v1").unwrap();
    assert!(!missing.complete());
    assert_eq!(missing.sources[6].status, SourceStatus::Missing);
    assert!(discover(&allowed, "a", "v999").is_err());
    assert!(discover(&allowed, "../outside", "flowguard.docs/v1").is_err());
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn oversized_source_is_terminal_not_complete() {
    let p = root("large");
    fs::create_dir_all(p.join("docs/project")).unwrap();
    fs::write(p.join("docs/project/02-architecture.md"), "x".repeat(101)).unwrap();
    let i = discover(
        &AllowedRoot::new(&p, 100).unwrap(),
        "a",
        "flowguard.docs/v1",
    )
    .unwrap();
    assert_eq!(i.sources[1].status, SourceStatus::Rejected);
    assert!(!i.complete());
    fs::remove_dir_all(p).unwrap();
}
