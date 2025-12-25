use std::path::Path;
use std::process::Command;

use crate::backend::traits::{Backend, ChecksumAlgorithm, CopyOptions, FileInfo};
use crate::backend::error::BackendError;
use crate::copy::CopyStats;
use crate::protocol::RemotePath;

pub struct SshBackend {
    remote_path: RemotePath,
}

impl SshBackend {
    pub fn new(remote_path: RemotePath) -> Self {
        Self { remote_path }
    }

    fn host(&self) -> Result<&str, BackendError> {
        self.remote_path
            .url
            .host_str()
            .ok_or_else(|| BackendError::ConnectionError("No host specified".to_string()))
    }

    fn port(&self) -> u16 {
        self.remote_path.url.port().unwrap_or(22)
    }

    fn username(&self) -> &str {
        self.remote_path.url.username()
    }

    fn remote_spec(&self, path: &str) -> String {
        format!("{}@{}:{}", self.username(), self.host().unwrap_or(""), path)
    }
}

impl Backend for SshBackend {
    fn name(&self) -> &str {
        "ssh"
    }

    fn copy_file(
        &self,
        src: &str,
        dst: &str,
        opts: &CopyOptions,
    ) -> Result<u64, BackendError> {
        let mut cmd = Command::new("scp");

        if self.port() != 22 {
            cmd.arg("-P").arg(self.port().to_string());
        }

        if opts.progress {
            cmd.arg("-v");
        } else if !opts.verbose {
            cmd.arg("-q");
        }

        for opt in &opts.ssh_opts {
            cmd.arg("-o").arg(opt);
        }

        if src.starts_with("ssh://") || src.contains('@') {
            cmd.arg(src);
        } else {
            let remote_spec = self.remote_spec(src);
            cmd.arg(&remote_spec);
        }

        if dst.starts_with("ssh://") || dst.contains('@') {
            cmd.arg(dst);
        } else {
            let dst_path = Path::new(dst);
            if let Some(parent) = dst_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| BackendError::IoError {
                    message: format!("Failed to create directory: {}", parent.display()),
                    error: e.to_string(),
                })?;
            }
            cmd.arg(dst);
        }

        if opts.verbose {
            println!("Executing: {:?}", cmd);
        }

        let status = cmd.status().map_err(|e| BackendError::IoError {
            message: "Failed to execute scp".to_string(),
            error: e.to_string(),
        })?;

        if !status.success() {
            return Err(BackendError::IoError {
                message: "scp failed".to_string(),
                error: format!("Exit code: {}", status.code().unwrap_or(-1)),
            });
        }

        let size = if dst.starts_with("ssh://") || dst.contains('@') {
            std::fs::metadata(src)
                .map(|m| m.len())
                .unwrap_or(0)
        } else {
            std::fs::metadata(dst)
                .map(|m| m.len())
                .unwrap_or(0)
        };

        Ok(size)
    }

    fn copy_directory(
        &self,
        src: &str,
        dst: &str,
        opts: &CopyOptions,
    ) -> Result<CopyStats, BackendError> {
        if !opts.recursive {
            return Err(BackendError::UnsupportedOperation(
                "Recursive copy requires -r flag".to_string(),
            ));
        }

        let mut cmd = Command::new("scp");
        cmd.arg("-r");

        if self.port() != 22 {
            cmd.arg("-P").arg(self.port().to_string());
        }

        if opts.progress {
            cmd.arg("-v");
        } else if !opts.verbose {
            cmd.arg("-q");
        }

        for opt in &opts.ssh_opts {
            cmd.arg("-o").arg(opt);
        }

        if src.starts_with("ssh://") || src.contains('@') {
            cmd.arg(src);
        } else {
            let remote_spec = self.remote_spec(src);
            cmd.arg(&remote_spec);
        }

        if dst.starts_with("ssh://") || dst.contains('@') {
            cmd.arg(dst);
        } else {
            let dst_path = Path::new(dst);
            if let Some(parent) = dst_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| BackendError::IoError {
                    message: format!("Failed to create directory: {}", parent.display()),
                    error: e.to_string(),
                })?;
            }
            cmd.arg(dst);
        }

        if opts.verbose {
            println!("Executing: {:?}", cmd);
        }

        let status = cmd.status().map_err(|e| BackendError::IoError {
            message: "Failed to execute scp".to_string(),
            error: e.to_string(),
        })?;

        if !status.success() {
            return Err(BackendError::IoError {
                message: "scp failed".to_string(),
                error: format!("Exit code: {}", status.code().unwrap_or(-1)),
            });
        }

        let stats = if opts.verbose || opts.progress {
            CopyStats::new()
        } else {
            CopyStats::new_minimal()
        };

        Ok(stats)
    }

    fn list(&self, path: &str) -> Result<Vec<FileInfo>, BackendError> {
        let host = self.host()?;

        let mut cmd = Command::new("ssh");
        if self.port() != 22 {
            cmd.arg("-p").arg(self.port().to_string());
        }

        for opt in &[] as &[String] {
            cmd.arg("-o").arg(opt);
        }

        cmd.arg(format!("{}@{}", self.username(), host));
        cmd.arg(format!("ls -la {}", path));

        let output = cmd.output().map_err(|e| BackendError::IoError {
            message: "Failed to execute ssh".to_string(),
            error: e.to_string(),
        })?;

        if !output.status.success() {
            return Err(BackendError::IoError {
                message: "ssh ls failed".to_string(),
                error: format!("Exit code: {}", output.status.code().unwrap_or(-1)),
            });
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut files = Vec::new();

        for line in output_str.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 9 {
                let is_dir = parts[0].starts_with('d');
                let size: u64 = parts[4].parse().unwrap_or(0);
                let name = parts[8..].join(" ");

                files.push(FileInfo {
                    path: format!("{}/{}", path.trim_end_matches('/'), name),
                    size,
                    is_dir,
                    modified: None,
                });
            }
        }

        Ok(files)
    }

    fn delete(&self, path: &str) -> Result<(), BackendError> {
        let host = self.host()?;

        let mut cmd = Command::new("ssh");
        if self.port() != 22 {
            cmd.arg("-p").arg(self.port().to_string());
        }

        cmd.arg(format!("{}@{}", self.username(), host));
        cmd.arg(format!("rm -rf {}", path));

        let status = cmd.status().map_err(|e| BackendError::IoError {
            message: "Failed to execute ssh".to_string(),
            error: e.to_string(),
        })?;

        if !status.success() {
            return Err(BackendError::IoError {
                message: "ssh rm failed".to_string(),
                error: format!("Exit code: {}", status.code().unwrap_or(-1)),
            });
        }

        Ok(())
    }

    fn checksum(
        &self,
        path: &str,
        algorithm: ChecksumAlgorithm,
    ) -> Result<String, BackendError> {
        let host = self.host()?;
        let algo_cmd = match algorithm {
            ChecksumAlgorithm::Md5 => "md5sum",
            ChecksumAlgorithm::Sha1 => "sha1sum",
            ChecksumAlgorithm::Sha256 => "sha256sum",
        };

        let mut cmd = Command::new("ssh");
        if self.port() != 22 {
            cmd.arg("-p").arg(self.port().to_string());
        }

        cmd.arg(format!("{}@{}", self.username(), host));
        cmd.arg(format!("{} {}", algo_cmd, path));

        let output = cmd.output().map_err(|e| BackendError::IoError {
            message: "Failed to execute ssh".to_string(),
            error: e.to_string(),
        })?;

        if !output.status.success() {
            return Err(BackendError::IoError {
                message: format!("ssh {} failed", algo_cmd),
                error: format!("Exit code: {}", output.status.code().unwrap_or(-1)),
            });
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let hash = output_str
            .split_whitespace()
            .next()
            .ok_or_else(|| BackendError::Other("Failed to parse checksum".to_string()))?
            .to_string();

        Ok(hash)
    }
}

