mod path;
mod copy;
mod protocol;
mod remote;

use clap::Parser;

use protocol::parse_path;
use copy::copy;

#[derive(Parser, Debug)]
#[command(name = "usync", author = "Yassin Bousaâdi", version = "0.1", about = "A smaller attempt at universal file copying, synchronization", long_about = None)]
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
            eprintln!("Error parsing source path: {}", e);
            std::process::exit(1);
        }
    };

    let dst_path = match parse_path(&args.dst) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error parsing destination path: {}", e);
            std::process::exit(1);
        }
    };

    let is_dir = match &src_path {
        protocol::Path::Local(local_path) => {
            if !local_path.exists() {
                eprintln!("Error: Source path does not exist: {}", local_path.to_string_lossy());
                std::process::exit(1);
            }
            local_path.is_dir()
        }
        protocol::Path::Remote(_) => {
            false
        }
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

    let show_progress = args.progress || verbose;

    if verbose {
        println!("Copying {} to {}...", src_str, dst_str);
    }

    match copy(&src_path, &dst_path, verbose, &ssh_opts, show_progress) {
        Ok(()) => {
            if verbose {
                println!("✓ Successfully copied {} to {}", src_str, dst_str);
            } else {
                println!("Successfully copied {} to {}", src_str, dst_str);
            }
        }
        Err(e) => {
            eprintln!("Error copying: {}", e);
            std::process::exit(1);
        }
    }
}
