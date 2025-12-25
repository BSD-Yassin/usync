use crate::backend::traits::{Backend, FileInfo};
use crate::backend::error::BackendError;

pub struct ListOperation {
    backend: Box<dyn Backend>,
}

impl ListOperation {
    pub fn new(backend: Box<dyn Backend>) -> Self {
        Self { backend }
    }

    pub fn list(&self, path: &str) -> Result<Vec<FileInfo>, BackendError> {
        self.backend.list(path)
    }

    pub fn list_recursive(&self, path: &str) -> Result<Vec<FileInfo>, BackendError> {
        let mut all_files = Vec::new();
        let mut to_process = vec![path.to_string()];

        while let Some(current_path) = to_process.pop() {
            let files = self.backend.list(&current_path)?;

            for file in files {
                if file.is_dir {
                    to_process.push(file.path.clone());
                }
                all_files.push(file);
            }
        }

        Ok(all_files)
    }

    pub fn exists(&self, path: &str) -> Result<bool, BackendError> {
        self.backend.exists(path)
    }
}

