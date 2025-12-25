use std::path::Path;
use std::process::Command;

use crate::backend::traits::{Backend, ChecksumAlgorithm, CopyOptions, FileInfo};
use crate::backend::error::BackendError;
use crate::copy::CopyStats;
use crate::protocol::RemotePath;

pub struct HttpBackend {
    remote_path: RemotePath,
}

impl HttpBackend {
    pub fn new(remote_path: RemotePath) -> Self {
        Self { remote_path }
    }

    fn url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            format!("{}://{}{}", self.remote_path.url.scheme(), self.remote_path.url.host_str().unwrap_or(""), path)
        }
    }

    fn try_curl(
        &self,
        url: &str,
        dst_path: &Path,
        verbose: bool,
        progress: bool,
    ) -> Result<Command, BackendError> {
        let check = Command::new("curl").arg("--version").output();

        if check.is_err() {
            return Err(BackendError::Other("curl not found in PATH".to_string()));
        }

        let mut cmd = Command::new("curl");
        cmd.arg("-L").arg("-f").arg("-o").arg(dst_path).arg(url);

        if progress {
            cmd.arg("--progress-bar");
        } else if !verbose {
            cmd.arg("-s");
        }

        Ok(cmd)
    }

    fn try_wget(
        &self,
        url: &str,
        dst_path: &Path,
        verbose: bool,
        progress: bool,
    ) -> Result<Command, BackendError> {
        let check = Command::new("wget").arg("--version").output();

        if check.is_err() {
            return Err(BackendError::Other("wget not found in PATH".to_string()));
        }

        let mut cmd = Command::new("wget");
        cmd.arg("-O").arg(dst_path).arg(url);

        if progress {
            cmd.arg("--progress=bar");
        } else if !verbose {
            cmd.arg("-q");
        }

        Ok(cmd)
    }
}

impl Backend for HttpBackend {
    fn name(&self) -> &str {
        "http"
    }

    fn copy_file(
        &self,
        src: &str,
        dst: &str,
        opts: &CopyOptions,
    ) -> Result<u64, BackendError> {
        let url = if src.starts_with("http://") || src.starts_with("https://") {
            src.to_string()
        } else {
            self.url(src)
        };

        let dst_path = Path::new(dst);
        if let Some(parent) = dst_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| BackendError::IoError {
                message: format!("Failed to create directory: {}", parent.display()),
                error: e.to_string(),
            })?;
        }

        let mut cmd = if let Ok(cmd) = self.try_curl(&url, dst_path, opts.verbose, opts.progress) {
            cmd
        } else if let Ok(cmd) = self.try_wget(&url, dst_path, opts.verbose, opts.progress) {
            cmd
        } else {
            return Err(BackendError::Other(
                "Neither curl nor wget found in PATH".to_string(),
            ));
        };

        if opts.verbose {
            println!("Executing: {:?}", cmd);
        }

        let status = cmd.status().map_err(|e| BackendError::IoError {
            message: "Failed to execute download command".to_string(),
            error: e.to_string(),
        })?;

        if !status.success() {
            return Err(BackendError::IoError {
                message: "Download failed".to_string(),
                error: format!("Exit code: {}", status.code().unwrap_or(-1)),
            });
        }

        let size = std::fs::metadata(dst_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(size)
    }

    fn copy_directory(
        &self,
        _src: &str,
        _dst: &str,
        _opts: &CopyOptions,
    ) -> Result<CopyStats, BackendError> {
        Err(BackendError::UnsupportedOperation(
            "HTTP/HTTPS does not support directory operations".to_string(),
        ))
    }

    fn list(&self, _path: &str) -> Result<Vec<FileInfo>, BackendError> {
        Err(BackendError::UnsupportedOperation(
            "HTTP/HTTPS does not support listing".to_string(),
        ))
    }

    fn delete(&self, _path: &str) -> Result<(), BackendError> {
        Err(BackendError::UnsupportedOperation(
            "HTTP/HTTPS does not support delete operations".to_string(),
        ))
    }

    fn checksum(
        &self,
        path: &str,
        algorithm: ChecksumAlgorithm,
    ) -> Result<String, BackendError> {
        let url = if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            self.url(path)
        };

        let temp_file = tempfile::NamedTempFile::new().map_err(|e| BackendError::IoError {
            message: "Failed to create temp file".to_string(),
            error: e.to_string(),
        })?;

        let temp_path = temp_file.path();

        let mut cmd = if let Ok(cmd) = self.try_curl(&url, temp_path, false, false) {
            cmd
        } else if let Ok(cmd) = self.try_wget(&url, temp_path, false, false) {
            cmd
        } else {
            return Err(BackendError::Other(
                "Neither curl nor wget found in PATH".to_string(),
            ));
        };

        let status = cmd.status().map_err(|e| BackendError::IoError {
            message: "Failed to download file for checksum".to_string(),
            error: e.to_string(),
        })?;

        if !status.success() {
            return Err(BackendError::IoError {
                message: "Download failed for checksum".to_string(),
                error: format!("Exit code: {}", status.code().unwrap_or(-1)),
            });
        }

        use std::io::Read;
        let mut file = std::fs::File::open(temp_path).map_err(|e| BackendError::IoError {
            message: "Failed to open temp file".to_string(),
            error: e.to_string(),
        })?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).map_err(|e| BackendError::IoError {
            message: "Failed to read temp file".to_string(),
            error: e.to_string(),
        })?;

        let hash = match algorithm {
            ChecksumAlgorithm::Md5 => {
                use md5::Context;
                let mut context = Context::new();
                context.consume(&buffer);
                format!("{:x}", context.compute())
            }
            ChecksumAlgorithm::Sha1 => {
                use sha1::Sha1;
                use digest::Digest;
                let mut hasher = Sha1::new();
                hasher.update(&buffer);
                format!("{:x}", hasher.finalize())
            }
            ChecksumAlgorithm::Sha256 => {
                use sha2::Sha256;
                use digest::Digest;
                let mut hasher = Sha256::new();
                hasher.update(&buffer);
                format!("{:x}", hasher.finalize())
            }
        };

        Ok(hash)
    }
}

