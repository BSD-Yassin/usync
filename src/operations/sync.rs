use crate::backend::traits::{Backend, CopyOptions, FileInfo};
use crate::backend::error::BackendError;
use crate::copy::CopyStats;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    OneWay,
    TwoWay,
    CopyOnly,
}

pub struct SyncOperation {
    src_backend: Box<dyn Backend>,
    dst_backend: Box<dyn Backend>,
    mode: SyncMode,
    options: CopyOptions,
}

impl SyncOperation {
    pub fn new(
        src_backend: Box<dyn Backend>,
        dst_backend: Box<dyn Backend>,
        mode: SyncMode,
        options: CopyOptions,
    ) -> Self {
        Self {
            src_backend,
            dst_backend,
            mode,
            options,
        }
    }

    pub fn sync(&self, src_path: &str, dst_path: &str) -> Result<SyncStats, BackendError> {
        match self.mode {
            SyncMode::OneWay => self.sync_one_way(src_path, dst_path),
            SyncMode::TwoWay => Err(BackendError::UnsupportedOperation(
                "Two-way sync not yet implemented".to_string(),
            )),
            SyncMode::CopyOnly => self.sync_copy_only(src_path, dst_path),
        }
    }

    fn sync_one_way(&self, src_path: &str, dst_path: &str) -> Result<SyncStats, BackendError> {
        let src_files = self.src_backend.list(src_path)?;
        let dst_files = self.dst_backend.list(dst_path).unwrap_or_default();

        let mut stats = SyncStats::default();
        let mut files_to_copy = Vec::new();
        let mut files_to_delete = Vec::new();

        for src_file in &src_files {
            let rel_path = src_file.path.strip_prefix(src_path)
                .unwrap_or(&src_file.path)
                .trim_start_matches('/');

            let dst_file_path = if dst_path.ends_with('/') {
                format!("{}{}", dst_path, rel_path)
            } else {
                format!("{}/{}", dst_path, rel_path)
            };

            let dst_file = dst_files.iter().find(|f| {
                f.path.strip_prefix(dst_path)
                    .unwrap_or(&f.path)
                    .trim_start_matches('/') == rel_path
            });

            let needs_copy = match dst_file {
                None => true,
                Some(dst) => {
                    src_file.size != dst.size
                        || src_file.modified != dst.modified
                        || (src_file.is_dir != dst.is_dir)
                }
            };

            if needs_copy && !src_file.is_dir {
                files_to_copy.push((src_file.path.clone(), dst_file_path));
            }
        }

        for dst_file in &dst_files {
            let rel_path = dst_file.path.strip_prefix(dst_path)
                .unwrap_or(&dst_file.path)
                .trim_start_matches('/');

            let src_exists = src_files.iter().any(|f| {
                f.path.strip_prefix(src_path)
                    .unwrap_or(&f.path)
                    .trim_start_matches('/') == rel_path
            });

            if !src_exists && !dst_file.is_dir {
                files_to_delete.push(dst_file.path.clone());
            }
        }

        for (src, dst) in &files_to_copy {
            if self.options.verbose {
                println!("Copying: {} -> {}", src, dst);
            }

            let bytes = self.src_backend.copy_file(src, dst, &self.options)?;
            stats.files_copied += 1;
            stats.bytes_copied += bytes;
        }

        for path in &files_to_delete {
            if self.options.verbose {
                println!("Deleting: {}", path);
            }
            self.dst_backend.delete(path)?;
            stats.files_deleted += 1;
        }

        Ok(stats)
    }

    fn sync_copy_only(&self, src_path: &str, dst_path: &str) -> Result<SyncStats, BackendError> {
        let src_files = self.src_backend.list(src_path)?;

        let mut stats = SyncStats::default();

        for src_file in &src_files {
            if src_file.is_dir {
                continue;
            }

            let rel_path = src_file.path.strip_prefix(src_path)
                .unwrap_or(&src_file.path)
                .trim_start_matches('/');

            let dst_file_path = if dst_path.ends_with('/') {
                format!("{}{}", dst_path, rel_path)
            } else {
                format!("{}/{}", dst_path, rel_path)
            };

            if self.options.verbose {
                println!("Copying: {} -> {}", src_file.path, dst_file_path);
            }

            let bytes = self.src_backend.copy_file(&src_file.path, &dst_file_path, &self.options)?;
            stats.files_copied += 1;
            stats.bytes_copied += bytes;
        }

        Ok(stats)
    }

    pub fn set_mode(&mut self, mode: SyncMode) {
        self.mode = mode;
    }

    pub fn mode(&self) -> SyncMode {
        self.mode
    }

    pub fn set_options(&mut self, options: CopyOptions) {
        self.options = options;
    }

    pub fn options(&self) -> &CopyOptions {
        &self.options
    }
}

#[derive(Debug, Default)]
pub struct SyncStats {
    pub files_copied: usize,
    pub bytes_copied: u64,
    pub files_deleted: usize,
}

