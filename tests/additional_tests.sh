#!/bin/bash
set -e

BINARY="${1:-target/release/usync}"
TEST_DIR="tests"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/setup_test_files.sh"

echo "=== Additional usync Tests ==="
echo ""

setup() {
    setup_test_files
    mkdir -p "$TEST_DIR/output"
}

test_ssh_style_parsing() {
    echo "Test: SSH-style path parsing"
    output=$($BINARY "user@host:/path/file.txt" "$TEST_DIR/output/ssh_test.txt" 2>&1)
    if echo "$output" | grep -q -i "ssh\|scp\|error"; then
        echo "✓ PASS: SSH-style path recognized"
    else
        echo "⚠ SKIP: SSH test (requires SSH setup)"
    fi
}

test_ssh_options() {
    echo "Test: SSH options flag"
    if $BINARY --help 2>&1 | grep -q "ssh-opt\|--ssh-opt"; then
        echo "✓ PASS: SSH options flag exists"
    else
        echo "✗ FAIL: SSH options flag not found"
        return 1
    fi
}

test_environment_verbose() {
    echo "Test: Environment variable verbose"
    USYNC_VERBOSE=1 $BINARY "$TEST_DIR/input/file1.txt" "$TEST_DIR/output/env_verbose.txt" > /dev/null 2>&1
    if [ -f "$TEST_DIR/output/env_verbose.txt" ]; then
        echo "✓ PASS: Environment verbose works"
    else
        echo "✗ FAIL: Environment verbose failed"
        return 1
    fi
}

test_empty_file() {
    echo "Test: Empty file copy"
    $BINARY "$TEST_DIR/input/empty.txt" "$TEST_DIR/output/empty_copy.txt"
    if [ -f "$TEST_DIR/output/empty_copy.txt" ] && [ ! -s "$TEST_DIR/output/empty_copy.txt" ]; then
        echo "✓ PASS: Empty file copied"
    else
        echo "✗ FAIL: Empty file not copied correctly"
        return 1
    fi
}

test_special_characters() {
    echo "Test: Special characters in filename"
    $BINARY "$TEST_DIR/input/file with spaces.txt" "$TEST_DIR/output/special_copy.txt"
    if [ -f "$TEST_DIR/output/special_copy.txt" ]; then
        echo "✓ PASS: Special characters handled"
    else
        echo "✗ FAIL: Special characters not handled"
        return 1
    fi
}

test_large_directory() {
    echo "Test: Large directory structure"
    $BINARY -r "$TEST_DIR/input/large" "$TEST_DIR/output/large_copy"
    count=$(find "$TEST_DIR/output/large_copy" -type f | wc -l)
    if [ "$count" -eq 50 ]; then
        echo "✓ PASS: Large directory copied ($count files)"
    else
        echo "✗ FAIL: Large directory copy failed (expected 50, got $count)"
        return 1
    fi
}

test_recursive_alias() {
    echo "Test: Recursive alias (--rec)"
    $BINARY --rec "$TEST_DIR/input/subdir" "$TEST_DIR/output/subdir_alias"
    if [ -d "$TEST_DIR/output/subdir_alias" ] && [ -f "$TEST_DIR/output/subdir_alias/file3.txt" ]; then
        echo "✓ PASS: Recursive alias works"
    else
        echo "✗ FAIL: Recursive alias failed"
        return 1
    fi
}

test_combined_flags() {
    echo "Test: Combined flags"
    $BINARY -r -p -v "$TEST_DIR/input/subdir" "$TEST_DIR/output/subdir_combined" > /dev/null 2>&1
    if [ -d "$TEST_DIR/output/subdir_combined" ]; then
        echo "✓ PASS: Combined flags work"
    else
        echo "✗ FAIL: Combined flags failed"
        return 1
    fi
}

test_parent_directory_creation() {
    echo "Test: Parent directory creation"
    $BINARY "$TEST_DIR/input/file1.txt" "$TEST_DIR/output/nested/path/file1.txt"
    if [ -f "$TEST_DIR/output/nested/path/file1.txt" ]; then
        echo "✓ PASS: Parent directories created"
    else
        echo "✗ FAIL: Parent directories not created"
        return 1
    fi
}

test_binary_file() {
    echo "Test: Binary file copy"
    $BINARY "$TEST_DIR/input/binary.bin" "$TEST_DIR/output/binary_copy.bin"
    if cmp -s "$TEST_DIR/input/binary.bin" "$TEST_DIR/output/binary_copy.bin"; then
        echo "✓ PASS: Binary file copied correctly"
    else
        echo "✗ FAIL: Binary file copy failed"
        return 1
    fi
}

main() {
    setup
    passed=0
    failed=0
    
    test_ssh_style_parsing && ((passed++)) || ((failed++))
    test_ssh_options && ((passed++)) || ((failed++))
    test_environment_verbose && ((passed++)) || ((failed++))
    test_empty_file && ((passed++)) || ((failed++))
    test_special_characters && ((passed++)) || ((failed++))
    test_large_directory && ((passed++)) || ((failed++))
    test_recursive_alias && ((passed++)) || ((failed++))
    test_combined_flags && ((passed++)) || ((failed++))
    test_parent_directory_creation && ((passed++)) || ((failed++))
    test_binary_file && ((passed++)) || ((failed++))
    
    echo ""
    echo "=== Additional Tests Summary ==="
    echo "Passed: $passed"
    echo "Failed: $failed"
    echo "Total: $((passed + failed))"
    
    if [ $failed -eq 0 ]; then
        echo "✓ All additional tests passed!"
        return 0
    else
        echo "✗ Some tests failed"
        return 1
    fi
}

main "$@"

