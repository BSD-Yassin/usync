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

mkdir -p "$BENCH_DIR"/{source,dest}
mkdir -p "$RESULTS_DIR"

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
    
    local cp_time=$(time_command "cp '$src' '$BENCH_DIR/dest/large_file_cp.bin'" "cp")
    local cp_speed=$(echo "scale=2; $size / $cp_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}cp:${NC} ${cp_time}s (${cp_speed} MB/s)"
    
    rm -rf "$BENCH_DIR/dest"/* 2>/dev/null || true
    local rsync_time=$(time_command "rsync -aq '$src' '$BENCH_DIR/dest/large_file_rsync.bin'" "rsync")
    local rsync_speed=$(echo "scale=2; $size / $rsync_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}rsync:${NC} ${rsync_time}s (${rsync_speed} MB/s)"
    
    rm -rf "$BENCH_DIR/dest"/* 2>/dev/null || true
    local usync_time=$(time_command "'$BINARY' '$src' '$BENCH_DIR/dest/large_file_usync.bin'" "usync (regular)")
    local usync_speed=$(echo "scale=2; $size / $usync_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}usync (regular):${NC} ${usync_time}s (${usync_speed} MB/s)"
    
    rm -rf "$BENCH_DIR/dest"/* 2>/dev/null || true
    local usync_ram_time=$(time_command "'$BINARY' --ram '$src' '$BENCH_DIR/dest/large_file_usync_ram.bin'" "usync (RAM)")
    local usync_ram_speed=$(echo "scale=2; $size / $usync_ram_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}usync (RAM):${NC} ${usync_ram_time}s (${usync_ram_speed} MB/s)"
    
    echo ""
    echo "Large File Copy ($size_human):" >> "$RESULTS_FILE"
    echo "  cp: ${cp_time}s (${cp_speed} MB/s)" >> "$RESULTS_FILE"
    echo "  rsync: ${rsync_time}s (${rsync_speed} MB/s)" >> "$RESULTS_FILE"
    echo "  usync (regular): ${usync_time}s (${usync_speed} MB/s)" >> "$RESULTS_FILE"
    echo "  usync (RAM): ${usync_ram_time}s (${usync_ram_speed} MB/s)" >> "$RESULTS_FILE"
    echo "" >> "$RESULTS_FILE"
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
    
    local cp_time=$(time_command "cp -r '$src' '$BENCH_DIR/dest/test_dir_cp'" "cp -r")
    local cp_speed=$(echo "scale=2; $size / $cp_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}cp -r:${NC} ${cp_time}s (${cp_speed} MB/s)"
    
    rm -rf "$BENCH_DIR/dest/test_dir"*
    local rsync_time=$(time_command "rsync -aq '$src' '$BENCH_DIR/dest/test_dir_rsync'" "rsync")
    local rsync_speed=$(echo "scale=2; $size / $rsync_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}rsync:${NC} ${rsync_time}s (${rsync_speed} MB/s)"
    
    rm -rf "$BENCH_DIR/dest/test_dir"*
    local usync_time=$(time_command "'$BINARY' -r '$src' '$BENCH_DIR/dest/test_dir_usync'" "usync -r (regular)")
    local usync_speed=$(echo "scale=2; $size / $usync_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}usync -r (regular):${NC} ${usync_time}s (${usync_speed} MB/s)"
    
    rm -rf "$BENCH_DIR/dest/test_dir"*
    local usync_ram_time=$(time_command "'$BINARY' -r --ram '$src' '$BENCH_DIR/dest/test_dir_usync_ram'" "usync -r (RAM)")
    local usync_ram_speed=$(echo "scale=2; $size / $usync_ram_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}usync -r (RAM):${NC} ${usync_ram_time}s (${usync_ram_speed} MB/s)"
    
    echo ""
    echo "Directory Copy ($size_human, ${NUM_FILES} files):" >> "$RESULTS_FILE"
    echo "  cp -r: ${cp_time}s (${cp_speed} MB/s)" >> "$RESULTS_FILE"
    echo "  rsync: ${rsync_time}s (${rsync_speed} MB/s)" >> "$RESULTS_FILE"
    echo "  usync -r (regular): ${usync_time}s (${usync_speed} MB/s)" >> "$RESULTS_FILE"
    echo "  usync -r (RAM): ${usync_ram_time}s (${usync_ram_speed} MB/s)" >> "$RESULTS_FILE"
    echo "" >> "$RESULTS_FILE"
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
    
    local cp_time=$(time_command "cp -r '$src' '$BENCH_DIR/dest/nested_dir_cp'" "cp -r")
    local cp_speed=$(echo "scale=2; $size / $cp_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}cp -r:${NC} ${cp_time}s (${cp_speed} MB/s)"
    
    rm -rf "$BENCH_DIR/dest/nested_dir"*
    local rsync_time=$(time_command "rsync -aq '$src' '$BENCH_DIR/dest/nested_dir_rsync'" "rsync")
    local rsync_speed=$(echo "scale=2; $size / $rsync_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}rsync:${NC} ${rsync_time}s (${rsync_speed} MB/s)"
    
    rm -rf "$BENCH_DIR/dest/nested_dir"*
    local usync_time=$(time_command "'$BINARY' -r '$src' '$BENCH_DIR/dest/nested_dir_usync'" "usync -r (regular)")
    local usync_speed=$(echo "scale=2; $size / $usync_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}usync -r (regular):${NC} ${usync_time}s (${usync_speed} MB/s)"
    
    rm -rf "$BENCH_DIR/dest/nested_dir"*
    local usync_ram_time=$(time_command "'$BINARY' -r --ram '$src' '$BENCH_DIR/dest/nested_dir_usync_ram'" "usync -r (RAM)")
    local usync_ram_speed=$(echo "scale=2; $size / $usync_ram_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}usync -r (RAM):${NC} ${usync_ram_time}s (${usync_ram_speed} MB/s)"
    
    echo ""
    echo "Nested Directory Copy ($size_human):" >> "$RESULTS_FILE"
    echo "  cp -r: ${cp_time}s (${cp_speed} MB/s)" >> "$RESULTS_FILE"
    echo "  rsync: ${rsync_time}s (${rsync_speed} MB/s)" >> "$RESULTS_FILE"
    echo "  usync -r (regular): ${usync_time}s (${usync_speed} MB/s)" >> "$RESULTS_FILE"
    echo "  usync -r (RAM): ${usync_ram_time}s (${usync_ram_speed} MB/s)" >> "$RESULTS_FILE"
    echo "" >> "$RESULTS_FILE"
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
    
    echo "File size: $size_human"
    echo ""
    
    local scp_time=$(time_command "scp -o ConnectTimeout=5 -o StrictHostKeyChecking=no '$src' '$REMOTE_HOST:$remote_path'" "scp" 60)
    local scp_speed=$(echo "scale=2; $size / $scp_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}scp:${NC} ${scp_time}s (${scp_speed} MB/s)"
    
    ssh -o ConnectTimeout=3 -o StrictHostKeyChecking=no "$REMOTE_HOST" "rm -f $remote_path" >/dev/null 2>&1 || true
    
    if command -v rsync &> /dev/null; then
        local rsync_time=$(time_command "rsync -aq -e 'ssh -o ConnectTimeout=5 -o StrictHostKeyChecking=no' '$src' '$REMOTE_HOST:$remote_path'" "rsync" 60)
        local rsync_speed=$(echo "scale=2; $size / $rsync_time / 1048576" | bc 2>/dev/null || echo "0")
        echo -e "${GREEN}rsync:${NC} ${rsync_time}s (${rsync_speed} MB/s)"
        ssh -o ConnectTimeout=3 -o StrictHostKeyChecking=no "$REMOTE_HOST" "rm -f $remote_path" >/dev/null 2>&1 || true
    fi
    
    local usync_time=$(time_command "'$BINARY' '$src' '$REMOTE_HOST:$remote_path'" "usync (regular)" 60)
    local usync_speed=$(echo "scale=2; $size / $usync_time / 1048576" | bc 2>/dev/null || echo "0")
    echo -e "${GREEN}usync (regular):${NC} ${usync_time}s (${usync_speed} MB/s)"
    
    ssh -o ConnectTimeout=3 -o StrictHostKeyChecking=no "$REMOTE_HOST" "rm -f $remote_path" >/dev/null 2>&1 || true
    
    echo ""
    echo "Remote File Copy ($size_human):" >> "$RESULTS_FILE"
    echo "  scp: ${scp_time}s (${scp_speed} MB/s)" >> "$RESULTS_FILE"
    if command -v rsync &> /dev/null; then
        echo "  rsync: ${rsync_time}s (${rsync_speed} MB/s)" >> "$RESULTS_FILE"
    fi
    echo "  usync (regular): ${usync_time}s (${usync_speed} MB/s)" >> "$RESULTS_FILE"
    echo "" >> "$RESULTS_FILE"
}

main() {
    if ! command -v bc &> /dev/null; then
        echo -e "${RED}Error: bc is required for calculations${NC}"
        echo "Install with: brew install bc (macOS) or apt-get install bc (Linux)"
        exit 1
    fi
    
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
    
    echo ""
    echo -e "${BLUE}=== Benchmark Complete ===${NC}"
    echo ""
    echo -e "${GREEN}Results saved to: $RESULTS_FILE${NC}"
    echo ""
    echo "Summary:"
    cat "$RESULTS_FILE"
}

main "$@"

