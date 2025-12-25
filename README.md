# usync

A universal file copying and synchronization tool written in Rust. `usync` provides a simple, unified interface for copying files and directories locally and remotely, supporting multiple protocols including SSH, SFTP, HTTP, and HTTPS.

u stood for universal, not ony in the platform or os that I would use but rather on the file transfert mecanism. I wanted to incorporate cloud services, general remote methods and in the future, less common ones. The methodology is to rely on the original tool instead of recreating the wheel or using less central tooling (prefer a client rather than an sdk, prefer a client rather than a module for the protocol ), but it could change based on how compatible this makes the tool, or maintenable. 
sync does echo to the good old rsync, but I also wanted to emphasize continous file synchronization in the possibilities. 


## Features

- **Local File Operations**: Copy files and directories recursively
- **Remote Protocol Support**: 
  - SSH/SFTP via `scp`
  - HTTP/HTTPS via `curl` or `wget`
- **Performance Optimizations**:
  - RAM-based copying for small files (`--ram`)
  - Zero-copy transfers on Linux (automatic)
  - Adaptive buffer sizing
  - Parallel processing (with `parallel` feature)
- **Flexible Options**:
  - Recursive directory copying (`-r`, `--recursive`)
  - Verbose output (`-v`, `--verbose`)
  - Progress display (`-p`, `--progress`)
  - SSH options support (`-s`, `--ssh-opt`)
  - Move files instead of copying (`-m`, `--move`)
- **Cloud Services** (via CLI tools):
  - AWS S3 via `aws s3 cp` and `aws s3 sync` (fully tested and supported)
  - Other cloud storage via their respective CLI tools (see [Cloud Services](#cloud-services) below)
- **Experimental Features**:
  - Continuous synchronization daemon (see [Daemon Mode](#daemon-mode-experimental) below)
- **Cross-Platform**: Works on Unix-like systems (Linux, macOS, BSD) and Windows (experimental)

## Experimental and Untested Features

**⚠️ Important**: The following features are experimental, placeholder implementations, or not yet fully tested. Suggestions, fixes, and contributions are welcome!

### Windows Builds
- Windows builds are **experimental** and not fully tested
- Windows compilation is currently commented out in CI/CD workflows
- Contributions to improve Windows support are welcome

### Cloud Storage Providers (Placeholder/Experimental)

The following cloud storage providers have placeholder implementations but are **not tested**:

- **Azure Blob Storage**: Placeholder implementation via `az storage` CLI
  - Not tested in production
  - Suggestions and fixes welcome
  - Will use Azure CLI when fully implemented

- **Google Cloud Storage**: Placeholder implementation via `gsutil` CLI
  - Not tested in production
  - Suggestions and fixes welcome
  - Will use Google Cloud SDK when fully implemented

### Planned Cloud Storage Providers

The following cloud storage providers are planned but **not yet implemented**:

- **Google Drive**: Planned, will be an optional feature (not in default build)
- **Dropbox**: Planned, will be an optional feature (not in default build)
- **SharePoint**: Planned, will be an optional feature (not in default build)

These will follow the project's methodology of using native CLI tools rather than SDKs when implemented. Contributions for these features are welcome!

## Installation

### From Source

```bash
git clone https://github.com/YOUR_USERNAME/usync.git
cd usync
cargo build --release
```

The binary will be available at `target/release/usync`.

## Usage

### Basic Examples

```bash
# Copy a file
usync source.txt destination.txt

# Copy a directory recursively
usync -r ./mydir/ ./dest/

# Copy with progress
usync -p largefile.txt ./backup/

# Copy from remote SSH
usync ssh://user@host:/path/file.txt ./local.txt

# Copy to remote SSH
usync ./local.txt ssh://user@host:/path/file.txt

# Download from HTTP/HTTPS
usync https://example.com/file.txt ./downloaded.txt

# Use SSH options
usync -s "IdentityFile=~/.ssh/id_rsa" -s "StrictHostKeyChecking=no" \
      ssh://user@host:/path/file.txt ./local.txt
```

### Command-Line Options

```
Options:
  -v, --verbose           Enable verbose output
  -s, --ssh-opt <OPTION>  SSH options to pass to scp (can be used multiple times)
  -r, --recursive         Copy directories recursively (skips confirmation)
  -p, --progress          Show progress during copy
  --ram, --memory         Copy via RAM (faster for small files, uses more memory)
  -m, --move              Move files instead of copying (removes source after copy)
  -h, --help              Print help
  -V, --version           Print version
```

### Environment Variables

usync supports several environment variables for configuration:

- **`USYNC_VERBOSE`**: Enable verbose mode (any non-empty value)
  ```bash
  export USYNC_VERBOSE=1
  # or
  export USYNC_VERBOSE=true
  ```

- **`USYNC_SSH_OPTS`**: SSH options (space-separated)
  ```bash
  export USYNC_SSH_OPTS="IdentityFile=~/.ssh/id_rsa StrictHostKeyChecking=no"
  ```

- **`USYNC_CONFIG_PATH`** (planned): Path to configuration file
  ```bash
  export USYNC_CONFIG_PATH=~/.config/usync/config.toml
  ```

- **`USYNC_LOG_LEVEL`** (planned): Logging level (debug, info, warn, error)
  ```bash
  export USYNC_LOG_LEVEL=info
  ```

See `env.example` for a complete list of available environment variables.

### Configuration File

usync supports configuration via a TOML file (experimental). The configuration file can be specified via the `USYNC_CONFIG_PATH` environment variable or will be searched in standard locations:

- `~/.config/usync/config.toml`
- `~/.usync/config.toml`
- `./usync.toml` (current directory)

**Example configuration file** (`~/.config/usync/config.toml`):

```toml
[defaults]
verbose = false
progress = true
recursive = false

[ssh]
default_opts = [
    "IdentityFile=~/.ssh/id_rsa",
    "StrictHostKeyChecking=no"
]

[cloud]
# AWS S3 configuration
aws_profile = "default"
aws_region = "us-east-1"

# Cloud storage endpoints
s3_endpoint = "https://s3.amazonaws.com"

[daemon]
# Daemon mode settings (experimental)
enabled = false
watch_directories = [
    "~/Documents/sync",
    "~/Projects"
]
sync_interval = 5  # seconds
```

### Cloud Services

usync can interact with cloud storage services through their native CLI tools:

#### AWS S3

usync leverages the AWS CLI for S3 operations:

```bash
# Copy from S3 (requires AWS CLI and credentials)
usync s3://my-bucket/path/file.txt ./local-file.txt

# Copy to S3
usync ./local-file.txt s3://my-bucket/path/file.txt

# Use specific AWS profile
export AWS_PROFILE=my-profile
usync s3://bucket/file.txt ./local.txt
```

**Requirements:**
- AWS CLI installed (`aws --version`)
- AWS credentials configured (`aws configure` or environment variables)
- Appropriate IAM permissions for S3 access

**Supported S3 operations:**
- File copying (`aws s3 cp`)
- Directory syncing (`aws s3 sync`)

#### Other Cloud Services

**⚠️ Experimental/Untested**: The following cloud storage providers have placeholder implementations but are **not tested**:

- **Google Cloud Storage** (via `gsutil`): Placeholder implementation, not tested. Suggestions and fixes welcome.
- **Azure Blob Storage** (via `az storage`): Placeholder implementation, not tested. Suggestions and fixes welcome.
- **Other S3-compatible services**: Should work with any S3-compatible service via AWS CLI

**Planned Cloud Storage Providers** (not yet implemented, will be optional features):
- Google Drive (via `gdrive` CLI or similar)
- Dropbox (via `dropbox` CLI or similar)
- SharePoint (via `sharepoint` CLI or similar)

The methodology is to rely on native CLI tools rather than SDKs for better compatibility and maintainability. Contributions for implementing these providers are welcome!

### Daemon Mode (Experimental)

usync includes an experimental daemon mode for continuous file synchronization. This feature allows you to monitor directories and automatically sync changes.

**⚠️ Warning**: Daemon mode is experimental and may have stability issues. Use with caution.

#### Building with Daemon Support

```bash
cargo build --release --features daemon
```

#### Configuration

Configure the daemon via the configuration file:

```toml
[daemon]
enabled = true
watch_directories = [
    "~/Documents/sync",
    "~/Projects/important"
]
sync_interval = 5  # seconds between sync checks
log_file = "~/.local/share/usync/daemon.log"
pid_file = "~/.local/share/usync/daemon.pid"

[sync_rules]
# Define sync rules
[[sync_rules.rule]]
source = "~/Documents/sync"
destination = "ssh://user@host:/backup/sync"
recursive = true
```

#### Starting the Daemon

```bash
# Start daemon (requires daemon feature)
usync --daemon start

# Stop daemon
usync --daemon stop

# Check daemon status
usync --daemon status

# View daemon logs
tail -f ~/.local/share/usync/daemon.log
```

#### How It Works

The daemon monitors specified directories for changes using file system events (inotify on Linux, FSEvents on macOS). When changes are detected, it automatically synchronizes files according to the configured rules.

**Current Limitations:**
- Linux and macOS only (requires platform-specific file watching)
- Single-direction sync (source → destination)
- No conflict resolution
- Experimental status - may have bugs

**Future Improvements:**
- Bidirectional synchronization
- Conflict resolution strategies
- Better error handling and recovery
- Systemd integration for Linux

## Requirements

### Runtime Dependencies

- `scp` (for SSH/SFTP operations)
- `curl` or `wget` (for HTTP/HTTPS downloads)
- `aws` CLI (for S3/cloud operations, optional)

### Build Requirements

- Rust 1.70+ (for building from source)
- Cargo (Rust package manager)

### Optional Dependencies

- `aws` CLI for S3 support
- `gsutil` for Google Cloud Storage support
- `az` CLI for Azure Blob Storage support

## Development

### Nix Development Environment

usync includes Nix development environment files (`shell.nix` and `flake.nix`) for easy setup:

```bash
# Using nix-shell (traditional)
nix-shell

# Using nix develop (flakes)
nix develop

# Both provide:
# - Rust toolchain (latest stable)
# - Cargo
# - All build dependencies
# - Development tools
```

The Nix files are tracked in git to ensure a consistent development environment across different systems. All dependencies are automatically provided by Nix.

### Building

```bash
# Standard build
cargo build --release

# With optional features
cargo build --release --features progress,color

# With daemon support (experimental)
cargo build --release --features daemon

# With all features
cargo build --release --features progress,color,parallel,ssh-rust,daemon
```

### Available Features

Build usync with optional features for enhanced functionality:

- **`progress`**: Progress bars for file transfers (requires `indicatif`)
- **`color`**: Colored terminal output (requires `colored`)
- **`parallel`**: Parallel directory processing (requires `rayon`)
- **`ssh-rust`**: Native Rust SSH implementation (requires `ssh2`, alternative to `scp`)
- **`daemon`**: Daemon mode for continuous synchronization (experimental)
- **`logging`**: Enhanced logging capabilities

Enable features during build:
```bash
cargo build --release --features progress,color,parallel
```

### Running Tests

```bash
# Run unit tests
cargo test --lib

# Run integration tests
./tests/test_runner.sh
./tests/additional_tests.sh
```

### Project Structure

```
usync/
├── src/
│   ├── main.rs       # CLI interface and argument parsing
│   ├── path.rs       # Local path parsing and validation
│   ├── protocol.rs   # Protocol detection and URL parsing
│   ├── copy.rs       # Local file copying with optimizations
│   ├── remote.rs     # Remote protocol implementations
│   └── utils.rs      # Utility functions (buffering, sendfile, etc.)
├── tests/            # Integration tests and test data
│   ├── input/        # Test input files
│   ├── output/       # Test output directory
│   ├── fixtures/     # Test fixtures
│   └── test_runner.sh # Main test script
├── .github/workflows/ # CI/CD workflows
├── shell.nix         # Nix development environment (traditional)
├── flake.nix         # Nix development environment (flakes)
├── env.example       # Example environment variables
└── Cargo.toml        # Project manifest and dependencies
```

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### AI-Assisted Development

This project uses AI-assisted development tools. All AI-generated code is reviewed and accepted by human developers. Changes go through code review before being merged.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Author

**Yassin Bousaâdi**

## Acknowledgments

- Built with [clap](https://github.com/clap-rs/clap) for CLI parsing
- Uses standard Unix tools (`scp`, `curl`, `wget`) for remote operations
