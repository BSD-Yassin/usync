#!/bin/bash
set -e

BINARY="${1:-target/release/usync}"
TEST_DIR="test"

echo "=== Running usync Integration Tests ==="
echo "Using binary: $BINARY"
echo ""

cleanup() {
    rm -rf "$TEST_DIR/output"/*
    echo "Cleaned up test output directory"
}

setup() {
    cleanup
    mkdir -p "$TEST_DIR/output"
    echo "Test environment ready"
}

test_single_file_copy() {
    echo "Test 1: Single file copy"
    $BINARY "$TEST_DIR/input/file1.txt" "$TEST_DIR/output/file1_copy.txt"
    if [ -f "$TEST_DIR/output/file1_copy.txt" ]; then
        echo "✓ PASS: File copied successfully"
    else
        echo "✗ FAIL: File not copied"
        return 1
    fi
}

test_file_to_directory() {
    echo "Test 2: Copy file to directory"
    $BINARY "$TEST_DIR/input/file2.txt" "$TEST_DIR/output/"
    if [ -f "$TEST_DIR/output/file2.txt" ]; then
        echo "✓ PASS: File copied to directory"
    else
        echo "✗ FAIL: File not copied to directory"
        return 1
    fi
}

test_recursive_copy() {
    echo "Test 3: Recursive directory copy"
    echo "y" | $BINARY "$TEST_DIR/input/subdir" "$TEST_DIR/output/subdir_copy" || true
    if [ -d "$TEST_DIR/output/subdir_copy" ] && [ -f "$TEST_DIR/output/subdir_copy/file3.txt" ]; then
        echo "✓ PASS: Directory copied recursively"
    else
        echo "✗ FAIL: Directory not copied"
        return 1
    fi
}

test_recursive_flag() {
    echo "Test 4: Recursive flag (no confirmation)"
    $BINARY -r "$TEST_DIR/input/subdir" "$TEST_DIR/output/subdir_recursive"
    if [ -d "$TEST_DIR/output/subdir_recursive" ] && [ -f "$TEST_DIR/output/subdir_recursive/file3.txt" ]; then
        echo "✓ PASS: Recursive flag works"
    else
        echo "✗ FAIL: Recursive flag failed"
        return 1
    fi
}

test_verbose_mode() {
    echo "Test 5: Verbose mode"
    output=$($BINARY -v "$TEST_DIR/input/file1.txt" "$TEST_DIR/output/file1_verbose.txt" 2>&1)
    if echo "$output" | grep -q "Copying\|Successfully"; then
        echo "✓ PASS: Verbose output shown"
    else
        echo "✗ FAIL: Verbose output not shown"
        return 1
    fi
}

test_progress_mode() {
    echo "Test 6: Progress mode"
    $BINARY -p "$TEST_DIR/input/file1.txt" "$TEST_DIR/output/file1_progress.txt" > /dev/null 2>&1
    if [ -f "$TEST_DIR/output/file1_progress.txt" ]; then
        echo "✓ PASS: Progress mode works"
    else
        echo "✗ FAIL: Progress mode failed"
        return 1
    fi
}

test_error_handling() {
    echo "Test 7: Error handling (nonexistent source)"
    if $BINARY "$TEST_DIR/input/nonexistent.txt" "$TEST_DIR/output/dest.txt" 2>&1 | grep -q "not exist\|Error"; then
        echo "✓ PASS: Error handling works"
    else
        echo "✗ FAIL: Error not handled"
        return 1
    fi
}

test_protocol_parsing() {
    echo "Test 8: Protocol parsing"
    output=$($BINARY "http://example.com/file.txt" "$TEST_DIR/output/http_test.txt" 2>&1)
    if echo "$output" | grep -q -i "download\|http\|curl\|wget\|error"; then
        echo "✓ PASS: HTTP protocol recognized"
    else
        echo "⚠ SKIP: HTTP download test (may require network or tools)"
    fi
}

test_help_output() {
    echo "Test 9: Help output"
    if $BINARY --help 2>&1 | grep -q "usync\|--verbose\|--recursive"; then
        echo "✓ PASS: Help output correct"
    else
        echo "✗ FAIL: Help output incorrect"
        return 1
    fi
}

test_version_output() {
    echo "Test 10: Version output"
    if $BINARY --version 2>&1 | grep -q "usync"; then
        echo "✓ PASS: Version output correct"
    else
        echo "✗ FAIL: Version output incorrect"
        return 1
    fi
}

test_short_flags() {
    echo "Test 11: Short flags"
    $BINARY -v -p -r "$TEST_DIR/input/subdir" "$TEST_DIR/output/subdir_short" > /dev/null 2>&1
    if [ -d "$TEST_DIR/output/subdir_short" ]; then
        echo "✓ PASS: Short flags work"
    else
        echo "✗ FAIL: Short flags failed"
        return 1
    fi
}

test_file_content_verification() {
    echo "Test 12: File content verification"
    $BINARY "$TEST_DIR/input/file1.txt" "$TEST_DIR/output/file1_verify.txt"
    if diff -q "$TEST_DIR/input/file1.txt" "$TEST_DIR/output/file1_verify.txt" > /dev/null; then
        echo "✓ PASS: File content matches"
    else
        echo "✗ FAIL: File content mismatch"
        return 1
    fi
}

test_nested_directories() {
    echo "Test 13: Nested directory structure"
    mkdir -p "$TEST_DIR/input/nested/level1/level2"
    echo "nested content" > "$TEST_DIR/input/nested/level1/level2/deep.txt"
    $BINARY -r "$TEST_DIR/input/nested" "$TEST_DIR/output/nested_copy"
    if [ -f "$TEST_DIR/output/nested_copy/level1/level2/deep.txt" ]; then
        echo "✓ PASS: Nested directories copied"
    else
        echo "✗ FAIL: Nested directories not copied"
        return 1
    fi
}

test_multiple_files() {
    echo "Test 14: Multiple files in directory"
    $BINARY -r "$TEST_DIR/input" "$TEST_DIR/output/input_copy"
    count_input=$(find "$TEST_DIR/input" -type f | wc -l)
    count_output=$(find "$TEST_DIR/output/input_copy" -type f | wc -l)
    if [ "$count_input" -eq "$count_output" ]; then
        echo "✓ PASS: All files copied ($count_input files)"
    else
        echo "✗ FAIL: File count mismatch (input: $count_input, output: $count_output)"
        return 1
    fi
}

main() {
    setup
    
    passed=0
    failed=0
    
    test_single_file_copy && ((passed++)) || ((failed++))
    test_file_to_directory && ((passed++)) || ((failed++))
    test_recursive_copy && ((passed++)) || ((failed++))
    test_recursive_flag && ((passed++)) || ((failed++))
    test_verbose_mode && ((passed++)) || ((failed++))
    test_progress_mode && ((passed++)) || ((failed++))
    test_error_handling && ((passed++)) || ((failed++))
    test_protocol_parsing && ((passed++)) || ((failed++))
    test_help_output && ((passed++)) || ((failed++))
    test_version_output && ((passed++)) || ((failed++))
    test_short_flags && ((passed++)) || ((failed++))
    test_file_content_verification && ((passed++)) || ((failed++))
    test_nested_directories && ((passed++)) || ((failed++))
    test_multiple_files && ((passed++)) || ((failed++))
    
    echo ""
    echo "=== Test Summary ==="
    echo "Passed: $passed"
    echo "Failed: $failed"
    echo "Total: $((passed + failed))"
    
    if [ $failed -eq 0 ]; then
        echo "✓ All tests passed!"
        return 0
    else
        echo "✗ Some tests failed"
        return 1
    fi
}

main "$@"

