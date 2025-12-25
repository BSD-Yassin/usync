# usync Test Suite

This directory contains comprehensive tests for the usync tool.

## Test Structure

### Unit Tests (in source files)
- `src/path.rs` - Tests for local path parsing and validation
- `src/protocol.rs` - Tests for protocol detection and URL parsing
- `src/copy.rs` - Tests for local file copying functionality

### Integration Tests
- `test_runner.sh` - Main integration test suite (14 tests)
- `additional_tests.sh` - Extended test suite (10+ tests)
- `integration_tests.rs` - Rust integration tests (requires binary compilation)

## Running Tests

### Quick Test Run
```bash
./tests/test_runner.sh
```

### Full Test Suite
```bash
./tests/test_runner.sh
./tests/additional_tests.sh
```

### Using Debug Binary
```bash
./tests/test_runner.sh target/debug/usync
```

## Test Coverage

### Core Functionality
- ✅ Single file copying
- ✅ File to directory copying
- ✅ Recursive directory copying
- ✅ Parent directory creation
- ✅ File content verification
- ✅ Binary file handling
- ✅ Empty file handling
- ✅ Special characters in filenames

### Flags and Options
- ✅ Verbose mode (`-v`, `--verbose`)
- ✅ Progress mode (`-p`, `--progress`)
- ✅ Recursive flag (`-r`, `--recursive`, `--rec`)
- ✅ SSH options (`-s`, `--ssh-opt`)
- ✅ Short flag combinations
- ✅ Environment variable support

### Error Handling
- ✅ Nonexistent source files
- ✅ Protocol validation
- ✅ Invalid path handling

### Protocol Support
- ✅ HTTP/HTTPS URL parsing
- ✅ SSH/SFTP URL parsing
- ✅ SSH-style path parsing (`user@host:path`)
- ✅ Local path validation

### Edge Cases
- ✅ Large directory structures (50+ files)
- ✅ Nested directories (multiple levels)
- ✅ Files with spaces in names
- ✅ Binary file preservation

## Test Results

Run the test suites to see current pass/fail status. Most tests should pass when:
- Binary is built (`cargo build --release`)
- Test input files exist in `test/input/`
- Required system tools are available (curl/wget for HTTP, scp for SSH)

## Adding New Tests

1. Add unit tests in the relevant `src/*.rs` file under `#[cfg(test)]`
2. Add integration tests to `test_runner.sh` or `additional_tests.sh`
3. For Rust integration tests, add to `integration_tests.rs`

