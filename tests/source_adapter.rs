use flowguard::{
    input_limits::AllowedRoot,
    openspec::{SourceAuthority, read_tasks},
};
use std::fs;
#[test]
fn native_task_references_preserve_ids_lines_and_version_without_copying_body() {
    let root = std::env::temp_dir().join(format!("fg-openspec-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("openspec/changes/a")).unwrap();
    let path = "openspec/changes/a/tasks.md";
    let body = "# Plan\n- [ ] 1.1 Native task text\n- [x] 1.2 Other native text\n```md\n- [ ] 9.9 Example only\n```\n";
    fs::write(root.join(path), body).unwrap();
    let allowed = AllowedRoot::new(&root, 4096).unwrap();
    let refs = read_tasks(&allowed, path, "1.14.1", &[SourceAuthority::OpenSpec]).unwrap();
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].task_id, "1.1");
    assert_eq!(refs[0].line, 2);
    assert_eq!(refs[1].source_version, "1.14.1");
    assert_eq!(refs[0].source_digest, flowguard::digest(body.as_bytes()));
    assert!(
        !serde_json::to_string(&refs)
            .unwrap()
            .contains("Native task text")
    );
    assert_eq!(fs::read_to_string(root.join(path)).unwrap(), body);
    assert!(read_tasks(&allowed, path, "2.0.0", &[SourceAuthority::OpenSpec]).is_err());
    assert!(
        read_tasks(
            &allowed,
            path,
            "1.14.1",
            &[SourceAuthority::OpenSpec, SourceAuthority::SpecKit]
        )
        .is_err()
    );
    fs::write(root.join(path), "- [ ] 1.1 first\n- [x] 1.1 duplicate").unwrap();
    assert!(read_tasks(&allowed, path, "1.14.1", &[SourceAuthority::OpenSpec]).is_err());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn fenced_examples_require_matching_length_and_valid_closing_syntax() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("openspec/changes/a")).unwrap();
    let path = "openspec/changes/a/tasks.md";
    let allowed = AllowedRoot::new(root.path(), 4096).unwrap();
    for body in [
        "- [ ] 1.1 Real task\n````md\n```\n- [ ] 9.9 Example\n```\n````\n",
        "- [ ] 1.1 Real task\n~~~~md\n~~~\n- [ ] 9.9 Example\n~~~~\n",
        "- [ ] 1.1 Real task\n```md\n```not-a-close\n- [ ] 9.9 Example\n```\n",
        "- [ ] 1.1 Real task\n```md\n    ```\n- [ ] 9.9 Example\n   ```  \n",
        "- [ ] 1.1 Real task\n```md\n~~~\n- [ ] 9.9 Example\n````\n",
        "- [ ] 1.1 Real task\n    - [ ] 9.9 Indented example\n",
    ] {
        fs::write(root.path().join(path), body).unwrap();
        let refs = read_tasks(&allowed, path, "1.14.1", &[SourceAuthority::OpenSpec]).unwrap();
        assert_eq!(
            refs.iter().map(|r| r.task_id.as_str()).collect::<Vec<_>>(),
            vec!["1.1"],
            "body {body:?}"
        );
    }
}
