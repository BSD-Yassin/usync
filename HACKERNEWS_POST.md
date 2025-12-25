# Hacker News Post

usync - A fast, unified file copy tool written in Rust [0]

Built in under a day with significant AI assistance, usync provides a single interface for copying files across local filesystem, SSH, S3, and HTTP/S protocols.

Benchmarks show usync often outperforms rsync and approaches cp speeds. The performance advantage comes from not automatically verifying file integrity after copy operations (unlike rsync), which makes sense for the speed gains. For critical transfers, users can manually verify checksums when needed.

Features include parallel directory processing, zero-copy transfers on Linux/macOS, and support for multiple protocols. The tool is designed for fast file transfers without the overhead of integrity checks on every operation.

[0] https://github.com/BSD-Yassin/usync
