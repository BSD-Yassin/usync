#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;
    
    use usync::backend::local::LocalBackend;
    use usync::backend::traits::{Backend, ChecksumAlgorithm, CopyOptions};
    use usync::filters::{Filter, FilterChain, PatternFilter, SizeFilter};
    use usync::operations::{CopyOperation, SyncOperation};
    use usync::operations::sync::SyncMode;

    fn setup_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        
        fs::create_dir_all(path.join("subdir")).unwrap();
        fs::write(path.join("file1.txt"), "content1").unwrap();
        fs::write(path.join("file2.txt"), "content2").unwrap();
        fs::write(path.join("subdir/file3.txt"), "content3").unwrap();
        fs::write(path.join("large.bin"), vec![0u8; 5000]).unwrap();
        
        dir
    }

    #[test]
    fn test_local_backend_copy_file() {
        let src_dir = setup_test_dir();
        let dst_dir = TempDir::new().unwrap();
        
        let backend = LocalBackend::new();
        let opts = CopyOptions::default();
        
        let src = src_dir.path().join("file1.txt");
        let dst = dst_dir.path().join("file1_copy.txt");
        
        let result = backend.copy_file(
            src.to_str().unwrap(),
            dst.to_str().unwrap(),
            &opts,
        );
        
        assert!(result.is_ok());
        assert!(dst.exists());
        assert_eq!(fs::read_to_string(&src).unwrap(), fs::read_to_string(&dst).unwrap());
    }

    #[test]
    fn test_local_backend_copy_directory() {
        let src_dir = setup_test_dir();
        let dst_dir = TempDir::new().unwrap();
        
        let backend = LocalBackend::new();
        let mut opts = CopyOptions::default();
        opts.recursive = true;
        
        let result = backend.copy_directory(
            src_dir.path().to_str().unwrap(),
            dst_dir.path().join("copy").to_str().unwrap(),
            &opts,
        );
        
        assert!(result.is_ok());
        assert!(dst_dir.path().join("copy/file1.txt").exists());
        assert!(dst_dir.path().join("copy/subdir/file3.txt").exists());
    }

    #[test]
    fn test_local_backend_checksum_md5() {
        let test_dir = setup_test_dir();
        let backend = LocalBackend::new();
        
        let file = test_dir.path().join("file1.txt");
        let result = backend.checksum(file.to_str().unwrap(), ChecksumAlgorithm::Md5);
        
        assert!(result.is_ok());
        let hash = result.unwrap();
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_local_backend_checksum_sha1() {
        let test_dir = setup_test_dir();
        let backend = LocalBackend::new();
        
        let file = test_dir.path().join("file1.txt");
        let result = backend.checksum(file.to_str().unwrap(), ChecksumAlgorithm::Sha1);
        
        assert!(result.is_ok());
        let hash = result.unwrap();
        assert_eq!(hash.len(), 40);
    }

    #[test]
    fn test_local_backend_checksum_sha256() {
        let test_dir = setup_test_dir();
        let backend = LocalBackend::new();
        
        let file = test_dir.path().join("file1.txt");
        let result = backend.checksum(file.to_str().unwrap(), ChecksumAlgorithm::Sha256);
        
        assert!(result.is_ok());
        let hash = result.unwrap();
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_local_backend_list() {
        let test_dir = setup_test_dir();
        let backend = LocalBackend::new();
        
        let result = backend.list(test_dir.path().to_str().unwrap());
        
        assert!(result.is_ok());
        let files = result.unwrap();
        assert!(files.len() >= 3);
    }

    #[test]
    fn test_local_backend_exists() {
        let test_dir = setup_test_dir();
        let backend = LocalBackend::new();
        
        assert!(backend.exists(test_dir.path().to_str().unwrap()).unwrap());
        assert!(!backend.exists("/nonexistent/path").unwrap());
    }

    #[test]
    fn test_copy_operation_with_checksum() {
        let src_dir = setup_test_dir();
        let dst_dir = TempDir::new().unwrap();
        
        let backend = LocalBackend::new();
        let opts = CopyOptions::default();
        
        let mut copy_op = CopyOperation::new(Box::new(backend), opts);
        copy_op = copy_op.with_checksum(ChecksumAlgorithm::Sha256);
        
        let src = src_dir.path().join("file1.txt");
        let dst = dst_dir.path().join("file1_copy.txt");
        
        let result = copy_op.copy_file(
            src.to_str().unwrap(),
            dst.to_str().unwrap(),
        );
        
        assert!(result.is_ok());
        assert!(dst.exists());
    }

    #[test]
    fn test_copy_operation_dry_run() {
        let src_dir = setup_test_dir();
        let dst_dir = TempDir::new().unwrap();
        
        let backend = LocalBackend::new();
        let mut opts = CopyOptions::default();
        opts.dry_run = true;
        
        let copy_op = CopyOperation::new(Box::new(backend), opts);
        
        let src = src_dir.path().join("file1.txt");
        let dst = dst_dir.path().join("file1_copy.txt");
        
        let result = copy_op.copy_file(
            src.to_str().unwrap(),
            dst.to_str().unwrap(),
        );
        
        assert!(result.is_ok());
        assert!(!dst.exists());
    }

    #[test]
    fn test_pattern_filter_include() {
        let test_dir = setup_test_dir();
        let backend = LocalBackend::new();
        
        let files = backend.list(test_dir.path().to_str().unwrap()).unwrap();
        
        let filter = PatternFilter::new(vec!["*.txt".to_string()], vec![]).unwrap();
        
        let txt_files: Vec<_> = files.iter()
            .filter(|f| filter.matches(f))
            .collect();
        
        assert!(!txt_files.is_empty());
    }

    #[test]
    fn test_pattern_filter_exclude() {
        let test_dir = setup_test_dir();
        let backend = LocalBackend::new();
        
        let files = backend.list(test_dir.path().to_str().unwrap()).unwrap();
        
        let filter = PatternFilter::new(vec![], vec!["*.bin".to_string()]).unwrap();
        
        let filtered_files: Vec<_> = files.iter()
            .filter(|f| filter.matches(f))
            .collect();
        
        assert!(filtered_files.iter().all(|f| !f.path.ends_with(".bin")));
    }

    #[test]
    fn test_size_filter() {
        let test_dir = setup_test_dir();
        let backend = LocalBackend::new();
        
        let files = backend.list(test_dir.path().to_str().unwrap()).unwrap();
        
        let filter = SizeFilter::new(Some(1000), None);
        
        let large_files: Vec<_> = files.iter()
            .filter(|f| filter.matches(f))
            .collect();
        
        assert!(large_files.iter().any(|f| f.path.ends_with("large.bin")));
    }

    #[test]
    fn test_filter_chain() {
        let test_dir = setup_test_dir();
        let backend = LocalBackend::new();
        
        let files = backend.list(test_dir.path().to_str().unwrap()).unwrap();
        
        let mut chain = FilterChain::new();
        chain.add(Box::new(PatternFilter::new(vec!["*.txt".to_string()], vec![]).unwrap()));
        chain.add(Box::new(SizeFilter::new(Some(10), None)));
        
        let filtered: Vec<_> = files.iter()
            .filter(|f| chain.matches(f))
            .collect();
        
        assert!(!filtered.is_empty());
    }

    #[test]
    fn test_sync_operation_one_way() {
        let src_dir = setup_test_dir();
        let dst_dir = TempDir::new().unwrap();
        
        let src_backend = LocalBackend::new();
        let dst_backend = LocalBackend::new();
        
        let opts = CopyOptions::default();
        
        let sync_op = SyncOperation::new(
            Box::new(src_backend),
            Box::new(dst_backend),
            SyncMode::OneWay,
            opts,
        );
        
        let result = sync_op.sync(
            src_dir.path().to_str().unwrap(),
            dst_dir.path().join("sync").to_str().unwrap(),
        );
        
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert!(stats.files_copied > 0);
    }
}

