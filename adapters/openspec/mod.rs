//! A pinned native task-reference reader; not an OpenSpec installer or executor.
use crate::input_limits::AllowedRoot;
use serde::Serialize;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceAuthority {
    OpenSpec,
    SpecKit,
    Superpowers,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NativeTaskRef {
    pub path: String,
    pub line: usize,
    pub task_id: String,
    pub source_version: String,
    pub source_digest: String,
}
pub fn read_tasks(
    root: &AllowedRoot,
    path: &str,
    version: &str,
    authorities: &[SourceAuthority],
) -> Result<Vec<NativeTaskRef>, &'static str> {
    if version != "1.14.1" {
        return Err("unsupported OpenSpec version");
    }
    if authorities != [SourceAuthority::OpenSpec] {
        return Err("ambiguous source authority");
    }
    if !path.starts_with("openspec/changes/") || !path.ends_with("/tasks.md") {
        return Err("unsupported native task path");
    }
    let bytes = root
        .read(path)
        .map_err(|_| "native task source unreadable")?;
    let text = std::str::from_utf8(&bytes).map_err(|_| "native task source not UTF-8")?;
    let source_digest = crate::digest(&bytes);
    let mut refs = Vec::new();
    let mut ids = std::collections::BTreeSet::new();
    let mut fence: Option<(u8, usize)> = None;
    for (index, line) in text.lines().enumerate() {
        // The supported native subset uses top-level Markdown blocks. Four-space
        // indentation is a code example, not a task or a closing fence.
        let indent = line.bytes().take_while(|b| *b == b' ').count();
        if indent > 3 {
            continue;
        }
        let line = &line[indent..];
        let marker = line.as_bytes().first().copied();
        let length = marker
            .filter(|b| matches!(b, b'`' | b'~'))
            .map(|b| line.bytes().take_while(|c| *c == b).count())
            .unwrap_or(0);
        if let Some((opening_marker, opening_length)) = fence {
            if marker == Some(opening_marker)
                && length >= opening_length
                && line[length..].bytes().all(|b| matches!(b, b' ' | b'\t'))
            {
                fence = None;
            }
            continue;
        }
        if length >= 3 {
            if marker == Some(b'`') && line[length..].contains('`') {
                return Err("unsupported backtick fence info");
            }
            fence = Some((marker.expect("fence marker"), length));
            continue;
        }
        let Some(rest) = line
            .strip_prefix("- [ ] ")
            .or_else(|| line.strip_prefix("- [x] "))
            .or_else(|| line.strip_prefix("- [X] "))
        else {
            continue;
        };
        let id = rest.split_whitespace().next().ok_or("task ID missing")?;
        if !id.contains('.')
            || id
                .split('.')
                .any(|segment| segment.is_empty() || !segment.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err("unsupported native task ID");
        }
        if !ids.insert(id.to_owned()) {
            return Err("duplicate task ID");
        }
        if refs.len() >= 8192 {
            return Err("task reference budget");
        }
        refs.push(NativeTaskRef {
            path: path.into(),
            line: index + 1,
            task_id: id.into(),
            source_version: version.into(),
            source_digest: source_digest.clone(),
        });
    }
    if fence.is_some() || refs.is_empty() {
        return Err("incomplete native task source");
    }
    Ok(refs)
}
