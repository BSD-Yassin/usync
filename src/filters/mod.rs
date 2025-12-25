pub mod pattern;
pub mod size;
pub mod date;

pub use pattern::PatternFilter;
pub use size::SizeFilter;
pub use date::DateFilter;

use crate::backend::traits::FileInfo;

pub trait Filter: Send + Sync {
    fn matches(&self, file: &FileInfo) -> bool;
}

pub struct FilterChain {
    filters: Vec<Box<dyn Filter>>,
}

impl FilterChain {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    pub fn add(&mut self, filter: Box<dyn Filter>) {
        self.filters.push(filter);
    }

    pub fn matches(&self, file: &FileInfo) -> bool {
        self.filters.iter().all(|f| f.matches(file))
    }
}

impl Default for FilterChain {
    fn default() -> Self {
        Self::new()
    }
}

