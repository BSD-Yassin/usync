use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::backend::traits::{Backend, ChecksumAlgorithm, CopyOptions, FileInfo};
use crate::backend::error::BackendError;
use crate::copy::CopyStats;
use crate::utils;

pub struct LocalBackend;

impl LocalBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Backend for LocalBackend {
    fn name(&self) -> &str {
        "local"
    }

    fn copy_file(
        &self,
        src: &str,
        dst: &str,
        opts: &CopyOptions,
    ) -> Result<u64, BackendError> {
        let src_path = Path::new(src);
        let dst_path = Path::new(dst);

        if !src_path.exists() {
            return Err(BackendError::NotFound(src.to_string()));
        }

        if !src_path.is_file() {
            return Err(BackendError::InvalidPath(format!(
                "Source is not a file: {}",
                src
            )));
        }

        if let Some(parent) = dst_path.parent() {
            fs::create_dir_all(parent).map_err(|e| BackendError::IoError {
                message: format!("Failed to create directory: {}", parent.display()),
                error: e.to_string(),
            })?;
        }

        let bytes = if opts.use_ram {
            utils::copy_file_via_ram(src_path, dst_path).map_err(|e| {
                BackendError::IoError {
                    message: format!("Failed to copy file via RAM: {}", src),
                    error: e.to_string(),
                }
            })?
        } else {
            #[cfg(target_os = "macos")]
            {
                utils::copy_file_range_macos(src_path, dst_path).map_err(|e| {
                    BackendError::IoError {
                        message: format!("Failed to copy file: {}", src),
                        error: e.to_string(),
                    }
                })?
            }
            #[cfg(target_os = "linux")]
            {
                utils::copy_file_sendfile(src_path, dst_path).map_err(|e| {
                    BackendError::IoError {
                        message: format!("Failed to copy file: {}", src),
                        error: e.to_string(),
                    }
                })?
            }
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            {
                utils::copy_file_buffered(src_path, dst_path).map_err(|e| {
                    BackendError::IoError {
                        message: format!("Failed to copy file: {}", src),
                        error: e.to_string(),
                    }
                })?
            }
        };

        if opts.verbose {
            println!("Copied {} bytes from {} to {}", bytes, src, dst);
        }

        Ok(bytes)
    }

    fn copy_directory(
        &self,
        src: &str,
        dst: &str,
        opts: &CopyOptions,
    ) -> Result<CopyStats, BackendError> {
        let src_path = Path::new(src);
        let dst_path = Path::new(dst);

        if !src_path.exists() {
            return Err(BackendError::NotFound(src.to_string()));
        }

        if !src_path.is_dir() {
            return Err(BackendError::InvalidPath(format!(
                "Source is not a directory: {}",
                src
            )));
        }

        let mut stats = if opts.verbose || opts.progress {
            CopyStats::new()
        } else {
            CopyStats::new_minimal()
        };

        copy_directory_recursive(
            src_path,
            dst_path,
            opts,
            &mut stats,
        )?;

        Ok(stats)
    }

    fn list(&self, path: &str) -> Result<Vec<FileInfo>, BackendError> {
        let path_buf = PathBuf::from(path);

        if !path_buf.exists() {
            return Err(BackendError::NotFound(path.to_string()));
        }

        let mut files = Vec::new();

        if path_buf.is_file() {
            let metadata = fs::metadata(&path_buf).map_err(|e| BackendError::IoError {
                message: format!("Failed to read metadata: {}", path),
                error: e.to_string(),
            })?;

            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs());

            files.push(FileInfo {
                path: path.to_string(),
                size: metadata.len(),
                is_dir: false,
                modified,
            });
        } else if path_buf.is_dir() {
            let entries = fs::read_dir(&path_buf).map_err(|e| BackendError::IoError {
                message: format!("Failed to read directory: {}", path),
                error: e.to_string(),
            })?;

            for entry in entries {
                let entry = entry.map_err(|e| BackendError::IoError {
                    message: format!("Failed to read directory entry: {}", path),
                    error: e.to_string(),
                })?;

                let entry_path = entry.path();
                let metadata = entry.metadata().map_err(|e| BackendError::IoError {
                    message: format!("Failed to read metadata: {}", entry_path.display()),
                    error: e.to_string(),
                })?;

                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs());

                files.push(FileInfo {
                    path: entry_path.to_string_lossy().to_string(),
                    size: if metadata.is_file() { metadata.len() } else { 0 },
                    is_dir: metadata.is_dir(),
                    modified,
                });
            }
        }

        Ok(files)
    }

    fn delete(&self, path: &str) -> Result<(), BackendError> {
        let path_buf = PathBuf::from(path);

        if !path_buf.exists() {
            return Err(BackendError::NotFound(path.to_string()));
        }

        if path_buf.is_file() {
            fs::remove_file(&path_buf).map_err(|e| BackendError::IoError {
                message: format!("Failed to delete file: {}", path),
                error: e.to_string(),
            })
        } else if path_buf.is_dir() {
            fs::remove_dir_all(&path_buf).map_err(|e| BackendError::IoError {
                message: format!("Failed to delete directory: {}", path),
                error: e.to_string(),
            })
        } else {
            Err(BackendError::InvalidPath(format!(
                "Path is neither file nor directory: {}",
                path
            )))
        }
    }

    fn checksum(
        &self,
        path: &str,
        algorithm: ChecksumAlgorithm,
    ) -> Result<String, BackendError> {
        use std::io::Read;

        let path_buf = PathBuf::from(path);
        if !path_buf.exists() || !path_buf.is_file() {
            return Err(BackendError::NotFound(path.to_string()));
        }

        let mut file = fs::File::open(&path_buf).map_err(|e| BackendError::IoError {
            message: format!("Failed to open file: {}", path),
            error: e.to_string(),
        })?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).map_err(|e| BackendError::IoError {
            message: format!("Failed to read file: {}", path),
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

fn copy_directory_recursive(
    src: &Path,
    dst: &Path,
    opts: &CopyOptions,
    stats: &mut CopyStats,
) -> Result<(), BackendError> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| BackendError::IoError {
            message: format!("Failed to create directory: {}", parent.display()),
            error: e.to_string(),
        })?;
    }

    let entries = fs::read_dir(src).map_err(|e| BackendError::IoError {
        message: format!("Failed to read directory: {}", src.display()),
        error: e.to_string(),
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| BackendError::IoError {
            message: format!("Failed to read directory entry: {}", src.display()),
            error: e.to_string(),
        })?;

        let entry_path = entry.path();
        let entry_name = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| BackendError::InvalidPath(format!(
                "Invalid entry name: {}",
                entry_path.display()
            )))?;

        let dst_entry = dst.join(entry_name);

        let metadata = entry.metadata().map_err(|e| BackendError::IoError {
            message: format!("Failed to read metadata: {}", entry_path.display()),
            error: e.to_string(),
        })?;

        if metadata.is_file() {
            let bytes = if opts.use_ram {
                utils::copy_file_via_ram(&entry_path, &dst_entry).map_err(|e| {
                    BackendError::IoError {
                        message: format!("Failed to copy file: {}", entry_path.display()),
                        error: e.to_string(),
                    }
                })?
            } else {
                #[cfg(target_os = "macos")]
                {
                    utils::copy_file_range_macos(&entry_path, &dst_entry).map_err(|e| {
                        BackendError::IoError {
                            message: format!("Failed to copy file: {}", entry_path.display()),
                            error: e.to_string(),
                        }
                    })?
                }
                #[cfg(target_os = "linux")]
                {
                    utils::copy_file_sendfile(&entry_path, &dst_entry).map_err(|e| {
                        BackendError::IoError {
                            message: format!("Failed to copy file: {}", entry_path.display()),
                            error: e.to_string(),
                        }
                    })?
                }
                #[cfg(not(any(target_os = "macos", target_os = "linux")))]
                {
                    utils::copy_file_buffered(&entry_path, &dst_entry).map_err(|e| {
                        BackendError::IoError {
                            message: format!("Failed to copy file: {}", entry_path.display()),
                            error: e.to_string(),
                        }
                    })?
                }
            };

            if stats.start_time.is_some() {
                stats.files_copied += 1;
                stats.bytes_copied += bytes;
            }

            if opts.verbose {
                println!("Copied {} to {}", entry_path.display(), dst_entry.display());
            }
        } else if metadata.is_dir() && opts.recursive {
            copy_directory_recursive(&entry_path, &dst_entry, opts, stats)?;
        }
    }

    Ok(())
}

