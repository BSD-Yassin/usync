#!/bin/bash

TEST_DIR="tests"

setup_test_files() {
    echo "Setting up test files..."
    
    mkdir -p "$TEST_DIR/input"
    mkdir -p "$TEST_DIR/output"
    
    echo "Hello, World!" > "$TEST_DIR/input/file1.txt"
    echo "Test content for file2" > "$TEST_DIR/input/file2.txt"
    touch "$TEST_DIR/input/empty.txt"
    printf '\x00\x01\x02\x03\xFF\xFE\xFD' > "$TEST_DIR/input/binary.bin"
    echo "file without extension" > "$TEST_DIR/input/file"
    echo "file with spaces content" > "$TEST_DIR/input/file with spaces.txt"
    
    mkdir -p "$TEST_DIR/input/subdir"
    echo "subdir file3 content" > "$TEST_DIR/input/subdir/file3.txt"
    echo "nested file content" > "$TEST_DIR/input/subdir/nested.txt"
    
    mkdir -p "$TEST_DIR/input/nested/level1/level2"
    echo "nested content" > "$TEST_DIR/input/nested/level1/level2/deep.txt"
    
    mkdir -p "$TEST_DIR/input/large"
    for i in {1..5}; do
        mkdir -p "$TEST_DIR/input/large/$i"
        for j in {1..10}; do
            echo "content $i-$j" > "$TEST_DIR/input/large/$i/file_$j.txt"
        done
    done
    
    echo "✓ Test files created"
}

cleanup_test_files() {
    if [ -d "$TEST_DIR/input" ]; then
        rm -rf "$TEST_DIR/input"/* "$TEST_DIR/input"/.[!.]* 2>/dev/null || true
    fi
    if [ -d "$TEST_DIR/output" ]; then
        rm -rf "$TEST_DIR/output"/* "$TEST_DIR/output"/.[!.]* 2>/dev/null || true
    fi
    if [ -d "$TEST_DIR/s3_output" ]; then
        rm -rf "$TEST_DIR/s3_output"/* "$TEST_DIR/s3_output"/.[!.]* 2>/dev/null || true
    fi
    if [ -d "$TEST_DIR/fixtures" ]; then
        rm -rf "$TEST_DIR/fixtures"/* "$TEST_DIR/fixtures"/.[!.]* 2>/dev/null || true
    fi
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    case "${1:-setup}" in
        setup)
            setup_test_files
            ;;
        cleanup)
            cleanup_test_files
            ;;
        *)
            echo "Usage: $0 [setup|cleanup]"
            exit 1
            ;;
    esac
fi

