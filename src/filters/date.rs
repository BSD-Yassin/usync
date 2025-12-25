use crate::backend::traits::FileInfo;
use crate::filters::Filter;

pub struct DateFilter {
    min_date: Option<u64>,
    max_date: Option<u64>,
}

impl DateFilter {
    pub fn new(min_date: Option<u64>, max_date: Option<u64>) -> Self {
        Self { min_date, max_date }
    }
}

impl Filter for DateFilter {
    fn matches(&self, file: &FileInfo) -> bool {
        let modified = match file.modified {
            Some(m) => m,
            None => return true,
        };

        if let Some(min) = self.min_date {
            if modified < min {
                return false;
            }
        }

        if let Some(max) = self.max_date {
            if modified > max {
                return false;
            }
        }

        true
    }
}

