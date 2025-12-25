use crate::backend::traits::{Backend, CopyOptions, ChecksumAlgorithm};
use crate::backend::error::BackendError;
use crate::copy::CopyStats;

pub struct CopyOperation {
    backend: Box<dyn Backend>,
    options: CopyOptions,
    checksum_algorithm: Option<ChecksumAlgorithm>,
}

impl CopyOperation {
    pub fn new(backend: Box<dyn Backend>, options: CopyOptions) -> Self {
        Self {
            backend,
            options,
            checksum_algorithm: None,
        }
    }

    pub fn with_checksum(mut self, algorithm: ChecksumAlgorithm) -> Self {
        self.checksum_algorithm = Some(algorithm);
        self
    }

    pub fn copy_file(&self, src: &str, dst: &str) -> Result<u64, BackendError> {
        if self.options.dry_run {
            if self.options.verbose {
                println!("[DRY RUN] Would copy: {} -> {}", src, dst);
            }
            return Ok(0);
        }

        let bytes = self.backend.copy_file(src, dst, &self.options)?;

        if let Some(algorithm) = self.checksum_algorithm {
            if self.options.verbose {
                println!("Verifying checksum using {:?}...", algorithm);
            }

            let src_checksum = self.backend.checksum(src, algorithm)?;
            let dst_checksum = self.backend.checksum(dst, algorithm)?;

            if src_checksum != dst_checksum {
                return Err(BackendError::ChecksumMismatch {
                    expected: src_checksum,
                    actual: dst_checksum,
                });
            }

            if self.options.verbose {
                println!("✓ Checksum verified: {}", src_checksum);
            }
        }

        Ok(bytes)
    }

    pub fn copy_directory(&self, src: &str, dst: &str) -> Result<CopyStats, BackendError> {
        if self.options.dry_run {
            if self.options.verbose {
                println!("[DRY RUN] Would copy directory: {} -> {}", src, dst);
            }
            return Ok(CopyStats::new_minimal());
        }

        let mut opts = self.options.clone();
        opts.recursive = true;
        let stats = self.backend.copy_directory(src, dst, &opts)?;

        if let Some(algorithm) = self.checksum_algorithm {
            if self.options.verbose {
                println!("Verifying checksums for directory...");
            }
            self.verify_directory_checksums(src, dst, algorithm)?;
        }

        Ok(stats)
    }

    fn verify_directory_checksums(
        &self,
        src: &str,
        dst: &str,
        algorithm: ChecksumAlgorithm,
    ) -> Result<(), BackendError> {
        let src_files = self.backend.list(src)?;
        let mut failed = Vec::new();

        for src_file in src_files {
            if src_file.is_dir {
                continue;
            }

            let rel_path = src_file.path.strip_prefix(src)
                .unwrap_or(&src_file.path)
                .trim_start_matches('/');

            let dst_path = if dst.ends_with('/') {
                format!("{}{}", dst, rel_path)
            } else {
                format!("{}/{}", dst, rel_path)
            };

            match (self.backend.checksum(&src_file.path, algorithm),
                   self.backend.checksum(&dst_path, algorithm)) {
                (Ok(src_hash), Ok(dst_hash)) if src_hash == dst_hash => {
                    if self.options.verbose {
                        println!("✓ {}: {}", rel_path, src_hash);
                    }
                }
                (Ok(src_hash), Ok(dst_hash)) => {
                    failed.push((rel_path.to_string(), src_hash, dst_hash));
                }
                (Err(e), _) | (_, Err(e)) => {
                    return Err(e);
                }
            }
        }

        if !failed.is_empty() {
            let failed_count = failed.len();
            eprintln!("Checksum verification failed for {} files:", failed_count);
            for (path, expected, actual) in &failed {
                eprintln!("  {}: expected {}, got {}", path, expected, actual);
            }
            return Err(BackendError::ChecksumMismatch {
                expected: format!("{} files verified", failed_count),
                actual: "mismatch".to_string(),
            });
        }

        if self.options.verbose {
            println!("✓ All files verified successfully");
        }

        Ok(())
    }

    pub fn set_options(&mut self, options: CopyOptions) {
        self.options = options;
    }

    pub fn options(&self) -> &CopyOptions {
        &self.options
    }
}

