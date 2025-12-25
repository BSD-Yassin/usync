use crate::backend::traits::FileInfo;
use crate::filters::Filter;

pub struct PatternFilter {
    include_patterns: Vec<glob::Pattern>,
    exclude_patterns: Vec<glob::Pattern>,
}

impl PatternFilter {
    pub fn new(include: Vec<String>, exclude: Vec<String>) -> Result<Self, String> {
        let include_patterns: Result<Vec<_>, _> = include
            .into_iter()
            .map(|p| glob::Pattern::new(&p))
            .collect();
        
        let exclude_patterns: Result<Vec<_>, _> = exclude
            .into_iter()
            .map(|p| glob::Pattern::new(&p))
            .collect();

        Ok(Self {
            include_patterns: include_patterns.map_err(|e| format!("Invalid include pattern: {}", e))?,
            exclude_patterns: exclude_patterns.map_err(|e| format!("Invalid exclude pattern: {}", e))?,
        })
    }
}

impl Filter for PatternFilter {
    fn matches(&self, file: &FileInfo) -> bool {
        if !self.exclude_patterns.is_empty() {
            for pattern in &self.exclude_patterns {
                if pattern.matches(&file.path) {
                    return false;
                }
            }
        }

        if !self.include_patterns.is_empty() {
            for pattern in &self.include_patterns {
                if pattern.matches(&file.path) {
                    return true;
                }
            }
            return false;
        }

        true
    }
}

