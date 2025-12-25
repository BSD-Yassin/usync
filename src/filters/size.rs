use crate::backend::traits::FileInfo;
use crate::filters::Filter;

pub struct SizeFilter {
    min_size: Option<u64>,
    max_size: Option<u64>,
}

impl SizeFilter {
    pub fn new(min_size: Option<u64>, max_size: Option<u64>) -> Self {
        Self { min_size, max_size }
    }
}

impl Filter for SizeFilter {
    fn matches(&self, file: &FileInfo) -> bool {
        if file.is_dir {
            return true;
        }

        if let Some(min) = self.min_size {
            if file.size < min {
                return false;
            }
        }

        if let Some(max) = self.max_size {
            if file.size > max {
                return false;
            }
        }

        true
    }
}

