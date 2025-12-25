use crate::copy::CopyStats;
use crate::backend::error::BackendError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumAlgorithm {
    Md5,
    Sha1,
    Sha256,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub modified: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct CopyOptions {
    pub verbose: bool,
    pub progress: bool,
    pub use_ram: bool,
    pub recursive: bool,
    pub ssh_opts: Vec<String>,
    pub dry_run: bool,
    pub filters: Option<Box<dyn crate::filters::Filter>>,
}

impl Default for CopyOptions {
    fn default() -> Self {
        Self {
            verbose: false,
            progress: false,
            use_ram: false,
            recursive: false,
            ssh_opts: Vec::new(),
            dry_run: false,
            filters: None,
        }
    }
}

pub trait Backend: Send + Sync {
    fn name(&self) -> &str;

    fn copy_file(
        &self,
        src: &str,
        dst: &str,
        opts: &CopyOptions,
    ) -> Result<u64, BackendError>;

    fn copy_directory(
        &self,
        src: &str,
        dst: &str,
        opts: &CopyOptions,
    ) -> Result<CopyStats, BackendError>;

    fn list(&self, path: &str) -> Result<Vec<FileInfo>, BackendError>;

    fn delete(&self, path: &str) -> Result<(), BackendError>;

    fn checksum(
        &self,
        path: &str,
        algorithm: ChecksumAlgorithm,
    ) -> Result<String, BackendError>;

    fn exists(&self, path: &str) -> Result<bool, BackendError> {
        match self.list(path) {
            Ok(_) => Ok(true),
            Err(BackendError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }
}

