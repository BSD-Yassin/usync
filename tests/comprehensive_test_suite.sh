#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

BINARY="${BINARY:-target/debug/usync}"
TEST_DIR="${TEST_DIR:-/tmp/usync_comprehensive_test}"
TEST_SRC="$TEST_DIR/src"
TEST_DST="$TEST_DIR/dst"

cleanup() {
    rm -rf "$TEST_DIR"
}
trap cleanup EXIT

setup_test_env() {
    rm -rf "$TEST_DIR"
    mkdir -p "$TEST_SRC" "$TEST_DST"
    
    echo "=== Setting up test environment ==="
    
    mkdir -p "$TEST_SRC/subdir"
    mkdir -p "$TEST_SRC/empty_dir"
    
    echo "test content 1" > "$TEST_SRC/file1.txt"
    echo "test content 2" > "$TEST_SRC/file2.txt"
    echo "test content 3" > "$TEST_SRC/subdir/file3.txt"
    echo "test content 4" > "$TEST_SRC/subdir/file4.txt"
    
    dd if=/dev/urandom of="$TEST_SRC/large_file.bin" bs=1024 count=100 2>/dev/null
    
    echo "✓ Test environment setup complete"
}

test_local_copy_file() {
    echo "=== Test: Local file copy ==="
    "$BINARY" "$TEST_SRC/file1.txt" "$TEST_DST/file1_copy.txt"
    [ -f "$TEST_DST/file1_copy.txt" ] || { echo "✗ File copy failed"; exit 1; }
    [ "$(cat "$TEST_SRC/file1.txt")" = "$(cat "$TEST_DST/file1_copy.txt")" ] || { echo "✗ File content mismatch"; exit 1; }
    echo "✓ Local file copy passed"
}

test_local_copy_directory() {
    echo "=== Test: Local directory copy (recursive) ==="
    "$BINARY" -r "$TEST_SRC" "$TEST_DST/recursive_copy"
    [ -f "$TEST_DST/recursive_copy/file1.txt" ] || { echo "✗ Directory copy failed"; exit 1; }
    [ -f "$TEST_DST/recursive_copy/subdir/file3.txt" ] || { echo "✗ Recursive copy failed"; exit 1; }
    echo "✓ Local directory copy passed"
}

test_checksum_md5() {
    echo "=== Test: Checksum verification (MD5) ==="
    "$BINARY" --checksum md5 "$TEST_SRC/file1.txt" "$TEST_DST/file1_md5.txt"
    [ -f "$TEST_DST/file1_md5.txt" ] || { echo "✗ Checksum copy failed"; exit 1; }
    echo "✓ MD5 checksum verification passed"
}

test_checksum_sha1() {
    echo "=== Test: Checksum verification (SHA1) ==="
    "$BINARY" --checksum sha1 "$TEST_SRC/file1.txt" "$TEST_DST/file1_sha1.txt"
    [ -f "$TEST_DST/file1_sha1.txt" ] || { echo "✗ SHA1 checksum copy failed"; exit 1; }
    echo "✓ SHA1 checksum verification passed"
}

test_checksum_sha256() {
    echo "=== Test: Checksum verification (SHA256) ==="
    "$BINARY" --checksum sha256 "$TEST_SRC/file1.txt" "$TEST_DST/file1_sha256.txt"
    [ -f "$TEST_DST/file1_sha256.txt" ] || { echo "✗ SHA256 checksum copy failed"; exit 1; }
    echo "✓ SHA256 checksum verification passed"
}

test_dry_run() {
    echo "=== Test: Dry-run mode ==="
    rm -rf "$TEST_DST/dry_run_test"
    output=$("$BINARY" --dry-run "$TEST_SRC/file1.txt" "$TEST_DST/dry_run_test/file1.txt" 2>&1)
    [ ! -f "$TEST_DST/dry_run_test/file1.txt" ] || { echo "✗ Dry-run actually copied file"; exit 1; }
    echo "$output" | grep -q "DRY RUN" || { echo "✗ Dry-run output missing"; exit 1; }
    echo "✓ Dry-run mode passed"
}

test_verbose() {
    echo "=== Test: Verbose mode ==="
    output=$("$BINARY" -v "$TEST_SRC/file1.txt" "$TEST_DST/file1_verbose.txt" 2>&1)
    [ -f "$TEST_DST/file1_verbose.txt" ] || { echo "✗ Verbose copy failed"; exit 1; }
    echo "$output" | grep -q "Copying" || { echo "✗ Verbose output missing"; exit 1; }
    echo "✓ Verbose mode passed"
}

test_progress() {
    echo "=== Test: Progress mode ==="
    "$BINARY" -p "$TEST_SRC/large_file.bin" "$TEST_DST/large_file_progress.bin"
    [ -f "$TEST_DST/large_file_progress.bin" ] || { echo "✗ Progress copy failed"; exit 1; }
    [ "$(stat -f%z "$TEST_SRC/large_file.bin" 2>/dev/null || stat -c%s "$TEST_SRC/large_file.bin")" = "$(stat -f%z "$TEST_DST/large_file_progress.bin" 2>/dev/null || stat -c%s "$TEST_DST/large_file_progress.bin")" ] || { echo "✗ File size mismatch"; exit 1; }
    echo "✓ Progress mode passed"
}

test_ram_copy() {
    echo "=== Test: RAM copy mode ==="
    "$BINARY" --ram "$TEST_SRC/file1.txt" "$TEST_DST/file1_ram.txt"
    [ -f "$TEST_DST/file1_ram.txt" ] || { echo "✗ RAM copy failed"; exit 1; }
    [ "$(cat "$TEST_SRC/file1.txt")" = "$(cat "$TEST_DST/file1_ram.txt")" ] || { echo "✗ RAM copy content mismatch"; exit 1; }
    echo "✓ RAM copy mode passed"
}

test_move() {
    echo "=== Test: Move operation ==="
    echo "move test content" > "$TEST_SRC/move_test.txt"
    "$BINARY" -m "$TEST_SRC/move_test.txt" "$TEST_DST/move_test.txt"
    [ -f "$TEST_DST/move_test.txt" ] || { echo "✗ Move destination missing"; exit 1; }
    [ ! -f "$TEST_SRC/move_test.txt" ] || { echo "✗ Move source still exists"; exit 1; }
    echo "✓ Move operation passed"
}

test_sync_one_way() {
    echo "=== Test: Sync mode (one-way) ==="
    mkdir -p "$TEST_DST/sync_test"
    echo "old content" > "$TEST_DST/sync_test/existing.txt"
    echo "new content" > "$TEST_SRC/new_file.txt"
    
    "$BINARY" --sync "$TEST_SRC" "$TEST_DST/sync_test"
    
    [ -f "$TEST_DST/sync_test/new_file.txt" ] || { echo "✗ Sync didn't copy new file"; exit 1; }
    echo "✓ Sync mode passed"
}

test_filter_include() {
    echo "=== Test: Filter include pattern ==="
    rm -rf "$TEST_DST/filter_test"
    "$BINARY" -r --include "*.txt" "$TEST_SRC" "$TEST_DST/filter_test"
    [ -f "$TEST_DST/filter_test/file1.txt" ] || { echo "✗ Include filter failed"; exit 1; }
    [ ! -f "$TEST_DST/filter_test/large_file.bin" ] || { echo "✗ Include filter didn't exclude binary"; exit 1; }
    echo "✓ Include filter passed"
}

test_filter_exclude() {
    echo "=== Test: Filter exclude pattern ==="
    rm -rf "$TEST_DST/filter_exclude_test"
    "$BINARY" -r --exclude "*.bin" "$TEST_SRC" "$TEST_DST/filter_exclude_test"
    [ -f "$TEST_DST/filter_exclude_test/file1.txt" ] || { echo "✗ Exclude filter failed"; exit 1; }
    [ ! -f "$TEST_DST/filter_exclude_test/large_file.bin" ] || { echo "✗ Exclude filter didn't work"; exit 1; }
    echo "✓ Exclude filter passed"
}

test_filter_size() {
    echo "=== Test: Filter by size ==="
    rm -rf "$TEST_DST/filter_size_test"
    "$BINARY" -r --min-size 1000 "$TEST_SRC" "$TEST_DST/filter_size_test"
    [ -f "$TEST_DST/filter_size_test/large_file.bin" ] || { echo "✗ Size filter failed"; exit 1; }
    echo "✓ Size filter passed"
}

test_error_handling() {
    echo "=== Test: Error handling ==="
    if "$BINARY" /nonexistent/file "$TEST_DST/error_test.txt" 2>&1 | grep -q "not found"; then
        echo "✓ Error handling for missing source passed"
    else
        echo "✗ Error handling failed"
        exit 1
    fi
}

test_combinations() {
    echo "=== Test: Flag combinations ==="
    
    rm -rf "$TEST_DST/combo_test"
    "$BINARY" -rvp --checksum sha256 "$TEST_SRC/file1.txt" "$TEST_DST/combo_test/file1.txt"
    [ -f "$TEST_DST/combo_test/file1.txt" ] || { echo "✗ Combination test failed"; exit 1; }
    
    echo "✓ Flag combinations passed"
}

run_all_tests() {
    echo "=========================================="
    echo "  Comprehensive Test Suite for usync"
    echo "=========================================="
    echo ""
    
    setup_test_env
    
    test_local_copy_file
    test_local_copy_directory
    test_checksum_md5
    test_checksum_sha1
    test_checksum_sha256
    test_dry_run
    test_verbose
    test_progress
    test_ram_copy
    test_move
    test_sync_one_way
    test_filter_include
    test_filter_exclude
    test_filter_size
    test_error_handling
    test_combinations
    
    echo ""
    echo "=========================================="
    echo "  All tests passed! ✓"
    echo "=========================================="
}

if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
    echo "Usage: $0"
    echo ""
    echo "Runs comprehensive tests for all usync functionality"
    exit 0
fi

run_all_tests

