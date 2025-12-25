# usync

A universal file copying and synchronization tool written in Rust. `usync` provides a simple, unified interface for copying files and directories locally and remotely, supporting multiple protocols including SSH, SFTP, HTTP, and HTTPS.

## Features

- **Local File Operations**: Copy files and directories recursively
- **Remote Protocol Support**: 
  - SSH/SFTP via `scp`
  - HTTP/HTTPS via `curl` or `wget`
- **Flexible Options**:
  - Recursive directory copying (`-r`, `--recursive`)
  - Verbose output (`-v`, `--verbose`)
  - Progress display (`-p`, `--progress`)
  - SSH options support (`-s`, `--ssh-opt`)
- **Cross-Platform**: Works on Unix-like systems (Linux, macOS, BSD)

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
  -h, --help              Print help
  -V, --version           Print version
```

### Environment Variables

- `USYNC_VERBOSE`: Enable verbose mode (any non-empty value)
- `USYNC_SSH_OPTS`: SSH options (space-separated, e.g., `"IdentityFile=~/.ssh/id_rsa StrictHostKeyChecking=no"`)

## Requirements

- Rust 1.70+ (for building from source)
- `scp` (for SSH/SFTP operations)
- `curl` or `wget` (for HTTP/HTTPS downloads)

## Development

### Building

```bash
cargo build
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
│   ├── main.rs       # CLI interface
│   ├── path.rs       # Local path parsing and validation
│   ├── protocol.rs   # Protocol detection and URL parsing
│   ├── copy.rs       # Local file copying
│   └── remote.rs     # Remote protocol implementations
├── tests/            # Integration tests
└── test/             # Test data
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

