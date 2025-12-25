use std::fs;
use std::io;
use std::path::Path;
use std::time::Instant;
use std::sync::{Arc, Mutex};

use crate::path::LocalPath;
use crate::protocol::Path as ProtocolPath;
use crate::remote;
use crate::utils;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Statistics for copy operations
#[derive(Default)]
pub struct CopyStats {
    pub files_copied: usize,
    pub bytes_copied: u64,
    pub files_skipped: usize,
    pub start_time: Option<Instant>,
}

impl CopyStats {
    pub fn new() -> Self {
        Self {
            files_copied: 0,
            bytes_copied: 0,
            files_skipped: 0,
            start_time: Some(Instant::now()),
        }
    }

    pub fn print_summary(&self, verbose: bool) {
        if let Some(start) = self.start_time {
            let duration = start.elapsed();
            let speed = if duration.as_secs_f64() > 0.0 {
                self.bytes_copied as f64 / duration.as_secs_f64() / 1_048_576.0
            } else {
                0.0
            };
            
            if verbose {
                println!("\n=== Copy Summary ===");
                println!("Files copied: {}", self.files_copied);
                println!("Bytes transferred: {} ({:.2} MB)", self.bytes_copied, self.bytes_copied as f64 / 1_048_576.0);
                println!("Files skipped: {}", self.files_skipped);
                println!("Time taken: {:.2}s", duration.as_secs_f64());
                println!("Average speed: {:.2} MB/s", speed);
            } else {
                println!("\nSummary: {} files, {:.2} MB, {:.2}s, {:.2} MB/s", 
                    self.files_copied,
                    self.bytes_copied as f64 / 1_048_576.0,
                    duration.as_secs_f64(),
                    speed
                );
            }
        }
    }
}

#[cfg(feature = "progress")]
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};

pub fn copy(src: &ProtocolPath, dst: &ProtocolPath, verbose: bool, ssh_opts: &[String], progress: bool, use_ram: bool) -> Result<CopyStats, CopyError> {
    let mut stats = CopyStats::new();
    
    let result = match (src, dst) {
        (ProtocolPath::Local(src_local), ProtocolPath::Local(dst_local)) => {
            copy_local_with_stats(src_local, dst_local, verbose, progress, use_ram, &mut stats)
        }
        (ProtocolPath::Remote(src_remote), ProtocolPath::Remote(dst_remote)) => {
            remote::copy_remote(src_remote, dst_remote, verbose, ssh_opts, progress)
                .map_err(|e| CopyError::RemoteError(e))
                .map(|_| ())
        }
        (ProtocolPath::Remote(src_remote), ProtocolPath::Local(dst_local)) => {
            copy_from_remote_to_local(src_remote, dst_local, verbose, ssh_opts, progress)
        }
        (ProtocolPath::Local(src_local), ProtocolPath::Remote(dst_remote)) => {
            copy_from_local_to_remote(src_local, dst_remote, verbose, ssh_opts, progress)
        }
    };
    
    result.map(|_| stats)
}

fn copy_local_with_stats(src: &LocalPath, dst: &LocalPath, verbose: bool, progress: bool, use_ram: bool, stats: &mut CopyStats) -> Result<(), CopyError> {
    if !src.exists() {
        return Err(CopyError::SourceNotFound(src.to_string_lossy().to_string()));
    }

    let src_path = src.as_path();
    let dst_path = dst.as_path();

    if src.is_file() {
        let bytes = copy_file(src_path, dst_path, verbose, progress, use_ram)?;
        stats.files_copied += 1;
        stats.bytes_copied += bytes;
        Ok(())
    } else if src.is_dir() {
        copy_directory_with_stats(src_path, dst_path, verbose, progress, use_ram, stats)
    } else {
        Err(CopyError::InvalidSource(
            "Source path is neither a file nor a directory".to_string(),
        ))
    }
}

fn copy_from_remote_to_local(
    src: &crate::protocol::RemotePath,
    dst: &LocalPath,
    verbose: bool,
    ssh_opts: &[String],
    progress: bool,
) -> Result<(), CopyError> {
    match src.protocol {
        crate::protocol::Protocol::Ssh | crate::protocol::Protocol::Sftp => {
            let dst_path = dst.as_path();
            remote::copy_from_ssh_to_file(src, dst_path, verbose, ssh_opts, progress)
                .map_err(|e| CopyError::RemoteError(e))
        }
        crate::protocol::Protocol::Http | crate::protocol::Protocol::Https => {
            let dst_path = dst.as_path();
            remote::copy_from_http_to_file(src, dst_path, verbose, progress)
                .map_err(|e| CopyError::RemoteError(e))
        }
        _ => Err(CopyError::UnsupportedProtocol(format!(
            "Copying from {} protocol is not supported",
            src.protocol
        ))),
    }
}

fn copy_from_local_to_remote(
    src: &LocalPath,
    dst: &crate::protocol::RemotePath,
    verbose: bool,
    ssh_opts: &[String],
    progress: bool,
) -> Result<(), CopyError> {
    match dst.protocol {
        crate::protocol::Protocol::Ssh | crate::protocol::Protocol::Sftp => {
            let src_path = src.as_path();
            if src.is_file() {
                remote::copy_file_to_ssh(src_path, dst, verbose, ssh_opts, progress)
                    .map_err(|e| CopyError::RemoteError(e))
            } else {
                Err(CopyError::UnsupportedProtocol(
                    "Directory copying to remote is not yet implemented".to_string()
                ))
            }
        }
        _ => Err(CopyError::UnsupportedProtocol(format!(
            "Copying to {} protocol is not supported",
            dst.protocol
        ))),
    }
}

// Legacy function for backward compatibility
pub fn copy_local(src: &LocalPath, dst: &LocalPath, verbose: bool, progress: bool) -> Result<(), CopyError> {
    let mut stats = CopyStats::new();
    copy_local_with_stats(src, dst, verbose, progress, false, &mut stats)
}

fn copy_file(src: &Path, dst: &Path, verbose: bool, progress: bool, use_ram: bool) -> Result<u64, CopyError> {
    let final_dst = if dst.is_dir() {
        if let Some(file_name) = src.file_name() {
            dst.join(file_name)
        } else {
            return Err(CopyError::InvalidSource(
                "Source file has no name".to_string(),
            ));
        }
    } else {
        dst.to_path_buf()
    };

    if let Some(parent) = final_dst.parent() {
        if verbose {
            println!("Creating directory: {}", parent.display());
        }
        fs::create_dir_all(parent).map_err(|e| CopyError::IoError {
            message: format!("Failed to create destination directory: {}", parent.display()),
            error: e,
        })?;
    }

    let src_size = fs::metadata(src)
        .map(|m| m.len())
        .unwrap_or(0);

    #[cfg(feature = "progress")]
    let pb: Option<ProgressBar> = if progress {
        use std::io::IsTerminal;
        if std::io::stdout().is_terminal() {
            let pb = ProgressBar::new(src_size);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("[{bar:40.cyan/blue}] {bytes}/{total_bytes} ({percent}%) {bytes_per_sec} ETA: {eta}")
                    .unwrap()
                    .progress_chars("#>-"),
            );
            Some(pb)
        } else {
            None
        }
    } else {
        None
    };

    if verbose && !progress {
        println!("Copying file: {} -> {}", src.display(), final_dst.display());
    } else if progress {
        let show_simple = {
            #[cfg(feature = "progress")]
            {
                pb.is_none()
            }
            #[cfg(not(feature = "progress"))]
            {
                true
            }
        };
        
        if show_simple {
            print!("Copying {} ({} bytes)... ", src.display(), src_size);
            use std::io::Write;
            io::stdout().flush().unwrap();
        }
    }

    let start = Instant::now();
    
    // Choose copy method based on flags
    let result = if use_ram {
        // Check file size before using RAM (warn if very large)
        if src_size > 100 * 1024 * 1024 { // 100MB
            if verbose {
                eprintln!("Warning: File is large ({} MB), RAM copy may use significant memory", src_size as f64 / 1_048_576.0);
            }
        }
        utils::copy_file_via_ram(src, &final_dst)
    } else {
        // Try sendfile on Linux, fallback to buffered copy
        #[cfg(target_os = "linux")]
        {
            if src_size > 1024 * 1024 {
                // Use sendfile for large files on Linux
                utils::copy_file_sendfile(src, &final_dst)
                    .or_else(|_| utils::copy_file_buffered(src, &final_dst))
            } else {
                utils::copy_file_buffered(src, &final_dst)
            }
        }
        #[cfg(not(target_os = "linux"))]
        utils::copy_file_buffered(src, &final_dst)
    };

    match result {
        Ok(bytes_copied) => {
            #[cfg(feature = "progress")]
            {
                if let Some(ref p) = pb {
                    p.finish_with_message("Done");
                } else if progress {
                    println!("✓");
                }
            }
            #[cfg(not(feature = "progress"))]
            {
                if progress {
                    println!("✓");
                }
            }
            
            if verbose {
                let duration = start.elapsed();
                let speed = if duration.as_secs_f64() > 0.0 {
                    bytes_copied as f64 / duration.as_secs_f64() / 1_048_576.0
                } else {
                    0.0
                };
                println!("Copied {} bytes in {:.2}s ({:.2} MB/s)", bytes_copied, duration.as_secs_f64(), speed);
            }
            Ok(bytes_copied)
        }
        Err(e) => {
            #[cfg(feature = "progress")]
            {
                if let Some(ref p) = pb {
                    p.abandon();
                }
            }
            Err(CopyError::IoError {
                message: format!(
                    "Failed to copy file from {} to {}",
                    src.display(),
                    final_dst.display()
                ),
                error: e,
            })
        }
    }
}

fn copy_directory_with_stats(src: &Path, dst: &Path, verbose: bool, progress: bool, use_ram: bool, stats: &mut CopyStats) -> Result<(), CopyError> {
    if !dst.exists() {
        if verbose {
            println!("Creating destination directory: {}", dst.display());
        }
        fs::create_dir_all(dst).map_err(|e| CopyError::IoError {
            message: format!("Failed to create destination directory: {}", dst.display()),
            error: e,
        })?;
    }

    copy_directory_recursive_with_stats(src, dst, verbose, progress, use_ram, stats)?;

    Ok(())
}

fn copy_directory(src: &Path, dst: &Path, verbose: bool, progress: bool) -> Result<(), CopyError> {
    let mut stats = CopyStats::new();
    copy_directory_with_stats(src, dst, verbose, progress, false, &mut stats)
}

fn copy_directory_recursive_with_stats(src: &Path, dst: &Path, verbose: bool, progress: bool, use_ram: bool, stats: &mut CopyStats) -> Result<(), CopyError> {
    // Count total files first for progress tracking
    let total_files = count_files(src)?;
    
    #[cfg(feature = "progress")]
    let (multi, overall_pb, current_pb) = {
        use std::io::IsTerminal;
        if progress && std::io::stdout().is_terminal() {
        let multi = MultiProgress::new();
        let overall_pb = multi.add(ProgressBar::new(total_files as u64));
        overall_pb.set_style(
            ProgressStyle::default_bar()
                .template("[{bar:40.cyan/blue}] {pos}/{len} files ({percent}%)")
                .unwrap()
                .progress_chars("#>-"),
        );
        let current_pb = multi.add(ProgressBar::new(0));
        current_pb.set_style(
            ProgressStyle::default_bar()
                .template("  [{bar:30.green/yellow}] {bytes}/{total_bytes} ({percent}%) {msg}")
                .unwrap()
                .progress_chars("=>-"),
        );
            (Some(multi), Some(overall_pb), Some(current_pb))
        } else {
            (None, None, None)
        }
    };
    
    #[cfg(not(feature = "progress"))]
    let (multi, overall_pb, current_pb): (Option<()>, Option<()>, Option<()>) = (None, None, None);

    #[cfg(feature = "progress")]
    copy_directory_recursive_impl(src, dst, verbose, progress, false, stats, &overall_pb, &current_pb)?;
    #[cfg(not(feature = "progress"))]
    copy_directory_recursive_impl(src, dst, verbose, progress, false, stats, &None, &None)?;

    #[cfg(feature = "progress")]
    if let (Some(ref o), Some(ref c)) = (overall_pb, current_pb) {
        o.finish();
        c.finish();
    }

    Ok(())
}

fn count_files(path: &Path) -> Result<usize, CopyError> {
    let mut count = 0;
    if path.is_dir() {
        let entries = fs::read_dir(path).map_err(|e| CopyError::IoError {
            message: format!("Failed to read directory: {}", path.display()),
            error: e,
        })?;
        for entry in entries {
            let entry = entry.map_err(|e| CopyError::IoError {
                message: format!("Failed to read directory entry: {}", path.display()),
                error: e,
            })?;
            let path = entry.path();
            if path.is_dir() {
                count += count_files(&path)?;
            } else {
                count += 1;
            }
        }
    } else {
        count = 1;
    }
    Ok(count)
}

fn copy_directory_recursive_impl(
    src: &Path,
    dst: &Path,
    verbose: bool,
    progress: bool,
    use_ram: bool,
    stats: &mut CopyStats,
    #[cfg(feature = "progress")] overall_pb: &Option<ProgressBar>,
    #[cfg(feature = "progress")] current_pb: &Option<ProgressBar>,
    #[cfg(not(feature = "progress"))] _overall_pb: &Option<()>,
    #[cfg(not(feature = "progress"))] _current_pb: &Option<()>,
) -> Result<(), CopyError> {
    let entries: Vec<_> = fs::read_dir(src)
        .map_err(|e| CopyError::IoError {
            message: format!("Failed to read source directory: {}", src.display()),
            error: e,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| CopyError::IoError {
            message: format!("Failed to read directory entry in: {}", src.display()),
            error: e,
        })?;

    // Collect files and directories separately for parallel processing
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    
    for entry in entries {
        let entry_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);
        
        if entry_path.is_dir() {
            dirs.push((entry_path, dst_path));
        } else {
            files.push((entry_path, dst_path, file_name));
        }
    }

    // Process directories first (sequential, as they may contain files)
    for (src_path, dst_path) in dirs {
        if verbose && !progress {
            println!("Copying directory: {} -> {}", src_path.display(), dst_path.display());
        }
        fs::create_dir_all(&dst_path).map_err(|e| CopyError::IoError {
            message: format!("Failed to create directory: {}", dst_path.display()),
            error: e,
        })?;
            #[cfg(feature = "progress")]
            copy_directory_recursive_impl(&src_path, &dst_path, verbose, progress, use_ram, stats, overall_pb, current_pb)?;
            #[cfg(not(feature = "progress"))]
            copy_directory_recursive_impl(&src_path, &dst_path, verbose, progress, use_ram, stats, &None, &None)?;
    }

    // Process files in parallel if feature enabled
    #[cfg(feature = "parallel")]
    {
        let stats_arc = Arc::new(Mutex::new((0usize, 0u64)));
        files.par_iter().try_for_each(|(src_path, dst_path, file_name)| -> Result<(), CopyError> {
            // Cache metadata
            let file_size = fs::metadata(src_path)
                .map(|m| m.len())
                .unwrap_or(0);
            
            #[cfg(feature = "progress")]
            if let Some(ref pb) = current_pb {
                pb.set_length(file_size);
                pb.set_message(file_name.to_string_lossy().to_string());
                pb.set_position(0);
            }
            
            if verbose && !progress {
                println!("Copying file: {} -> {}", src_path.display(), dst_path.display());
            } else if progress {
                let show_simple = {
                    #[cfg(feature = "progress")]
                    { current_pb.is_none() }
                    #[cfg(not(feature = "progress"))]
                    { true }
                };
                if show_simple {
                    print!("  {} ({} bytes)... ", file_name.to_string_lossy(), file_size);
                    use std::io::Write;
                    io::stdout().flush().unwrap();
                }
            }
            
            let bytes = if use_ram {
                utils::copy_file_via_ram(src_path, dst_path).map_err(|e| CopyError::IoError {
                    message: format!(
                        "Failed to copy file from {} to {}",
                        src_path.display(),
                        dst_path.display()
                    ),
                    error: e,
                })?
            } else {
                fs::copy(src_path, dst_path).map_err(|e| CopyError::IoError {
                    message: format!(
                        "Failed to copy file from {} to {}",
                        src_path.display(),
                        dst_path.display()
                    ),
                    error: e,
                })?
            };
            
            #[cfg(feature = "progress")]
            if let Some(ref pb) = current_pb {
                pb.finish();
            }
            
            {
                let mut s = stats_arc.lock().unwrap();
                s.0 += 1;
                s.1 += bytes;
            }
            
            #[cfg(feature = "progress")]
            if let Some(ref pb) = overall_pb {
                pb.inc(1);
            }
            
            {
                let show_check = {
                    #[cfg(feature = "progress")]
                    { progress && current_pb.is_none() }
                    #[cfg(not(feature = "progress"))]
                    { progress }
                };
                if show_check {
                    println!("✓");
                }
            }
            
            Ok(())
        })?;
        
        let (files_count, bytes_count) = *stats_arc.lock().unwrap();
        stats.files_copied += files_count;
        stats.bytes_copied += bytes_count;
    }
    
    #[cfg(not(feature = "parallel"))]
    {
        for (src_path, dst_path, file_name) in files {
            // Cache metadata
            let file_size = fs::metadata(&src_path)
                .map(|m| m.len())
                .unwrap_or(0);
            
            #[cfg(feature = "progress")]
            if let Some(ref pb) = current_pb {
                pb.set_length(file_size);
                pb.set_message(file_name.to_string_lossy().to_string());
                pb.set_position(0);
            }
            
            if verbose && !progress {
                println!("Copying file: {} -> {}", src_path.display(), dst_path.display());
            } else if progress {
                let show_simple = {
                    #[cfg(feature = "progress")]
                    { current_pb.is_none() }
                    #[cfg(not(feature = "progress"))]
                    { true }
                };
                if show_simple {
                    print!("  {} ({} bytes)... ", file_name.to_string_lossy(), file_size);
                    use std::io::Write;
                    io::stdout().flush().unwrap();
                }
            }
            
            let bytes = if use_ram {
                utils::copy_file_via_ram(&src_path, &dst_path).map_err(|e| CopyError::IoError {
                    message: format!(
                        "Failed to copy file from {} to {}",
                        src_path.display(),
                        dst_path.display()
                    ),
                    error: e,
                })?
            } else {
                fs::copy(&src_path, &dst_path).map_err(|e| CopyError::IoError {
                    message: format!(
                        "Failed to copy file from {} to {}",
                        src_path.display(),
                        dst_path.display()
                    ),
                    error: e,
                })?
            };
            
            #[cfg(feature = "progress")]
            if let Some(ref pb) = current_pb {
                pb.finish();
            }
            
            stats.files_copied += 1;
            stats.bytes_copied += bytes;
            
            #[cfg(feature = "progress")]
            if let Some(ref pb) = overall_pb {
                pb.inc(1);
            }
            
            {
                let show_check = {
                    #[cfg(feature = "progress")]
                    { progress && current_pb.is_none() }
                    #[cfg(not(feature = "progress"))]
                    { progress }
                };
                if show_check {
                    println!("✓");
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
pub enum CopyError {
    SourceNotFound(String),
    InvalidSource(String),
    IoError { message: String, error: io::Error },
    RemoteError(crate::remote::RemoteCopyError),
    UnsupportedProtocol(String),
}

impl std::fmt::Display for CopyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CopyError::SourceNotFound(path) => {
                write!(f, "Source path not found: {}\n\nSuggestion: Check that the file or directory exists and you have read permissions.", path)
            }
            CopyError::InvalidSource(msg) => {
                write!(f, "Invalid source: {}\n\nSuggestion: Ensure the source is a valid file or directory.", msg)
            }
            CopyError::IoError { message, error } => {
                write!(f, "{}\n\nError details: {}\n\nSuggestion: Check file permissions and available disk space.", message, error)
            }
            CopyError::RemoteError(e) => {
                write!(f, "Remote copy error: {}\n\nSuggestion: Verify network connectivity and remote server access.", e)
            }
            CopyError::UnsupportedProtocol(msg) => {
                write!(f, "Unsupported protocol: {}\n\nSupported protocols: ssh://, sftp://, http://, https://\nFor more information, see: https://github.com/yassinbousaadi/usync", msg)
            }
        }
    }
}

impl std::error::Error for CopyError {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_files() -> (TempDir, LocalPath, LocalPath) {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        let dst_dir = temp_dir.path().join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dst_dir).unwrap();
        
        fs::write(src_dir.join("test.txt"), "test content").unwrap();
        
        let src_path = LocalPath::parse(src_dir.join("test.txt").to_str().unwrap()).unwrap();
        let dst_path = LocalPath::parse(dst_dir.join("test_copy.txt").to_str().unwrap()).unwrap();
        
        (temp_dir, src_path, dst_path)
    }

    #[test]
    fn test_copy_local_file() {
        let (_temp, src, dst) = setup_test_files();
        let result = copy_local(&src, &dst, false, false);
        assert!(result.is_ok());
        assert!(dst.as_path().exists());
        
        let content = fs::read_to_string(dst.as_path()).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_copy_to_nonexistent_source() {
        let temp_dir = TempDir::new().unwrap();
        let src = LocalPath::parse("/nonexistent/path/file.txt").unwrap();
        let dst = LocalPath::parse(temp_dir.path().join("dest.txt").to_str().unwrap()).unwrap();
        
        let result = copy_local(&src, &dst, false, false);
        assert!(result.is_err());
        assert!(matches!(result, Err(CopyError::SourceNotFound(_))));
    }

    #[test]
    fn test_copy_directory() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        let dst_dir = temp_dir.path().join("dst");
        
        fs::create_dir_all(src_dir.join("subdir")).unwrap();
        fs::write(src_dir.join("file1.txt"), "content1").unwrap();
        fs::write(src_dir.join("subdir").join("file2.txt"), "content2").unwrap();
        
        let src = LocalPath::parse(src_dir.to_str().unwrap()).unwrap();
        let dst = LocalPath::parse(dst_dir.to_str().unwrap()).unwrap();
        
        let result = copy_local(&src, &dst, false, false);
        assert!(result.is_ok());
        assert!(dst_dir.join("file1.txt").exists());
        assert!(dst_dir.join("subdir").join("file2.txt").exists());
        
        let content1 = fs::read_to_string(dst_dir.join("file1.txt")).unwrap();
        let content2 = fs::read_to_string(dst_dir.join("subdir").join("file2.txt")).unwrap();
        assert_eq!(content1, "content1");
        assert_eq!(content2, "content2");
    }

    #[test]
    fn test_copy_error_display() {
        let error = CopyError::SourceNotFound("test.txt".to_string());
        let display = format!("{}", error);
        assert!(display.contains("test.txt"));
        assert!(display.contains("not found"));
    }
}
