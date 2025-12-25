# Performance Comparisons

This directory contains performance benchmarks comparing `usync` against standard Unix tools (`cp`, `rsync`, `scp`).

## System Information

**OS:** Darwin  
**Architecture:** arm64  
**Kernel:** 25.1.0  
**CPU Cores:** 8  
**CPU Model:** Apple M3  
**Memory:** 16.00 GB

## Running Benchmarks

```bash
# Run all benchmarks
./comparisons/bench.sh [binary_path] [remote_host]

# Example with default binary
./comparisons/bench.sh

# Example with custom binary and remote host
./comparisons/bench.sh target/release/usync user@remote.example.com
```

## Benchmark Tests

1. **Single Large File Copy** (100 MB)
   - Compares: `cp`, `rsync`, `usync` (regular), `usync` (RAM)

2. **Directory Copy** (100 files, 100 MB total)
   - Compares: `cp -r`, `rsync`, `usync -r` (regular), `usync -r` (RAM)

3. **Nested Directory Structure** (30 files across 3 levels)
   - Compares: `cp -r`, `rsync`, `usync -r` (regular), `usync -r` (RAM)

4. **Remote File Copy** (SSH, optional)
   - Compares: `scp`, `rsync`, `usync` (regular)
   - Requires SSH access to remote host

## Results

Results are saved to `comparisons/results/results_YYYYMMDD_HHMMSS.txt` with:
- System information
- Timestamp
- Binary path
- Detailed timing and speed measurements for each test

## Performance Optimizations

`usync` uses several performance optimizations:

- **Parallel directory processing**: Directories are processed in parallel while files within each directory are handled sequentially to avoid directory contention (inspired by [Fast Unix Commands (FUC)](https://alexsaveau.dev/blog/projects/performance/files/fuc/fast-unix-commands))
- **Zero-copy transfers**: Uses `sendfile()` on Linux and `copyfile()` on macOS for efficient file copying
- **Adaptive buffer sizing**: Automatically adjusts buffer size based on file size
- **RAM-based copying**: Optional in-memory copying for small files (`--ram` flag)

## References

- [Fast Unix Commands (FUC)](https://alexsaveau.dev/blog/projects/performance/files/fuc/fast-unix-commands) - Research on optimizing Unix commands, particularly directory-level parallelization strategies
- [FUC Comparisons](https://github.com/SUPERCILEX/fuc/tree/master/comparisons) - Benchmarking methodology reference

