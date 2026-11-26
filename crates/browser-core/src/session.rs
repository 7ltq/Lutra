use std::io;
use std::path::Path;
use tempfile::{Builder, TempDir};

pub struct SessionDirectory {
    directory: TempDir,
}

impl SessionDirectory {
    pub fn create() -> io::Result<Self> {
        let directory = Builder::new().prefix("lutra-session-").tempdir()?;
        Ok(Self { directory })
    }

    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    pub fn close(self) -> io::Result<()> {
        self.directory.close()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn sessions_do_not_reuse_directories() {
        let first = SessionDirectory::create().unwrap();
        let second = SessionDirectory::create().unwrap();
        assert_ne!(first.path(), second.path());
        fs::write(first.path().join("test"), b"session").unwrap();
        assert!(!second.path().join("test").exists());
    }

    #[test]
    fn normal_close_removes_session_files() {
        let session = SessionDirectory::create().unwrap();
        let path = session.path().to_path_buf();
        fs::write(path.join("test"), b"session").unwrap();
        session.close().unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn dropping_session_removes_directory() {
        let path = {
            let session = SessionDirectory::create().unwrap();
            session.path().to_path_buf()
        };
        assert!(!path.exists());
    }
}
