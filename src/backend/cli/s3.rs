use std::path::Path;
use std::process::Command;

use crate::backend::traits::{Backend, ChecksumAlgorithm, CopyOptions, FileInfo};
use crate::backend::error::BackendError;
use crate::copy::CopyStats;
use crate::protocol::RemotePath;

pub struct S3Backend {
    remote_path: RemotePath,
}

impl S3Backend {
    pub fn new(remote_path: RemotePath) -> Self {
        Self { remote_path }
    }

    fn s3_url(&self, path: &str) -> String {
        if path.starts_with("s3://") {
            path.to_string()
        } else {
            format!("s3://{}{}", self.remote_path.url.host_str().unwrap_or(""), path)
        }
    }

    fn try_aws_cli(
        &self,
        src: &str,
        dst: Option<&Path>,
        verbose: bool,
        progress: bool,
        is_download: bool,
    ) -> Result<Command, BackendError> {
        let mut cmd = Command::new("aws");
        cmd.arg("s3").arg("cp");

        if let Some(endpoint) = std::env::var("AWS_ENDPOINT_URL_S3")
            .or_else(|_| std::env::var("AWS_ENDPOINT_URL"))
            .ok()
        {
            cmd.env("AWS_ENDPOINT_URL_S3", &endpoint);
        }

        if verbose {
            cmd.arg("--debug");
        }

        if progress {
            cmd.arg("--no-progress");
        }

        if is_download {
            cmd.arg(&self.s3_url(src));
            if let Some(dst) = dst {
                cmd.arg(dst);
            }
        } else {
            if let Some(src_path) = dst {
                cmd.arg(src_path);
            }
            cmd.arg(&self.s3_url(src));
        }

        Ok(cmd)
    }

    fn try_aws_cli_sync(
        &self,
        src: &Path,
        dst: &str,
        verbose: bool,
        progress: bool,
    ) -> Result<Command, BackendError> {
        let mut cmd = Command::new("aws");
        cmd.arg("s3").arg("sync");

        if let Some(endpoint) = std::env::var("AWS_ENDPOINT_URL_S3")
            .or_else(|_| std::env::var("AWS_ENDPOINT_URL"))
            .ok()
        {
            cmd.env("AWS_ENDPOINT_URL_S3", &endpoint);
        }

        if verbose {
            cmd.arg("--debug");
        }

        if progress {
            cmd.arg("--no-progress");
        }

        cmd.arg(src);
        cmd.arg(&self.s3_url(dst));

        Ok(cmd)
    }
}

impl Backend for S3Backend {
    fn name(&self) -> &str {
        "s3"
    }

    fn copy_file(
        &self,
        src: &str,
        dst: &str,
        opts: &CopyOptions,
    ) -> Result<u64, BackendError> {
        let is_download = !src.starts_with("s3://") && dst.starts_with("s3://");
        let is_upload = src.starts_with("s3://") && !dst.starts_with("s3://");

        let mut cmd = if is_download {
            let dst_path = Path::new(dst);
            if let Some(parent) = dst_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| BackendError::IoError {
                    message: format!("Failed to create directory: {}", parent.display()),
                    error: e.to_string(),
                })?;
            }
            self.try_aws_cli(src, Some(dst_path), opts.verbose, opts.progress, true)?
        } else if is_upload {
            let src_path = Path::new(src);
            if !src_path.exists() {
                return Err(BackendError::NotFound(src.to_string()));
            }
            self.try_aws_cli(dst, Some(src_path), opts.verbose, opts.progress, false)?
        } else {
            return Err(BackendError::UnsupportedOperation(
                "S3 to S3 copy not yet implemented".to_string(),
            ));
        };

        if opts.verbose {
            println!("Executing: {:?}", cmd);
        }

        let output = cmd.output().map_err(|e| BackendError::IoError {
            message: "Failed to execute aws s3 cp".to_string(),
            error: e.to_string(),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BackendError::IoError {
                message: "aws s3 cp failed".to_string(),
                error: if stderr.trim().is_empty() {
                    format!("Exit code: {}", output.status.code().unwrap_or(-1))
                } else {
                    stderr.trim().to_string()
                },
            });
        }

        let size = if is_download {
            std::fs::metadata(dst)
                .map(|m| m.len())
                .unwrap_or(0)
        } else {
            std::fs::metadata(src)
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

        let is_download = !src.starts_with("s3://") && dst.starts_with("s3://");
        let is_upload = src.starts_with("s3://") && !dst.starts_with("s3://");

        let mut cmd = if is_download {
            let dst_path = Path::new(dst);
            if let Some(parent) = dst_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| BackendError::IoError {
                    message: format!("Failed to create directory: {}", parent.display()),
                    error: e.to_string(),
                })?;
            }
            self.try_aws_cli_sync(dst_path, src, opts.verbose, opts.progress)?
        } else if is_upload {
            let src_path = Path::new(src);
            if !src_path.exists() {
                return Err(BackendError::NotFound(src.to_string()));
            }
            self.try_aws_cli_sync(src_path, dst, opts.verbose, opts.progress)?
        } else {
            return Err(BackendError::UnsupportedOperation(
                "S3 to S3 sync not yet implemented".to_string(),
            ));
        };

        if opts.verbose {
            println!("Executing: {:?}", cmd);
        }

        let output = cmd.output().map_err(|e| BackendError::IoError {
            message: "Failed to execute aws s3 sync".to_string(),
            error: e.to_string(),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BackendError::IoError {
                message: "aws s3 sync failed".to_string(),
                error: if stderr.trim().is_empty() {
                    format!("Exit code: {}", output.status.code().unwrap_or(-1))
                } else {
                    stderr.trim().to_string()
                },
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
        let s3_url = self.s3_url(path);
        let mut cmd = Command::new("aws");
        cmd.arg("s3").arg("ls");

        if let Some(endpoint) = std::env::var("AWS_ENDPOINT_URL_S3")
            .or_else(|_| std::env::var("AWS_ENDPOINT_URL"))
            .ok()
        {
            cmd.env("AWS_ENDPOINT_URL_S3", &endpoint);
        }

        cmd.arg(&s3_url);

        let output = cmd.output().map_err(|e| BackendError::IoError {
            message: "Failed to execute aws s3 ls".to_string(),
            error: e.to_string(),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BackendError::IoError {
                message: "aws s3 ls failed".to_string(),
                error: if stderr.trim().is_empty() {
                    format!("Exit code: {}", output.status.code().unwrap_or(-1))
                } else {
                    stderr.trim().to_string()
                },
            });
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut files = Vec::new();

        for line in output_str.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let is_dir = parts[0].starts_with("PRE");
                let size: u64 = if is_dir {
                    0
                } else {
                    parts[2].parse().unwrap_or(0)
                };
                let name = parts[3];

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
        let mut cmd = Command::new("aws");
        cmd.arg("s3").arg("rm").arg(&self.s3_url(path));

        if let Some(endpoint) = std::env::var("AWS_ENDPOINT_URL_S3")
            .or_else(|_| std::env::var("AWS_ENDPOINT_URL"))
            .ok()
        {
            cmd.env("AWS_ENDPOINT_URL_S3", &endpoint);
        }

        let status = cmd.status().map_err(|e| BackendError::IoError {
            message: "Failed to execute aws s3 rm".to_string(),
            error: e.to_string(),
        })?;

        if !status.success() {
            return Err(BackendError::IoError {
                message: "aws s3 rm failed".to_string(),
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
        let mut cmd = Command::new("aws");
        cmd.arg("s3api").arg("head-object");
        cmd.arg("--bucket").arg(
            self.remote_path
                .url
                .host_str()
                .ok_or_else(|| BackendError::InvalidPath("No bucket specified".to_string()))?,
        );
        cmd.arg("--key").arg(path.trim_start_matches('/'));

        if let Some(endpoint) = std::env::var("AWS_ENDPOINT_URL_S3")
            .or_else(|_| std::env::var("AWS_ENDPOINT_URL"))
            .ok()
        {
            cmd.env("AWS_ENDPOINT_URL_S3", &endpoint);
        }

        let output = cmd.output().map_err(|e| BackendError::IoError {
            message: "Failed to execute aws s3api head-object".to_string(),
            error: e.to_string(),
        })?;

        if !output.status.success() {
            return Err(BackendError::IoError {
                message: "aws s3api head-object failed".to_string(),
                error: format!("Exit code: {}", output.status.code().unwrap_or(-1)),
            });
        }

        let output_str = String::from_utf8_lossy(&output.stdout);

        match algorithm {
            ChecksumAlgorithm::Md5 => {
                if let Some(etag) = output_str
                    .lines()
                    .find(|l| l.contains("ETag"))
                    .and_then(|l| l.split('"').nth(1))
                {
                    Ok(etag.to_string())
                } else {
                    Err(BackendError::Other("ETag not found in response".to_string()))
                }
            }
            ChecksumAlgorithm::Sha1 | ChecksumAlgorithm::Sha256 => {
                Err(BackendError::UnsupportedOperation(format!(
                    "S3 does not support {} checksums via CLI",
                    match algorithm {
                        ChecksumAlgorithm::Sha1 => "SHA1",
                        ChecksumAlgorithm::Sha256 => "SHA256",
                        _ => unreachable!(),
                    }
                )))
            }
        }
    }
}

