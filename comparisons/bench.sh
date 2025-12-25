#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BINARY="${1:-$PROJECT_ROOT/target/release/usync}"
REMOTE_HOST="${2:-}"

BENCH_DIR="$SCRIPT_DIR/bench"
RESULTS_DIR="$SCRIPT_DIR/results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_FILE="$RESULTS_DIR/results_${TIMESTAMP}.txt"
RESULTS_MD="$RESULTS_DIR/results_${TIMESTAMP}.md"

mkdir -p "$BENCH_DIR"/{source,dest}
mkdir -p "$RESULTS_DIR"

# Source checksum utilities
CHECKSUM_UTILS="$PROJECT_ROOT/tests/checksum_utils.sh"
if [ -f "$CHECKSUM_UTILS" ]; then
    source "$CHECKSUM_UTILS"
fi

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

LARGE_FILE_SIZE="100M"
MEDIUM_FILE_SIZE="10M"
NUM_FILES=100
DIR_DEPTH=3

echo -e "${BLUE}=== usync Performance Comparison ===${NC}"
echo "Binary: $BINARY"
echo "Results: $RESULTS_FILE"
echo ""

cleanup() {
    echo -e "${YELLOW}Cleaning up benchmark files...${NC}"
    rm -rf "$BENCH_DIR"
}

trap cleanup EXIT

setup_benchmark() {
    echo -e "${BLUE}Setting up benchmark environment...${NC}"
    
    echo -e "${YELLOW}Creating large test file (${LARGE_FILE_SIZE})...${NC}"
    dd if=/dev/urandom of="$BENCH_DIR/source/large_file.bin" bs=1M count=100 2>/dev/null
    
    echo -e "${YELLOW}Creating medium test file (${MEDIUM_FILE_SIZE})...${NC}"
    dd if=/dev/urandom of="$BENCH_DIR/source/medium_file.bin" bs=1M count=10 2>/dev/null
    
    echo -e "${YELLOW}Creating test directory with ${NUM_FILES} files...${NC}"
    mkdir -p "$BENCH_DIR/source/test_dir"
    for i in $(seq 1 $NUM_FILES); do
        dd if=/dev/urandom of="$BENCH_DIR/source/test_dir/file_$i.bin" bs=1M count=1 2>/dev/null
    done
    
    echo -e "${YELLOW}Creating nested directory structure (depth ${DIR_DEPTH})...${NC}"
    mkdir -p "$BENCH_DIR/source/nested_dir"
    for depth in $(seq 1 $DIR_DEPTH); do
        for i in $(seq 1 10); do
            mkdir -p "$BENCH_DIR/source/nested_dir/level${depth}/dir$i"
            dd if=/dev/urandom of="$BENCH_DIR/source/nested_dir/level${depth}/dir$i/file.bin" bs=1M count=1 2>/dev/null
        done
    done
    
    echo -e "${GREEN}✓ Benchmark environment ready${NC}"
    echo ""
}

get_system_info() {
    local os=$(uname -s)
    local arch=$(uname -m)
    local kernel=$(uname -r)
    local cpu_count=$(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo "unknown")
    local cpu_model=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || grep -m1 "model name" /proc/cpuinfo 2>/dev/null | cut -d: -f2 | sed 's/^[ \t]*//' || echo "unknown")
    local mem_total
    if [ "$os" = "Darwin" ]; then
        mem_total=$(sysctl -n hw.memsize 2>/dev/null | awk '{printf "%.2f", $1/1024/1024/1024}' || echo "unknown")
    else
        mem_total=$(grep MemTotal /proc/meminfo 2>/dev/null | awk '{printf "%.2f", $2/1024/1024}' || echo "unknown")
    fi
    
    echo "System Information:" >> "$RESULTS_FILE"
    echo "  OS: $os" >> "$RESULTS_FILE"
    echo "  Architecture: $arch" >> "$RESULTS_FILE"
    echo "  Kernel: $kernel" >> "$RESULTS_FILE"
    echo "  CPU Cores: $cpu_count" >> "$RESULTS_FILE"
    echo "  CPU Model: $cpu_model" >> "$RESULTS_FILE"
    if [ "$mem_total" != "unknown" ]; then
        echo "  Memory: ${mem_total} GB" >> "$RESULTS_FILE"
    else
        echo "  Memory: unknown" >> "$RESULTS_FILE"
    fi
    echo "  Binary: $BINARY" >> "$RESULTS_FILE"
    echo "  Date: $(date)" >> "$RESULTS_FILE"
    echo "" >> "$RESULTS_FILE"
}

time_command() {
    local cmd="$1"
    local label="$2"
    local timeout_seconds="${3:-300}"
    
    echo -e "${YELLOW}Running: $label${NC}" >&2
    
    local start=$(date +%s.%N)
    if command -v gtimeout &> /dev/null; then
        eval "gtimeout $timeout_seconds $cmd" >/dev/null 2>&1
    elif command -v timeout &> /dev/null; then
        eval "timeout $timeout_seconds $cmd" >/dev/null 2>&1
    else
        eval "$cmd" >/dev/null 2>&1
    fi
    local end=$(date +%s.%N)
    
    local duration=$(echo "$end - $start" | bc)
    echo "$duration"
}

get_size() {
    if [ -f "$1" ]; then
        stat -f%z "$1" 2>/dev/null || stat -c%s "$1" 2>/dev/null || echo "0"
    else
        du -sb "$1" 2>/dev/null | cut -f1 || echo "0"
    fi
}

verify_integrity() {
    local src="$1"
    local dst="$2"
    
    if [ ! -f "$src" ] || [ ! -f "$dst" ]; then
        return 1
    fi
    
    if [ -f "$CHECKSUM_UTILS" ]; then
        compare_files "$src" "$dst" >/dev/null 2>&1
        return $?
    else
        # Fallback: compare file sizes
        local src_size=$(get_size "$src")
        local dst_size=$(get_size "$dst")
        [ "$src_size" = "$dst_size" ]
        return $?
    fi
}

time_with_integrity() {
    local cmd="$1"
    local label="$2"
    local src="$3"
    local dst="$4"
    local timeout_seconds="${5:-300}"
    
    local copy_time=$(time_command "$cmd" "$label" "$timeout_seconds")
    local verify_start=$(date +%s.%N)
    verify_integrity "$src" "$dst" >/dev/null 2>&1
    local verify_end=$(date +%s.%N)
    local verify_time=$(echo "$verify_end - $verify_start" | bc)
    local total_time=$(echo "$copy_time + $verify_time" | bc)
    
    echo "$copy_time|$verify_time|$total_time"
}

format_size() {
    local bytes=$1
    if [ "$bytes" -gt 1073741824 ]; then
        echo "$(echo "scale=2; $bytes / 1073741824" | bc) GB"
    elif [ "$bytes" -gt 1048576 ]; then
        echo "$(echo "scale=2; $bytes / 1048576" | bc) MB"
    elif [ "$bytes" -gt 1024 ]; then
        echo "$(echo "scale=2; $bytes / 1024" | bc) KB"
    else
        echo "${bytes} B"
    fi
}

benchmark_large_file() {
    echo -e "${BLUE}=== Benchmark 1: Single Large File Copy ===${NC}"
    echo ""
    
    local src="$BENCH_DIR/source/large_file.bin"
    local size=$(get_size "$src")
    local size_human=$(format_size "$size")
    
    echo "File size: $size_human"
    echo ""
    
    rm -rf "$BENCH_DIR/dest"/* 2>/dev/null || true
    
    # Without integrity check
    local cp_time=$(time_command "cp '$src' '$BENCH_DIR/dest/large_file_cp.bin'" "cp")
    local cp_speed=$(echo "scale=2; $size / $cp_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}cp (no check):${NC} ${cp_time}s (${cp_speed} MB/s)"
    
    rm -rf "$BENCH_DIR/dest"/* 2>/dev/null || true
    local rsync_time=$(time_command "rsync -aq '$src' '$BENCH_DIR/dest/large_file_rsync.bin'" "rsync (no check)")
    local rsync_speed=$(echo "scale=2; $size / $rsync_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}rsync (no check):${NC} ${rsync_time}s (${rsync_speed} MB/s)"
    
    rm -rf "$BENCH_DIR/dest"/* 2>/dev/null || true
    local usync_time=$(time_command "'$BINARY' '$src' '$BENCH_DIR/dest/large_file_usync.bin'" "usync (no check)")
    local usync_speed=$(echo "scale=2; $size / $usync_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}usync (no check):${NC} ${usync_time}s (${usync_speed} MB/s)"
    
    # With integrity check
    rm -rf "$BENCH_DIR/dest"/* 2>/dev/null || true
    local cp_result=$(time_with_integrity "cp '$src' '$BENCH_DIR/dest/large_file_cp_check.bin'" "cp (with check)" "$src" "$BENCH_DIR/dest/large_file_cp_check.bin")
    local cp_copy_time=$(echo "$cp_result" | cut -d'|' -f1)
    local cp_verify_time=$(echo "$cp_result" | cut -d'|' -f2)
    local cp_total_time=$(echo "$cp_result" | cut -d'|' -f3)
    local cp_check_speed=$(echo "scale=2; $size / $cp_total_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}cp (with check):${NC} ${cp_total_time}s (${cp_check_speed} MB/s) [copy: ${cp_copy_time}s, verify: ${cp_verify_time}s]"
    
    rm -rf "$BENCH_DIR/dest"/* 2>/dev/null || true
    local rsync_result=$(time_with_integrity "rsync -aq '$src' '$BENCH_DIR/dest/large_file_rsync_check.bin'" "rsync (with check)" "$src" "$BENCH_DIR/dest/large_file_rsync_check.bin")
    local rsync_copy_time=$(echo "$rsync_result" | cut -d'|' -f1)
    local rsync_verify_time=$(echo "$rsync_result" | cut -d'|' -f2)
    local rsync_total_time=$(echo "$rsync_result" | cut -d'|' -f3)
    local rsync_check_speed=$(echo "scale=2; $size / $rsync_total_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}rsync (with check):${NC} ${rsync_total_time}s (${rsync_check_speed} MB/s) [copy: ${rsync_copy_time}s, verify: ${rsync_verify_time}s]"
    
    rm -rf "$BENCH_DIR/dest"/* 2>/dev/null || true
    local usync_result=$(time_with_integrity "'$BINARY' '$src' '$BENCH_DIR/dest/large_file_usync_check.bin'" "usync (with check)" "$src" "$BENCH_DIR/dest/large_file_usync_check.bin")
    local usync_copy_time=$(echo "$usync_result" | cut -d'|' -f1)
    local usync_verify_time=$(echo "$usync_result" | cut -d'|' -f2)
    local usync_total_time=$(echo "$usync_result" | cut -d'|' -f3)
    local usync_check_speed=$(echo "scale=2; $size / $usync_total_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}usync (with check):${NC} ${usync_total_time}s (${usync_check_speed} MB/s) [copy: ${usync_copy_time}s, verify: ${usync_verify_time}s]"
    
    # Store results
    {
        echo "Large File Copy ($size_human):"
        echo "  cp (no check): ${cp_time}s (${cp_speed} MB/s)"
        echo "  cp (with check): ${cp_total_time}s (${cp_check_speed} MB/s)"
        echo "  rsync (no check): ${rsync_time}s (${rsync_speed} MB/s)"
        echo "  rsync (with check): ${rsync_total_time}s (${rsync_check_speed} MB/s)"
        echo "  usync (no check): ${usync_time}s (${usync_speed} MB/s)"
        echo "  usync (with check): ${usync_total_time}s (${usync_check_speed} MB/s)"
        echo ""
    } >> "$RESULTS_FILE"
    
    # Store for markdown table
    BENCH_LARGE_FILE=(
        "cp|no check|${cp_time}|${cp_speed}"
        "cp|with check|${cp_total_time}|${cp_check_speed}"
        "rsync|no check|${rsync_time}|${rsync_speed}"
        "rsync|with check|${rsync_total_time}|${rsync_check_speed}"
        "usync|no check|${usync_time}|${usync_speed}"
        "usync|with check|${usync_total_time}|${usync_check_speed}"
    )
}

benchmark_directory() {
    echo -e "${BLUE}=== Benchmark 2: Directory Copy (${NUM_FILES} files) ===${NC}"
    echo ""
    
    local src="$BENCH_DIR/source/test_dir"
    local size=$(get_size "$src")
    local size_human=$(format_size "$size")
    
    echo "Directory size: $size_human"
    echo ""
    
    rm -rf "$BENCH_DIR/dest/test_dir"*
    
    # Without integrity check
    local cp_time=$(time_command "cp -r '$src' '$BENCH_DIR/dest/test_dir_cp'" "cp -r (no check)")
    local cp_speed=$(echo "scale=2; $size / $cp_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}cp -r (no check):${NC} ${cp_time}s (${cp_speed} MB/s)"
    
    rm -rf "$BENCH_DIR/dest/test_dir"*
    local rsync_time=$(time_command "rsync -aq '$src' '$BENCH_DIR/dest/test_dir_rsync'" "rsync (no check)")
    local rsync_speed=$(echo "scale=2; $size / $rsync_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}rsync (no check):${NC} ${rsync_time}s (${rsync_speed} MB/s)"
    
    rm -rf "$BENCH_DIR/dest/test_dir"*
    local usync_time=$(time_command "'$BINARY' -r '$src' '$BENCH_DIR/dest/test_dir_usync'" "usync -r (no check)")
    local usync_speed=$(echo "scale=2; $size / $usync_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}usync -r (no check):${NC} ${usync_time}s (${usync_speed} MB/s)"
    
    # With integrity check (sample first file)
    rm -rf "$BENCH_DIR/dest/test_dir"*
    local cp_result=$(time_with_integrity "cp -r '$src' '$BENCH_DIR/dest/test_dir_cp_check'" "cp -r (with check)" "$src/file_1.bin" "$BENCH_DIR/dest/test_dir_cp_check/file_1.bin")
    local cp_copy_time=$(echo "$cp_result" | cut -d'|' -f1)
    local cp_verify_time=$(echo "$cp_result" | cut -d'|' -f2)
    local cp_total_time=$(echo "$cp_result" | cut -d'|' -f3)
    local cp_check_speed=$(echo "scale=2; $size / $cp_total_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}cp -r (with check):${NC} ${cp_total_time}s (${cp_check_speed} MB/s) [copy: ${cp_copy_time}s, verify: ${cp_verify_time}s]"
    
    rm -rf "$BENCH_DIR/dest/test_dir"*
    local rsync_result=$(time_with_integrity "rsync -aq '$src' '$BENCH_DIR/dest/test_dir_rsync_check'" "rsync (with check)" "$src/file_1.bin" "$BENCH_DIR/dest/test_dir_rsync_check/file_1.bin")
    local rsync_copy_time=$(echo "$rsync_result" | cut -d'|' -f1)
    local rsync_verify_time=$(echo "$rsync_result" | cut -d'|' -f2)
    local rsync_total_time=$(echo "$rsync_result" | cut -d'|' -f3)
    local rsync_check_speed=$(echo "scale=2; $size / $rsync_total_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}rsync (with check):${NC} ${rsync_total_time}s (${rsync_check_speed} MB/s) [copy: ${rsync_copy_time}s, verify: ${rsync_verify_time}s]"
    
    rm -rf "$BENCH_DIR/dest/test_dir"*
    local usync_result=$(time_with_integrity "'$BINARY' -r '$src' '$BENCH_DIR/dest/test_dir_usync_check'" "usync -r (with check)" "$src/file_1.bin" "$BENCH_DIR/dest/test_dir_usync_check/file_1.bin")
    local usync_copy_time=$(echo "$usync_result" | cut -d'|' -f1)
    local usync_verify_time=$(echo "$usync_result" | cut -d'|' -f2)
    local usync_total_time=$(echo "$usync_result" | cut -d'|' -f3)
    local usync_check_speed=$(echo "scale=2; $size / $usync_total_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}usync -r (with check):${NC} ${usync_total_time}s (${usync_check_speed} MB/s) [copy: ${usync_copy_time}s, verify: ${usync_verify_time}s]"
    
    # Store results
    {
        echo "Directory Copy ($size_human, ${NUM_FILES} files):"
        echo "  cp -r (no check): ${cp_time}s (${cp_speed} MB/s)"
        echo "  cp -r (with check): ${cp_total_time}s (${cp_check_speed} MB/s)"
        echo "  rsync (no check): ${rsync_time}s (${rsync_speed} MB/s)"
        echo "  rsync (with check): ${rsync_total_time}s (${rsync_check_speed} MB/s)"
        echo "  usync -r (no check): ${usync_time}s (${usync_speed} MB/s)"
        echo "  usync -r (with check): ${usync_total_time}s (${usync_check_speed} MB/s)"
        echo ""
    } >> "$RESULTS_FILE"
    
    # Store for markdown table
    BENCH_DIRECTORY=(
        "cp -r|no check|${cp_time}|${cp_speed}"
        "cp -r|with check|${cp_total_time}|${cp_check_speed}"
        "rsync|no check|${rsync_time}|${rsync_speed}"
        "rsync|with check|${rsync_total_time}|${rsync_check_speed}"
        "usync -r|no check|${usync_time}|${usync_speed}"
        "usync -r|with check|${usync_total_time}|${usync_check_speed}"
    )
}

benchmark_nested() {
    echo -e "${BLUE}=== Benchmark 3: Nested Directory Structure ===${NC}"
    echo ""
    
    local src="$BENCH_DIR/source/nested_dir"
    local size=$(get_size "$src")
    local size_human=$(format_size "$size")
    
    echo "Directory size: $size_human"
    echo ""
    
    rm -rf "$BENCH_DIR/dest/nested_dir"*
    
    # Without integrity check
    local cp_time=$(time_command "cp -r '$src' '$BENCH_DIR/dest/nested_dir_cp'" "cp -r (no check)")
    local cp_speed=$(echo "scale=2; $size / $cp_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}cp -r (no check):${NC} ${cp_time}s (${cp_speed} MB/s)"
    
    rm -rf "$BENCH_DIR/dest/nested_dir"*
    local rsync_time=$(time_command "rsync -aq '$src' '$BENCH_DIR/dest/nested_dir_rsync'" "rsync (no check)")
    local rsync_speed=$(echo "scale=2; $size / $rsync_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}rsync (no check):${NC} ${rsync_time}s (${rsync_speed} MB/s)"
    
    rm -rf "$BENCH_DIR/dest/nested_dir"*
    local usync_time=$(time_command "'$BINARY' -r '$src' '$BENCH_DIR/dest/nested_dir_usync'" "usync -r (no check)")
    local usync_speed=$(echo "scale=2; $size / $usync_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}usync -r (no check):${NC} ${usync_time}s (${usync_speed} MB/s)"
    
    # With integrity check (sample one file)
    local sample_file=$(find "$src" -type f | head -n1)
    local sample_rel="${sample_file#$src/}"
    
    rm -rf "$BENCH_DIR/dest/nested_dir"*
    local cp_result=$(time_with_integrity "cp -r '$src' '$BENCH_DIR/dest/nested_dir_cp_check'" "cp -r (with check)" "$sample_file" "$BENCH_DIR/dest/nested_dir_cp_check/$sample_rel")
    local cp_copy_time=$(echo "$cp_result" | cut -d'|' -f1)
    local cp_verify_time=$(echo "$cp_result" | cut -d'|' -f2)
    local cp_total_time=$(echo "$cp_result" | cut -d'|' -f3)
    local cp_check_speed=$(echo "scale=2; $size / $cp_total_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}cp -r (with check):${NC} ${cp_total_time}s (${cp_check_speed} MB/s) [copy: ${cp_copy_time}s, verify: ${cp_verify_time}s]"
    
    rm -rf "$BENCH_DIR/dest/nested_dir"*
    local rsync_result=$(time_with_integrity "rsync -aq '$src' '$BENCH_DIR/dest/nested_dir_rsync_check'" "rsync (with check)" "$sample_file" "$BENCH_DIR/dest/nested_dir_rsync_check/$sample_rel")
    local rsync_copy_time=$(echo "$rsync_result" | cut -d'|' -f1)
    local rsync_verify_time=$(echo "$rsync_result" | cut -d'|' -f2)
    local rsync_total_time=$(echo "$rsync_result" | cut -d'|' -f3)
    local rsync_check_speed=$(echo "scale=2; $size / $rsync_total_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}rsync (with check):${NC} ${rsync_total_time}s (${rsync_check_speed} MB/s) [copy: ${rsync_copy_time}s, verify: ${rsync_verify_time}s]"
    
    rm -rf "$BENCH_DIR/dest/nested_dir"*
    local usync_result=$(time_with_integrity "'$BINARY' -r '$src' '$BENCH_DIR/dest/nested_dir_usync_check'" "usync -r (with check)" "$sample_file" "$BENCH_DIR/dest/nested_dir_usync_check/$sample_rel")
    local usync_copy_time=$(echo "$usync_result" | cut -d'|' -f1)
    local usync_verify_time=$(echo "$usync_result" | cut -d'|' -f2)
    local usync_total_time=$(echo "$usync_result" | cut -d'|' -f3)
    local usync_check_speed=$(echo "scale=2; $size / $usync_total_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}usync -r (with check):${NC} ${usync_total_time}s (${usync_check_speed} MB/s) [copy: ${usync_copy_time}s, verify: ${usync_verify_time}s]"
    
    # Store results
    {
        echo "Nested Directory Copy ($size_human):"
        echo "  cp -r (no check): ${cp_time}s (${cp_speed} MB/s)"
        echo "  cp -r (with check): ${cp_total_time}s (${cp_check_speed} MB/s)"
        echo "  rsync (no check): ${rsync_time}s (${rsync_speed} MB/s)"
        echo "  rsync (with check): ${rsync_total_time}s (${rsync_check_speed} MB/s)"
        echo "  usync -r (no check): ${usync_time}s (${usync_speed} MB/s)"
        echo "  usync -r (with check): ${usync_total_time}s (${usync_check_speed} MB/s)"
        echo ""
    } >> "$RESULTS_FILE"
    
    # Store for markdown table
    BENCH_NESTED=(
        "cp -r|no check|${cp_time}|${cp_speed}"
        "cp -r|with check|${cp_total_time}|${cp_check_speed}"
        "rsync|no check|${rsync_time}|${rsync_speed}"
        "rsync|with check|${rsync_total_time}|${rsync_check_speed}"
        "usync -r|no check|${usync_time}|${usync_speed}"
        "usync -r|with check|${usync_total_time}|${usync_check_speed}"
    )
}

benchmark_remote() {
    if ! command -v ssh &> /dev/null || ! command -v scp &> /dev/null; then
        echo -e "${YELLOW}SSH/SCP not available, skipping remote benchmarks${NC}"
        return 1
    fi
    
    if ! ssh -o ConnectTimeout=5 -o BatchMode=yes -o StrictHostKeyChecking=no "$REMOTE_HOST" "echo 'OK'" >/dev/null 2>&1; then
        echo -e "${YELLOW}Cannot connect to remote host, skipping remote benchmarks${NC}"
        return 1
    fi
    
    echo -e "${BLUE}=== Benchmark 4: Remote File Copy (SSH) ===${NC}"
    echo ""
    
    local src="$BENCH_DIR/source/medium_file.bin"
    local size=$(get_size "$src")
    local size_human=$(format_size "$size")
    local remote_path="/tmp/benchmark_medium_file.bin"
    local local_dest="$BENCH_DIR/dest/medium_file_remote.bin"
    
    echo "File size: $size_human"
    echo ""
    
    # Without integrity check
    local scp_time=$(time_command "scp -o ConnectTimeout=5 -o StrictHostKeyChecking=no '$src' '$REMOTE_HOST:$remote_path'" "scp (no check)" 60)
    local scp_speed=$(echo "scale=2; $size / $scp_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}scp (no check):${NC} ${scp_time}s (${scp_speed} MB/s)"
    
    ssh -o ConnectTimeout=3 -o StrictHostKeyChecking=no "$REMOTE_HOST" "rm -f $remote_path" >/dev/null 2>&1 || true
    
    local rsync_time=""
    local rsync_speed=""
    if command -v rsync &> /dev/null; then
        rsync_time=$(time_command "rsync -aq -e 'ssh -o ConnectTimeout=5 -o StrictHostKeyChecking=no' '$src' '$REMOTE_HOST:$remote_path'" "rsync (no check)" 60)
        rsync_speed=$(echo "scale=2; $size / $rsync_time / 1048576" | bc 2>/dev/null || echo "0")
        echo -e "${GREEN}rsync (no check):${NC} ${rsync_time}s (${rsync_speed} MB/s)"
        ssh -o ConnectTimeout=3 -o StrictHostKeyChecking=no "$REMOTE_HOST" "rm -f $remote_path" >/dev/null 2>&1 || true
    fi
    
    local usync_time=$(time_command "'$BINARY' '$src' '$REMOTE_HOST:$remote_path'" "usync (no check)" 60)
    local usync_speed=$(echo "scale=2; $size / $usync_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}usync (no check):${NC} ${usync_time}s (${usync_speed} MB/s)"
    
    ssh -o ConnectTimeout=3 -o StrictHostKeyChecking=no "$REMOTE_HOST" "rm -f $remote_path" >/dev/null 2>&1 || true
    
    # With integrity check (upload, then download back and verify)
    local scp_upload_time=$(time_command "scp -o ConnectTimeout=5 -o StrictHostKeyChecking=no '$src' '$REMOTE_HOST:$remote_path'" "scp upload" 60)
    local scp_download_time=$(time_command "scp -o ConnectTimeout=5 -o StrictHostKeyChecking=no '$REMOTE_HOST:$remote_path' '$local_dest'" "scp download" 60)
    local verify_start=$(date +%s.%N)
    verify_integrity "$src" "$local_dest" >/dev/null 2>&1
    local verify_end=$(date +%s.%N)
    local scp_verify_time=$(echo "$verify_end - $verify_start" | bc)
    local scp_total_time=$(echo "$scp_upload_time + $scp_download_time + $scp_verify_time" | bc)
    local scp_check_speed=$(echo "scale=2; $size / $scp_total_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}scp (with check):${NC} ${scp_total_time}s (${scp_check_speed} MB/s) [upload: ${scp_upload_time}s, download: ${scp_download_time}s, verify: ${scp_verify_time}s]"
    
    ssh -o ConnectTimeout=3 -o StrictHostKeyChecking=no "$REMOTE_HOST" "rm -f $remote_path" >/dev/null 2>&1 || true
    rm -f "$local_dest"
    
    local rsync_upload_time=""
    local rsync_download_time=""
    local rsync_verify_time=""
    local rsync_total_time=""
    local rsync_check_speed=""
    if command -v rsync &> /dev/null; then
        rsync_upload_time=$(time_command "rsync -aq -e 'ssh -o ConnectTimeout=5 -o StrictHostKeyChecking=no' '$src' '$REMOTE_HOST:$remote_path'" "rsync upload" 60)
        rsync_download_time=$(time_command "rsync -aq -e 'ssh -o ConnectTimeout=5 -o StrictHostKeyChecking=no' '$REMOTE_HOST:$remote_path' '$local_dest'" "rsync download" 60)
        verify_start=$(date +%s.%N)
        verify_integrity "$src" "$local_dest" >/dev/null 2>&1
        verify_end=$(date +%s.%N)
        rsync_verify_time=$(echo "$verify_end - $verify_start" | bc)
        rsync_total_time=$(echo "$rsync_upload_time + $rsync_download_time + $rsync_verify_time" | bc)
        rsync_check_speed=$(echo "scale=2; $size / $rsync_total_time / 1048576" | bc 2>/dev/null || echo "0")
        echo -e "${GREEN}rsync (with check):${NC} ${rsync_total_time}s (${rsync_check_speed} MB/s) [upload: ${rsync_upload_time}s, download: ${rsync_download_time}s, verify: ${rsync_verify_time}s]"
        ssh -o ConnectTimeout=3 -o StrictHostKeyChecking=no "$REMOTE_HOST" "rm -f $remote_path" >/dev/null 2>&1 || true
        rm -f "$local_dest"
    fi
    
    local usync_upload_time=$(time_command "'$BINARY' '$src' '$REMOTE_HOST:$remote_path'" "usync upload" 60)
    local usync_download_time=$(time_command "'$BINARY' '$REMOTE_HOST:$remote_path' '$local_dest'" "usync download" 60)
    verify_start=$(date +%s.%N)
    verify_integrity "$src" "$local_dest" >/dev/null 2>&1
    verify_end=$(date +%s.%N)
    local usync_verify_time=$(echo "$verify_end - $verify_start" | bc)
    local usync_total_time=$(echo "$usync_upload_time + $usync_download_time + $usync_verify_time" | bc)
    local usync_check_speed=$(echo "scale=2; $size / $usync_total_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}usync (with check):${NC} ${usync_total_time}s (${usync_check_speed} MB/s) [upload: ${usync_upload_time}s, download: ${usync_download_time}s, verify: ${usync_verify_time}s]"
    
    ssh -o ConnectTimeout=3 -o StrictHostKeyChecking=no "$REMOTE_HOST" "rm -f $remote_path" >/dev/null 2>&1 || true
    rm -f "$local_dest"
    
    # Store results
    {
        echo "Remote File Copy ($size_human):"
        echo "  scp (no check): ${scp_time}s (${scp_speed} MB/s)"
        echo "  scp (with check): ${scp_total_time}s (${scp_check_speed} MB/s)"
        if [ -n "$rsync_time" ]; then
            echo "  rsync (no check): ${rsync_time}s (${rsync_speed} MB/s)"
            echo "  rsync (with check): ${rsync_total_time}s (${rsync_check_speed} MB/s)"
        fi
        echo "  usync (no check): ${usync_time}s (${usync_speed} MB/s)"
        echo "  usync (with check): ${usync_total_time}s (${usync_check_speed} MB/s)"
        echo ""
    } >> "$RESULTS_FILE"
    
    # Store for markdown table
    BENCH_REMOTE=(
        "scp|no check|${scp_time}|${scp_speed}"
        "scp|with check|${scp_total_time}|${scp_check_speed}"
    )
    if [ -n "$rsync_time" ]; then
        BENCH_REMOTE+=(
            "rsync|no check|${rsync_time}|${rsync_speed}"
            "rsync|with check|${rsync_total_time}|${rsync_check_speed}"
        )
    fi
    BENCH_REMOTE+=(
        "usync|no check|${usync_time}|${usync_speed}"
        "usync|with check|${usync_total_time}|${usync_check_speed}"
    )
}

generate_markdown_table() {
    {
        echo "# usync Performance Comparison Results"
        echo ""
        echo "## System Information"
        echo ""
        cat "$RESULTS_FILE" | grep -A 10 "System Information:" | sed 's/^/  /'
        echo ""
        echo "## Benchmark Results"
        echo ""
        echo "### Large File Copy"
        echo ""
        echo "| Tool | Integrity Check | Time (s) | Speed (MB/s) |"
        echo "|------|----------------|----------|--------------|"
        if [ ${#BENCH_LARGE_FILE[@]} -gt 0 ]; then
            for entry in "${BENCH_LARGE_FILE[@]}"; do
                IFS='|' read -r tool check time speed <<< "$entry"
                echo "| $tool | $check | $time | $speed |"
            done
        fi
        echo ""
        echo "### Directory Copy"
        echo ""
        echo "| Tool | Integrity Check | Time (s) | Speed (MB/s) |"
        echo "|------|----------------|----------|--------------|"
        if [ ${#BENCH_DIRECTORY[@]} -gt 0 ]; then
            for entry in "${BENCH_DIRECTORY[@]}"; do
                IFS='|' read -r tool check time speed <<< "$entry"
                echo "| $tool | $check | $time | $speed |"
            done
        fi
        echo ""
        echo "### Nested Directory Copy"
        echo ""
        echo "| Tool | Integrity Check | Time (s) | Speed (MB/s) |"
        echo "|------|----------------|----------|--------------|"
        if [ ${#BENCH_NESTED[@]} -gt 0 ]; then
            for entry in "${BENCH_NESTED[@]}"; do
                IFS='|' read -r tool check time speed <<< "$entry"
                echo "| $tool | $check | $time | $speed |"
            done
        fi
        if [ ${#BENCH_REMOTE[@]} -gt 0 ] 2>/dev/null; then
            echo ""
            echo "### Remote File Copy (SSH)"
            echo ""
            echo "| Tool | Integrity Check | Time (s) | Speed (MB/s) |"
            echo "|------|----------------|----------|--------------|"
            for entry in "${BENCH_REMOTE[@]}"; do
                IFS='|' read -r tool check time speed <<< "$entry"
                echo "| $tool | $check | $time | $speed |"
            done
        fi
        echo ""
        echo "---"
        echo ""
        echo "**Note:** usync does not automatically verify file integrity after copy operations (unlike rsync), which explains the speed advantage. For critical transfers, users should manually verify checksums when needed."
    } > "$RESULTS_MD"
}

main() {
    if ! command -v bc &> /dev/null; then
        echo -e "${RED}Error: bc is required for calculations${NC}"
        echo "Install with: brew install bc (macOS) or apt-get install bc (Linux)"
        exit 1
    fi
    
    # Initialize arrays
    BENCH_LARGE_FILE=()
    BENCH_DIRECTORY=()
    BENCH_NESTED=()
    BENCH_REMOTE=()
    
    echo "usync Performance Comparison Results" > "$RESULTS_FILE"
    echo "========================================" >> "$RESULTS_FILE"
    echo "" >> "$RESULTS_FILE"
    get_system_info
    
    setup_benchmark
    
    benchmark_large_file
    benchmark_directory
    benchmark_nested
    
    if [ -n "$REMOTE_HOST" ]; then
        echo ""
        echo -e "${BLUE}=== Remote Benchmark (SSH) ===${NC}"
        echo ""
        benchmark_remote || echo -e "${RED}Remote benchmark skipped (SSH not available)${NC}"
    fi
    
    generate_markdown_table
    
    echo ""
    echo -e "${BLUE}=== Benchmark Complete ===${NC}"
    echo ""
    echo -e "${GREEN}Results saved to:${NC}"
    echo "  - Text: $RESULTS_FILE"
    echo "  - Markdown: $RESULTS_MD"
    echo ""
    echo -e "${BLUE}=== Markdown Table ===${NC}"
    echo ""
    cat "$RESULTS_MD"
}

main "$@"

