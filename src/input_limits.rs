use std::{
    fs,
    io::{self, Read},
    path::{Component, Path, PathBuf},
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    InvalidPath,
    OutsideRoot,
    TooLarge,
    Missing,
    Io,
}
/// The caller must keep the authorized checkout immutable during analysis.
#[derive(Debug)]
pub struct AllowedRoot {
    root: PathBuf,
    max_bytes: usize,
}
impl AllowedRoot {
    pub fn new(root: impl AsRef<Path>, max_bytes: usize) -> Result<Self, ReadError> {
        let root = fs::canonicalize(root).map_err(|_| ReadError::Io)?;
        if !root.is_dir() || max_bytes == 0 || max_bytes > 16 * 1024 * 1024 {
            return Err(ReadError::InvalidPath);
        }
        Ok(Self { root, max_bytes })
    }
    pub fn read(&self, relative: &str) -> Result<Vec<u8>, ReadError> {
        let path = Path::new(relative);
        if relative.is_empty()
            || relative.contains(':')
            || relative.contains('\\')
            || path
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(ReadError::InvalidPath);
        }
        let resolved = fs::canonicalize(self.root.join(path)).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                ReadError::Missing
            } else {
                ReadError::Io
            }
        })?;
        if !resolved.starts_with(&self.root) {
            return Err(ReadError::OutsideRoot);
        }
        // Inspect before open: opening a FIFO can block before fstat is possible.
        if !fs::metadata(&resolved)
            .map_err(|_| ReadError::Io)?
            .is_file()
        {
            return Err(ReadError::InvalidPath);
        }
        let file = fs::File::open(resolved).map_err(|_| ReadError::Io)?;
        let meta = file.metadata().map_err(|_| ReadError::Io)?;
        if !meta.is_file() {
            return Err(ReadError::InvalidPath);
        }
        if meta.len() > self.max_bytes as u64 {
            return Err(ReadError::TooLarge);
        }
        let mut bytes = Vec::new();
        file.take(self.max_bytes as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| ReadError::Io)?;
        if bytes.len() > self.max_bytes {
            return Err(ReadError::TooLarge);
        }
        Ok(bytes)
    }
}
