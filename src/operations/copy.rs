use crate::backend::traits::{Backend, CopyOptions};
use crate::backend::error::BackendError;
use crate::copy::CopyStats;

pub struct CopyOperation {
    backend: Box<dyn Backend>,
    options: CopyOptions,
}

impl CopyOperation {
    pub fn new(backend: Box<dyn Backend>, options: CopyOptions) -> Self {
        Self { backend, options }
    }

    pub fn copy_file(&self, src: &str, dst: &str) -> Result<u64, BackendError> {
        self.backend.copy_file(src, dst, &self.options)
    }

    pub fn copy_directory(&self, src: &str, dst: &str) -> Result<CopyStats, BackendError> {
        let mut opts = self.options.clone();
        opts.recursive = true;
        self.backend.copy_directory(src, dst, &opts)
    }

    pub fn set_options(&mut self, options: CopyOptions) {
        self.options = options;
    }

    pub fn options(&self) -> &CopyOptions {
        &self.options
    }
}

