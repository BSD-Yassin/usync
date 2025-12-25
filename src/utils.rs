use std::fs;
use std::io;
use std::path::Path;

#[inline]
pub fn get_buffer_size(file_size: u64) -> usize {
    if file_size > 1_048_576 {
        64 * 1024
    } else {
        8 * 1024
    }
}

#[inline]
pub fn copy_file_buffered(src: &Path, dst: &Path) -> io::Result<u64> {
    copy_file_buffered_with_resume(src, dst, 0)
}

pub fn copy_file_buffered_with_resume(src: &Path, dst: &Path, resume_from: u64) -> io::Result<u64> {
    use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};

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
pub fn copy_file_sendfile(src: &Path, dst: &Path) -> io::Result<u64> {
    use std::os::unix::io::{AsRawFd, RawFd};

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

#[cfg(target_os = "macos")]
pub fn copy_file_range_macos(src: &Path, dst: &Path) -> io::Result<u64> {
    use std::ffi::CString;

    if let Some(parent) = dst.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    let file_size = fs::metadata(src)?.len();

    unsafe {
        extern "C" {
            fn copyfile(
                from: *const i8,
                to: *const i8,
                state: *mut std::ffi::c_void,
                flags: u32,
            ) -> i32;
        }

        let src_cstr = match CString::new(src.to_string_lossy().as_ref()) {
            Ok(s) => s,
            Err(_) => return copy_file_buffered(src, dst),
        };
        let dst_cstr = match CString::new(dst.to_string_lossy().as_ref()) {
            Ok(s) => s,
            Err(_) => return copy_file_buffered(src, dst),
        };

        let result = copyfile(
            src_cstr.as_ptr(),
            dst_cstr.as_ptr(),
            std::ptr::null_mut(),
            0x0001,
        );

        if result == 0 {
            if let Ok(dst_metadata) = fs::metadata(dst) {
                if dst_metadata.len() == file_size && file_size > 0 {
                    Ok(file_size)
                } else {
                    copy_file_buffered(src, dst)
                }
            } else {
                copy_file_buffered(src, dst)
            }
        } else {
            copy_file_buffered(src, dst)
        }
    }
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
pub fn copy_file_range_macos(_src: &Path, _dst: &Path) -> io::Result<u64> {
    copy_file_buffered(_src, _dst)
}

#[cfg(not(target_os = "linux"))]
#[allow(dead_code)]
pub fn copy_file_sendfile(src: &Path, dst: &Path) -> io::Result<u64> {
    copy_file_buffered(src, dst)
}

#[inline]
pub fn copy_file_via_ram(src: &Path, dst: &Path) -> io::Result<u64> {
    if let Some(parent) = dst.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    let data = fs::read(src)?;
    let file_size = data.len() as u64;

    fs::write(dst, &data)?;

    Ok(file_size)
}

#[allow(dead_code)]
#[inline]
pub fn get_file_size(path: &Path) -> io::Result<u64> {
    fs::metadata(path).map(|m| m.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_get_buffer_size() {
        assert_eq!(get_buffer_size(500_000), 8 * 1024);
        assert_eq!(get_buffer_size(2_000_000), 64 * 1024);
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
