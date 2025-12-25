mod backend;
mod copy;
mod filters;
mod operations;
mod path;
mod protocol;
mod remote;
mod utils;

use clap::Parser;

use copy::copy_with_options;
use protocol::parse_path;
use operations::sync::SyncStats;
use std::fs;

enum SyncResult {
    Copy(copy::CopyStats),
    Sync(SyncStats),
}

#[cfg(feature = "color")]
use colored::*;

#[derive(Parser, Debug)]
#[command(
    name = "usync",
    author = "Yassin Bousaâdi",
    version = "0.2.0",
    about = "A universal file copying and synchronization tool",
    long_about = r#"usync - Universal File Copying and Synchronization Tool

A simple, unified interface for copying files and directories locally and remotely.
Supports multiple protocols including SSH, SFTP, HTTP, and HTTPS.

FEATURES:
  • Local file operations (recursive directory copying)
  • SSH/SFTP via scp
  • HTTP/HTTPS downloads (via curl/wget)
  • Progress bars for file transfers
  • Adaptive buffer sizing for optimal performance
  • Zero-copy transfers on Linux (sendfile)
  • RAM-based copying for small files (--ram)
  • Move files instead of copying (--move)

EXAMPLES:
  # Copy a file locally
  usync source.txt destination.txt

  # Copy a directory recursively
  usync -r ./mydir/ ./dest/

  # Copy with progress
  usync -p largefile.txt ./backup/

  # Copy via RAM (faster for small files)
  usync --ram smallfile.txt ./backup/

  # Move file (removes source after copy)
  usync -m source.txt destination.txt

  # Copy from remote SSH
  usync ssh://user@host:/path/file.txt ./local.txt

  # Copy to remote SSH
  usync ./local.txt ssh://user@host:/path/file.txt

  # Download from HTTP/HTTPS
  usync https://example.com/file.txt ./downloaded.txt

  # Use SSH options
  usync -s "IdentityFile=~/.ssh/id_rsa" -s "StrictHostKeyChecking=no" \
        ssh://user@host:/path/file.txt ./local.txt

ENVIRONMENT VARIABLES:
  USYNC_VERBOSE    Enable verbose mode (any non-empty value)
  USYNC_SSH_OPTS   SSH options (space-separated)

For more information, visit: https://github.com/yassinbousaadi/usync"#,
    after_help = r#"EXAMPLES:
  Basic file copy:
    usync file.txt backup.txt

  Recursive directory copy:
    usync -r ./source/ ./destination/

  Copy with progress:
    usync -p largefile.dat ./backup/

  Copy via RAM:
    usync --ram smallfile.txt ./backup/

  Move file:
    usync -m source.txt destination.txt

  Remote SSH copy:
    usync ssh://user@host:/remote/file.txt ./local.txt

  HTTP download:
    usync https://example.com/file.zip ./downloads/

  Verbose mode:
    usync -v source.txt dest.txt

FEATURES:
  Enable progress bars: cargo build --features progress
  Enable colored output: cargo build --features color
  Enable SSH Rust library: cargo build --features ssh-rust"#
)]
struct Args {
    #[arg(value_name = "SOURCE")]
    src: String,

    #[arg(value_name = "DEST")]
    dst: String,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// SSH options to pass to scp (can be used multiple times)
    /// Example: --ssh-opt "IdentityFile=~/.ssh/id_rsa" --ssh-opt "StrictHostKeyChecking=no"
    #[arg(short = 's', long = "ssh-opt", value_name = "OPTION")]
    ssh_opts: Vec<String>,

    /// Copy directories recursively
    #[arg(short = 'r', long = "recursive", alias = "rec")]
    recursive: bool,

    /// Show progress during copy
    #[arg(short = 'p', long = "progress")]
    progress: bool,

    /// Copy via RAM (load entire file into memory first). Useful for small files or ensuring data integrity.
    /// Warning: Uses more memory, not recommended for very large files.
    #[arg(long = "ram", alias = "memory")]
    use_ram: bool,

    /// Move files instead of copying (removes source after successful copy)
    #[arg(short = 'm', long = "move")]
    move_files: bool,

    /// Verify file integrity using checksums after copy (MD5, SHA1, or SHA256)
    #[arg(long = "checksum", value_name = "ALGORITHM")]
    checksum: Option<String>,

    /// Dry-run mode: show what would be copied without actually copying
    #[arg(long = "dry-run")]
    dry_run: bool,

    /// Sync mode: only copy changed/new files (one-way sync)
    #[arg(long = "sync")]
    sync: bool,

    /// Include files matching pattern (glob, can be used multiple times)
    #[arg(long = "include", value_name = "PATTERN")]
    include: Vec<String>,

    /// Exclude files matching pattern (glob, can be used multiple times)
    #[arg(long = "exclude", value_name = "PATTERN")]
    exclude: Vec<String>,

    /// Minimum file size in bytes
    #[arg(long = "min-size", value_name = "BYTES")]
    min_size: Option<u64>,

    /// Maximum file size in bytes
    #[arg(long = "max-size", value_name = "BYTES")]
    max_size: Option<u64>,
}

fn main() {
    let args = Args::parse();

    let env_verbose = std::env::var("USYNC_VERBOSE")
        .map(|v| !v.is_empty() && v != "0" && v.to_lowercase() != "false")
        .unwrap_or(false);
    let verbose = args.verbose || env_verbose;

    let src_path = match parse_path(&args.src) {
        Ok(path) => path,
        Err(e) => {
            #[cfg(feature = "color")]
            eprintln!("{}: {}", "Error parsing source path".red().bold(), e);
            #[cfg(not(feature = "color"))]
            eprintln!("Error parsing source path: {}", e);
            std::process::exit(1);
        }
    };

    let dst_path = match parse_path(&args.dst) {
        Ok(path) => path,
        Err(e) => {
            #[cfg(feature = "color")]
            eprintln!("{}: {}", "Error parsing destination path".red().bold(), e);
            #[cfg(not(feature = "color"))]
            eprintln!("Error parsing destination path: {}", e);
            std::process::exit(1);
        }
    };

    let is_dir = match &src_path {
        protocol::Path::Local(local_path) => {
            if !local_path.exists() {
                #[cfg(feature = "color")]
                eprintln!(
                    "{}: {}",
                    "Error".red().bold(),
                    format!(
                        "Source path does not exist: {}",
                        local_path.to_string_lossy()
                    )
                );
                #[cfg(not(feature = "color"))]
                eprintln!(
                    "Error: Source path does not exist: {}",
                    local_path.to_string_lossy()
                );
                std::process::exit(1);
            }
            local_path.is_dir()
        }
        protocol::Path::Remote(_) => false,
    };

    if is_dir && !args.recursive {
        println!("Source is a directory. This will copy recursively.");
        print!("Continue? [y/N]: ");
        use std::io::{self, Write};
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let trimmed = input.trim().to_lowercase();
        if trimmed != "y" && trimmed != "yes" {
            if verbose {
                println!("Copy cancelled by user.");
            } else {
                println!("Copy cancelled.");
            }
            std::process::exit(0);
        }
    }

    let src_str = match &src_path {
        protocol::Path::Local(p) => p.to_string_lossy().to_string(),
        protocol::Path::Remote(r) => r.url.to_string(),
    };
    let dst_str = match &dst_path {
        protocol::Path::Local(p) => p.to_string_lossy().to_string(),
        protocol::Path::Remote(r) => r.url.to_string(),
    };

    let ssh_opts = if !args.ssh_opts.is_empty() {
        args.ssh_opts
    } else {
        std::env::var("USYNC_SSH_OPTS")
            .map(|v| v.split_whitespace().map(|s| s.to_string()).collect())
            .unwrap_or_default()
    };

    let checksum_algorithm = args.checksum.as_ref().and_then(|s| {
        match s.to_lowercase().as_str() {
            "md5" => Some(crate::backend::traits::ChecksumAlgorithm::Md5),
            "sha1" => Some(crate::backend::traits::ChecksumAlgorithm::Sha1),
            "sha256" => Some(crate::backend::traits::ChecksumAlgorithm::Sha256),
            _ => {
                eprintln!("Invalid checksum algorithm: {}. Use md5, sha1, or sha256", s);
                None
            }
        }
    });

    let dry_run = args.dry_run || std::env::var("USYNC_DRY_RUN")
        .map(|v| !v.is_empty() && v != "0" && v.to_lowercase() != "false")
        .unwrap_or(false);

    let env_progress = std::env::var("USYNC_PROGRESS")
        .map(|v| !v.is_empty() && v != "0" && v.to_lowercase() != "false")
        .unwrap_or(false);
    let show_progress = args.progress || env_progress;

    if dry_run {
        println!("[DRY RUN] Would {} {} to {}", 
            if args.move_files { "move" } else { "copy" },
            src_str, dst_str);
    }

    if verbose {
        if args.move_files {
            println!("Moving {} to {}...", src_str, dst_str);
        } else {
            println!("Copying {} to {}...", src_str, dst_str);
        }
    }

    let result: Result<SyncResult, copy::CopyError> = if args.sync {
        copy::sync_with_options(
            &src_path,
            &dst_path,
            verbose,
            &ssh_opts,
            show_progress,
            args.use_ram,
            checksum_algorithm,
            dry_run,
        ).map(|stats| SyncResult::Sync(stats))
    } else {
        copy::copy_with_options_and_filters(
            &src_path,
            &dst_path,
            verbose,
            &ssh_opts,
            show_progress,
            args.use_ram,
            checksum_algorithm,
            dry_run,
            filters,
        ).map(|stats| SyncResult::Copy(stats))
    };

    match result {
        Ok(SyncResult::Copy(_)) | Ok(SyncResult::Sync(_)) => {
            if args.move_files {
                match delete_source(&src_path, verbose) {
                    Ok(()) => {
                        if verbose {
                            #[cfg(feature = "color")]
                            println!(
                                "{} {} and removed source",
                                "✓".green().bold(),
                                if args.use_ram {
                                    "Moved via RAM"
                                } else {
                                    "Moved"
                                }
                            );
                            #[cfg(not(feature = "color"))]
                            println!("✓ Moved and removed source");
                        } else {
                            #[cfg(feature = "color")]
                            println!("{} {} to {}", "Moved".green(), src_str, dst_str);
                            #[cfg(not(feature = "color"))]
                            println!("Moved {} to {}", src_str, dst_str);
                        }
                    }
                    Err(e) => {
                        #[cfg(feature = "color")]
                        eprintln!(
                            "{}: Copy succeeded but failed to remove source: {}",
                            "Warning".yellow().bold(),
                            e
                        );
                        #[cfg(not(feature = "color"))]
                        eprintln!("Warning: Copy succeeded but failed to remove source: {}", e);
                    }
                }
            } else if verbose {
                #[cfg(feature = "color")]
                println!(
                    "{} {} to {}",
                    "✓".green().bold(),
                    "Successfully copied".green(),
                    format!("{} to {}", src_str, dst_str)
                );
                #[cfg(not(feature = "color"))]
                println!("✓ Successfully copied {} to {}", src_str, dst_str);
            } else {
                #[cfg(feature = "color")]
                println!(
                    "{} {} to {}",
                    "Successfully copied".green(),
                    src_str,
                    dst_str
                );
                #[cfg(not(feature = "color"))]
                println!("Successfully copied {} to {}", src_str, dst_str);
            }
            if verbose || show_progress {
                match &result {
                    Ok(SyncResult::Copy(stats)) => stats.print_summary(verbose),
                    Ok(SyncResult::Sync(stats)) => {
                        println!("\n=== Sync Summary ===");
                        println!("Files copied: {}", stats.files_copied);
                        println!("Bytes copied: {} ({:.2} MB)", 
                            stats.bytes_copied,
                            stats.bytes_copied as f64 / 1_048_576.0);
                        println!("Files deleted: {}", stats.files_deleted);
                    }
                    _ => {}
                }
            }
        }
        Err(e) => {
            #[cfg(feature = "color")]
            eprintln!(
                "{}: {}",
                if args.move_files {
                    "Error moving"
                } else {
                    "Error copying"
                }
                .red()
                .bold(),
                e
            );
            #[cfg(not(feature = "color"))]
            eprintln!(
                "{}: {}",
                if args.move_files {
                    "Error moving"
                } else {
                    "Error copying"
                },
                e
            );
            std::process::exit(1);
        }
    }
}

fn delete_source(path: &protocol::Path, verbose: bool) -> Result<(), String> {
    match path {
        protocol::Path::Local(local_path) => {
            let path = local_path.as_path();
            if path.is_dir() {
                if verbose {
                    println!("Removing directory and all contents: {}", path.display());
                }
                fs::remove_dir_all(path)
                    .map_err(|e| format!("Failed to remove directory {}: {}", path.display(), e))?;
                if verbose {
                    println!("Removed directory: {}", path.display());
                }
            } else {
                fs::remove_file(path)
                    .map_err(|e| format!("Failed to remove file {}: {}", path.display(), e))?;
                if verbose {
                    println!("Removed file: {}", path.display());
                }
            }
            Ok(())
        }
        protocol::Path::Remote(_) => Err(
            "Cannot remove remote files. Move operation only supported for local files."
                .to_string(),
        ),
    }
}
