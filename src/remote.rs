use std::path::Path;
use std::process::Command;

use crate::protocol::{Protocol, RemotePath};

pub fn copy_remote(
    src: &RemotePath,
    dst: &RemotePath,
    verbose: bool,
    ssh_opts: &[String],
    progress: bool,
) -> Result<(), RemoteCopyError> {
    match (&src.protocol, &dst.protocol) {
        (Protocol::Ssh | Protocol::Sftp, Protocol::Ssh | Protocol::Sftp) => {
            copy_ssh_to_ssh(src, dst, verbose, ssh_opts, progress)
        }
        (Protocol::Ssh | Protocol::Sftp, _) => copy_from_ssh(src, dst, verbose),
        (_, Protocol::Ssh | Protocol::Sftp) => copy_to_ssh(src, dst, verbose),
        _ => Err(RemoteCopyError::UnsupportedProtocol {
            src: src.protocol.to_string(),
            dst: dst.protocol.to_string(),
        }),
    }
}

pub fn copy_from_ssh_to_file(
    src: &RemotePath,
    dst_path: &Path,
    verbose: bool,
    ssh_opts: &[String],
    progress: bool,
) -> Result<(), RemoteCopyError> {
    let host = src.url.host_str().ok_or_else(|| {
        RemoteCopyError::ConnectionError("No host specified in SSH URL".to_string())
    })?;

    let port = src.url.port().unwrap_or(22);
    let username = src.url.username();
    let remote_path = src.path.as_str();

    if verbose {
        println!("Connecting to SSH: {}@{}:{}", username, host, port);
        println!(
            "Copying from remote: {} to local: {}",
            remote_path,
            dst_path.display()
        );
    }

    if let Some(parent) = dst_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| RemoteCopyError::IoError {
            message: format!("Failed to create directory: {}", parent.display()),
            error: e.to_string(),
        })?;
    }

    let remote_spec = format!("{}@{}:{}", username, host, remote_path);

    let mut cmd = Command::new("scp");

    if port != 22 {
        cmd.arg("-P").arg(port.to_string());
    }

    if progress {
        cmd.arg("-v");
    } else if !verbose {
        cmd.arg("-q");
    }

    for opt in ssh_opts {
        cmd.arg("-o").arg(opt);
    }

    cmd.arg(&remote_spec).arg(dst_path);

    let status = cmd.status().map_err(|e| RemoteCopyError::IoError {
        message: "Failed to execute scp".to_string(),
        error: e.to_string(),
    })?;

    if status.success() {
        if verbose {
            println!("✓ Successfully copied from remote to local");
        }
        Ok(())
    } else {
        Err(RemoteCopyError::IoError {
            message: "scp failed to copy file".to_string(),
            error: format!("Exit code: {}", status.code().unwrap_or(-1)),
        })
    }
}

pub fn copy_from_ssh(
    src: &RemotePath,
    _dst: &RemotePath,
    verbose: bool,
) -> Result<(), RemoteCopyError> {
    if verbose {
        println!(
            "Connecting to SSH: {}://{}",
            src.protocol,
            src.url.host_str().unwrap_or("")
        );
    }

    Err(RemoteCopyError::NotImplemented(
        "Use copy_from_ssh_to_file for file operations".to_string(),
    ))
}

pub fn copy_file_to_ssh(
    src_path: &Path,
    dst: &RemotePath,
    verbose: bool,
    ssh_opts: &[String],
    progress: bool,
) -> Result<(), RemoteCopyError> {
    let host = dst.url.host_str().ok_or_else(|| {
        RemoteCopyError::ConnectionError("No host specified in SSH URL".to_string())
    })?;

    let port = dst.url.port().unwrap_or(22);
    let username = dst.url.username();
    let remote_path = dst.path.as_str();

    if verbose {
        println!("Connecting to SSH: {}@{}:{}", username, host, port);
        println!(
            "Copying from local: {} to remote: {}",
            src_path.display(),
            remote_path
        );
    }

    let remote_spec = format!("{}@{}:{}", username, host, remote_path);

    let mut cmd = Command::new("scp");

    if port != 22 {
        cmd.arg("-P").arg(port.to_string());
    }

    if progress {
        cmd.arg("-v");
    } else if !verbose {
        cmd.arg("-q");
    }

    for opt in ssh_opts {
        cmd.arg("-o").arg(opt);
    }

    cmd.arg(src_path).arg(&remote_spec);

    let status = cmd.status().map_err(|e| RemoteCopyError::IoError {
        message: "Failed to execute scp".to_string(),
        error: e.to_string(),
    })?;

    if status.success() {
        if verbose {
            println!("✓ Successfully copied from local to remote");
        }
        Ok(())
    } else {
        Err(RemoteCopyError::IoError {
            message: "scp failed to copy file".to_string(),
            error: format!("Exit code: {}", status.code().unwrap_or(-1)),
        })
    }
}

pub fn copy_to_ssh(
    _src: &RemotePath,
    dst: &RemotePath,
    verbose: bool,
) -> Result<(), RemoteCopyError> {
    if verbose {
        println!(
            "Connecting to SSH: {}://{}",
            dst.protocol,
            dst.url.host_str().unwrap_or("")
        );
    }

    Err(RemoteCopyError::NotImplemented(
        "Use copy_file_to_ssh for file operations".to_string(),
    ))
}

pub fn copy_ssh_to_ssh(
    src: &RemotePath,
    dst: &RemotePath,
    verbose: bool,
    ssh_opts: &[String],
    progress: bool,
) -> Result<(), RemoteCopyError> {
    if verbose {
        println!(
            "Copying from {}://{} to {}://{}",
            src.protocol,
            src.url.host_str().unwrap_or(""),
            dst.protocol,
            dst.url.host_str().unwrap_or("")
        );
    }

    Err(RemoteCopyError::NotImplemented(
        "SSH to SSH copy is not yet fully implemented".to_string(),
    ))
}

#[derive(Debug)]
pub enum RemoteCopyError {
    NotImplemented(String),
    UnsupportedProtocol { src: String, dst: String },
    ConnectionError(String),
    AuthenticationError(String),
    IoError { message: String, error: String },
}

impl std::fmt::Display for RemoteCopyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoteCopyError::NotImplemented(msg) => {
                write!(f, "Feature not yet implemented: {}", msg)
            }
            RemoteCopyError::UnsupportedProtocol { src, dst } => {
                write!(f, "Unsupported protocol combination: {} -> {}", src, dst)
            }
            RemoteCopyError::ConnectionError(msg) => {
                write!(f, "Connection error: {}", msg)
            }
            RemoteCopyError::AuthenticationError(msg) => {
                write!(f, "Authentication error: {}", msg)
            }
            RemoteCopyError::IoError { message, error } => {
                write!(f, "{}: {}", message, error)
            }
        }
    }
}

impl std::error::Error for RemoteCopyError {}

pub fn copy_from_http_to_file(
    src: &RemotePath,
    dst_path: &Path,
    verbose: bool,
    progress: bool,
) -> Result<(), RemoteCopyError> {
    let url = src.url.to_string();

    if verbose {
        println!("Downloading from {} to {}", url, dst_path.display());
    }

    if let Some(parent) = dst_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| RemoteCopyError::IoError {
            message: format!("Failed to create directory: {}", parent.display()),
            error: e.to_string(),
        })?;
    }

    if let Ok(mut cmd) = try_curl(&url, dst_path, verbose, progress) {
        let status = cmd.status().map_err(|e| RemoteCopyError::IoError {
            message: "Failed to execute curl".to_string(),
            error: e.to_string(),
        })?;

        if status.success() {
            if verbose {
                println!("✓ Successfully downloaded file");
            }
            return Ok(());
        } else {
            return Err(RemoteCopyError::IoError {
                message: "curl failed to download file".to_string(),
                error: format!("Exit code: {}", status.code().unwrap_or(-1)),
            });
        }
    }

    if let Ok(mut cmd) = try_wget(&url, dst_path, verbose, progress) {
        let status = cmd.status().map_err(|e| RemoteCopyError::IoError {
            message: "Failed to execute wget".to_string(),
            error: e.to_string(),
        })?;

        if status.success() {
            if verbose {
                println!("✓ Successfully downloaded file");
            }
            return Ok(());
        } else {
            return Err(RemoteCopyError::IoError {
                message: "wget failed to download file".to_string(),
                error: format!("Exit code: {}", status.code().unwrap_or(-1)),
            });
        }
    }

    Err(RemoteCopyError::IoError {
        message: "Neither curl nor wget found in PATH".to_string(),
        error: "Please install curl or wget to download HTTP/HTTPS files".to_string(),
    })
}

fn try_curl(url: &str, dst_path: &Path, verbose: bool, progress: bool) -> Result<Command, ()> {
    let check = Command::new("curl").arg("--version").output();

    if check.is_err() {
        return Err(());
    }

    let mut cmd = Command::new("curl");
    cmd.arg("-L").arg("-f").arg("-o").arg(dst_path).arg(url);

    if progress {
        cmd.arg("--progress-bar");
    } else if !verbose {
        cmd.arg("-s");
    }

    Ok(cmd)
}

fn try_wget(url: &str, dst_path: &Path, verbose: bool, progress: bool) -> Result<Command, ()> {
    let check = Command::new("wget").arg("--version").output();

    if check.is_err() {
        return Err(());
    }

    let mut cmd = Command::new("wget");
    cmd.arg("-O").arg(dst_path).arg(url);

    if progress {
        cmd.arg("--progress=bar");
    } else if !verbose {
        cmd.arg("-q");
    }

    Ok(cmd)
}
