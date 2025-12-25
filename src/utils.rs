use std::fs;
use std::io;
use std::path::Path;

/// Get the optimal buffer size based on file size
pub fn get_buffer_size(file_size: u64) -> usize {
    // Use 64KB for files larger than 1MB, 8KB for smaller files
    if file_size > 1_048_576 {
        64 * 1024
    } else {
        8 * 1024
    }
}

/// Copy file with adaptive buffer size
pub fn copy_file_buffered(src: &Path, dst: &Path) -> io::Result<u64> {
    copy_file_buffered_with_resume(src, dst, 0)
}

/// Copy file with adaptive buffer size and resume support
pub fn copy_file_buffered_with_resume(src: &Path, dst: &Path, resume_from: u64) -> io::Result<u64> {
    use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};

    // Ensure parent directory exists
    if let Some(parent) = dst.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut src_file = fs::File::open(src)?;
    let mut dst_file = if resume_from > 0 && dst.exists() {
        fs::OpenOptions::new().write(true).open(dst)?
    } else {
        fs::File::create(dst)?
    };

    if resume_from > 0 {
        src_file.seek(SeekFrom::Start(resume_from))?;
        dst_file.seek(SeekFrom::Start(resume_from))?;
    }

    // Get file size for adaptive buffer
    let file_size = src_file.metadata()?.len();
    let buffer_size = get_buffer_size(file_size);

    let mut reader = BufReader::with_capacity(buffer_size, &mut src_file);
    let mut writer = BufWriter::with_capacity(buffer_size, dst_file);

    let mut buffer = vec![0u8; buffer_size];
    let mut total = resume_from;

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        writer.write_all(&buffer[..bytes_read])?;
        total += bytes_read as u64;
    }

    writer.flush()?;
    Ok(total)
}

#[cfg(target_os = "linux")]
/// Zero-copy file transfer using sendfile (Linux only)
pub fn copy_file_sendfile(src: &Path, dst: &Path) -> io::Result<u64> {
    use std::os::unix::io::{AsRawFd, RawFd};

    // Ensure parent directory exists
    if let Some(parent) = dst.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    let src_file = fs::File::open(src)?;
    let dst_file = fs::File::create(dst)?;

    let src_fd: RawFd = src_file.as_raw_fd();
    let dst_fd: RawFd = dst_file.as_raw_fd();

    let file_size = src_file.metadata()?.len();
    let mut offset: i64 = 0;

    // Use sendfile for zero-copy transfer
    unsafe {
        extern "C" {
            fn sendfile(out_fd: i32, in_fd: i32, offset: *mut i64, count: usize) -> isize;
        }
        let result = sendfile(dst_fd, src_fd, &mut offset, file_size as usize);
        if result < 0 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(file_size)
}

#[cfg(not(target_os = "linux"))]
/// Fallback for non-Linux systems
pub fn copy_file_sendfile(src: &Path, dst: &Path) -> io::Result<u64> {
    copy_file_buffered(src, dst)
}

/// Copy file via RAM (load entire file into memory first)
/// This can be faster for small files and ensures data integrity
/// Warning: Uses memory equal to file size, not recommended for very large files
pub fn copy_file_via_ram(src: &Path, dst: &Path) -> io::Result<u64> {
    // Ensure parent directory exists
    if let Some(parent) = dst.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Read entire file into memory
    let data = fs::read(src)?;
    let file_size = data.len() as u64;

    // Write entire file from memory
    fs::write(dst, &data)?;

    Ok(file_size)
}

/// Get file size
#[allow(dead_code)]
pub fn get_file_size(path: &Path) -> io::Result<u64> {
    fs::metadata(path).map(|m| m.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_get_buffer_size() {
        assert_eq!(get_buffer_size(500_000), 8 * 1024); // Small file
        assert_eq!(get_buffer_size(2_000_000), 64 * 1024); // Large file
    }

    #[test]
    fn test_copy_file_buffered() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src.txt");
        let dst = temp_dir.path().join("dst.txt");

        fs::write(&src, "test content").unwrap();
        let result = copy_file_buffered(&src, &dst);

        assert!(result.is_ok());
        assert!(dst.exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "test content");
    }
}
