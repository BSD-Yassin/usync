use std::fs;
use std::io;
use std::path::Path;

use crate::path::LocalPath;
use crate::protocol::Path as ProtocolPath;
use crate::remote;

pub fn copy(src: &ProtocolPath, dst: &ProtocolPath, verbose: bool, ssh_opts: &[String], progress: bool) -> Result<(), CopyError> {
    match (src, dst) {
        (ProtocolPath::Local(src_local), ProtocolPath::Local(dst_local)) => {
            copy_local(src_local, dst_local, verbose, progress)
        }
        (ProtocolPath::Remote(src_remote), ProtocolPath::Remote(dst_remote)) => {
            remote::copy_remote(src_remote, dst_remote, verbose, ssh_opts, progress)
                .map_err(|e| CopyError::RemoteError(e))
        }
        (ProtocolPath::Remote(src_remote), ProtocolPath::Local(dst_local)) => {
            copy_from_remote_to_local(src_remote, dst_local, verbose, ssh_opts, progress)
        }
        (ProtocolPath::Local(src_local), ProtocolPath::Remote(dst_remote)) => {
            copy_from_local_to_remote(src_local, dst_remote, verbose, ssh_opts, progress)
        }
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

pub fn copy_local(src: &LocalPath, dst: &LocalPath, verbose: bool, progress: bool) -> Result<(), CopyError> {
    if !src.exists() {
        return Err(CopyError::SourceNotFound(src.to_string_lossy().to_string()));
    }

    let src_path = src.as_path();
    let dst_path = dst.as_path();

    if src.is_file() {
        copy_file(src_path, dst_path, verbose, progress)
    } else if src.is_dir() {
        copy_directory(src_path, dst_path, verbose, progress)
    } else {
        Err(CopyError::InvalidSource(
            "Source path is neither a file nor a directory".to_string(),
        ))
    }
}

fn copy_file(src: &Path, dst: &Path, verbose: bool, progress: bool) -> Result<(), CopyError> {
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

    if progress || verbose {
        let src_size = fs::metadata(src)
            .map(|m| m.len())
            .unwrap_or(0);
        if progress {
            print!("Copying {} ({} bytes)... ", src.display(), src_size);
            use std::io::Write;
            io::stdout().flush().unwrap();
        } else {
            println!("Copying file: {} -> {}", src.display(), final_dst.display());
        }
    }
    fs::copy(src, &final_dst).map_err(|e| CopyError::IoError {
        message: format!(
            "Failed to copy file from {} to {}",
            src.display(),
            final_dst.display()
        ),
        error: e,
    })?;

    if progress {
        println!("✓");
    }

    Ok(())
}

fn copy_directory(src: &Path, dst: &Path, verbose: bool, progress: bool) -> Result<(), CopyError> {
    if !dst.exists() {
        if verbose {
            println!("Creating destination directory: {}", dst.display());
        }
        fs::create_dir_all(dst).map_err(|e| CopyError::IoError {
            message: format!("Failed to create destination directory: {}", dst.display()),
            error: e,
        })?;
    }

    copy_directory_recursive(src, dst, verbose, progress)?;

    Ok(())
}

fn copy_directory_recursive(src: &Path, dst: &Path, verbose: bool, progress: bool) -> Result<(), CopyError> {
    let entries = fs::read_dir(src).map_err(|e| CopyError::IoError {
        message: format!("Failed to read source directory: {}", src.display()),
        error: e,
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| CopyError::IoError {
            message: format!("Failed to read directory entry in: {}", src.display()),
            error: e,
        })?;

        let src_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        if src_path.is_dir() {
            if progress || verbose {
                if progress {
                    println!("Copying directory: {} -> {}", src_path.display(), dst_path.display());
                } else {
                    println!("Copying directory: {} -> {}", src_path.display(), dst_path.display());
                }
            }
            fs::create_dir_all(&dst_path).map_err(|e| CopyError::IoError {
                message: format!("Failed to create directory: {}", dst_path.display()),
                error: e,
            })?;
            copy_directory_recursive(&src_path, &dst_path, verbose, progress)?;
        } else {
            if progress || verbose {
                let file_size = fs::metadata(&src_path)
                    .map(|m| m.len())
                    .unwrap_or(0);
                if progress {
                    print!("  {} ({} bytes)... ", file_name.to_string_lossy(), file_size);
                    use std::io::Write;
                    io::stdout().flush().unwrap();
                } else {
                    println!("Copying file: {} -> {}", src_path.display(), dst_path.display());
                }
            }
            fs::copy(&src_path, &dst_path).map_err(|e| CopyError::IoError {
                message: format!(
                    "Failed to copy file from {} to {}",
                    src_path.display(),
                    dst_path.display()
                ),
                error: e,
            })?;
            if progress {
                println!("✓");
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
                write!(f, "Source path not found: {}", path)
            }
            CopyError::InvalidSource(msg) => {
                write!(f, "Invalid source: {}", msg)
            }
            CopyError::IoError { message, error } => {
                write!(f, "{}: {}", message, error)
            }
            CopyError::RemoteError(e) => {
                write!(f, "Remote copy error: {}", e)
            }
            CopyError::UnsupportedProtocol(msg) => {
                write!(f, "Unsupported protocol: {}", msg)
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
