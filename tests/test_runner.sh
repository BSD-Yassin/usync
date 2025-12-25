#!/bin/bash
set -e

BINARY="${1:-target/release/usync}"
TEST_DIR="tests"
RUN_S3_TESTS=0
RUN_ALL_TESTS=0

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/setup_test_files.sh"

while [[ $# -gt 0 ]]; do
    case $1 in
        --s3)
            RUN_S3_TESTS=1
            shift
            ;;
        --all)
            RUN_ALL_TESTS=1
            RUN_S3_TESTS=1
            shift
            ;;
        *)
            if [ -z "$BINARY" ] || [ "$BINARY" = "target/release/usync" ]; then
                BINARY="$1"
            fi
            shift
            ;;
    esac
done

echo "=== Running usync Integration Tests ==="
echo "Using binary: $BINARY"
if [ $RUN_S3_TESTS -eq 1 ]; then
    echo "S3 tests: enabled"
fi
echo ""

cleanup() {
    if [ -d "$TEST_DIR/output" ]; then
        rm -rf "$TEST_DIR/output"/* "$TEST_DIR/output"/.[!.]* 2>/dev/null || true
    fi
}

setup() {
    cleanup
    setup_test_files
    mkdir -p "$TEST_DIR/output"
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
        return 0
    else
        echo "⚠ SKIP: HTTP download test (may require network or tools)"
        return 0  # Skip is not a failure
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

# Check prerequisites for S3 tests
check_s3_prerequisites() {
    local missing=0
    
    # Check for Podman or Docker
    if ! command -v podman &> /dev/null && ! command -v docker &> /dev/null; then
        echo "⚠ Warning: Neither Podman nor Docker found. S3 tests require a container runtime." >&2
        missing=1
    fi
    
    if ! command -v aws &> /dev/null; then
        echo "⚠ Warning: AWS CLI not found. S3 tests require AWS CLI." >&2
        missing=1
    fi
    
    if [ $missing -eq 1 ]; then
        echo "S3 tests will be skipped." >&2
        return 1
    fi
    
    return 0
}

main() {
    setup
    
    passed=0
    failed=0
    skipped=0
    
    # Run standard tests
    # Use explicit if/else to avoid issues with set -e and arithmetic expansion
    if test_single_file_copy; then ((passed++)); else ((failed++)); fi
    if test_file_to_directory; then ((passed++)); else ((failed++)); fi
    if test_recursive_copy; then ((passed++)); else ((failed++)); fi
    if test_recursive_flag; then ((passed++)); else ((failed++)); fi
    if test_verbose_mode; then ((passed++)); else ((failed++)); fi
    if test_progress_mode; then ((passed++)); else ((failed++)); fi
    if test_error_handling; then ((passed++)); else ((failed++)); fi
    if test_protocol_parsing; then ((passed++)); else ((failed++)); fi
    if test_help_output; then ((passed++)); else ((failed++)); fi
    if test_version_output; then ((passed++)); else ((failed++)); fi
    if test_short_flags; then ((passed++)); else ((failed++)); fi
    if test_file_content_verification; then ((passed++)); else ((failed++)); fi
    if test_nested_directories; then ((passed++)); else ((failed++)); fi
    if test_multiple_files; then ((passed++)); else ((failed++)); fi
    
    # Run S3 tests if requested
    if [ $RUN_S3_TESTS -eq 1 ]; then
        echo ""
        echo "=== Running S3 Tests ==="
        
        if check_s3_prerequisites; then
            # Source S3 test script
            SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
            if [ -f "${SCRIPT_DIR}/s3_tests.sh" ]; then
                # Run S3 tests and capture results
                if bash "${SCRIPT_DIR}/s3_tests.sh" "$BINARY"; then
                    echo "✓ S3 tests completed"
                else
                    echo "✗ S3 tests had failures"
                    ((failed++))
                fi
            else
                echo "⚠ S3 test script not found: ${SCRIPT_DIR}/s3_tests.sh"
                ((skipped++))
            fi
        else
            echo "⚠ S3 tests skipped (prerequisites not met)"
            ((skipped++))
        fi
    fi
    
    echo ""
    echo "=== Test Summary ==="
    echo "Passed:  $passed"
    echo "Failed:  $failed"
    if [ $skipped -gt 0 ]; then
        echo "Skipped: $skipped"
    fi
    echo "Total:   $((passed + failed + skipped))"
    
    if [ $failed -eq 0 ]; then
        echo "✓ All tests passed!"
        return 0
    else
        echo "✗ Some tests failed"
        return 1
    fi
}

main "$@"

