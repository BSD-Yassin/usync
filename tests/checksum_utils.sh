#!/bin/bash
# Checksum verification utilities for testing
# Supports SHA256, SHA1, and MD5 on macOS and Linux

# Detect OS and use appropriate tools
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    SHA256_CMD="shasum -a 256"
    SHA1_CMD="shasum -a 1"
    MD5_CMD="md5"
    MD5_PARSE="awk '{print \$NF}'"
else
    # Linux
    SHA256_CMD="sha256sum"
    SHA1_CMD="sha1sum"
    MD5_CMD="md5sum"
    MD5_PARSE="awk '{print \$1}'"
fi

# Compute SHA256 checksum of a file
compute_sha256() {
    local file="$1"
    if [ ! -f "$file" ]; then
        echo "Error: File not found: $file" >&2
        return 1
    fi
    
    if [[ "$OSTYPE" == "darwin"* ]]; then
        shasum -a 256 "$file" | awk '{print $1}'
    else
        sha256sum "$file" | awk '{print $1}'
    fi
}

# Compute SHA1 checksum of a file
compute_sha1() {
    local file="$1"
    if [ ! -f "$file" ]; then
        echo "Error: File not found: $file" >&2
        return 1
    fi
    
    if [[ "$OSTYPE" == "darwin"* ]]; then
        shasum -a 1 "$file" | awk '{print $1}'
    else
        sha1sum "$file" | awk '{print $1}'
    fi
}

# Compute MD5 checksum of a file
compute_md5() {
    local file="$1"
    if [ ! -f "$file" ]; then
        echo "Error: File not found: $file" >&2
        return 1
    fi
    
    if [[ "$OSTYPE" == "darwin"* ]]; then
        md5 "$file" | awk '{print $NF}'
    else
        md5sum "$file" | awk '{print $1}'
    fi
}

# Compare checksums of two files using SHA256
compare_files_sha256() {
    local file1="$1"
    local file2="$2"
    
    local hash1=$(compute_sha256 "$file1")
    local hash2=$(compute_sha256 "$file2")
    
    if [ "$hash1" = "$hash2" ]; then
        return 0
    else
        echo "SHA256 mismatch: $hash1 != $hash2" >&2
        return 1
    fi
}

# Compare checksums of two files using SHA1
compare_files_sha1() {
    local file1="$1"
    local file2="$2"
    
    local hash1=$(compute_sha1 "$file1")
    local hash2=$(compute_sha1 "$file2")
    
    if [ "$hash1" = "$hash2" ]; then
        return 0
    else
        echo "SHA1 mismatch: $hash1 != $hash2" >&2
        return 1
    fi
}

# Compare checksums of two files using MD5
compare_files_md5() {
    local file1="$1"
    local file2="$2"
    
    local hash1=$(compute_md5 "$file1")
    local hash2=$(compute_md5 "$file2")
    
    if [ "$hash1" = "$hash2" ]; then
        return 0
    else
        echo "MD5 mismatch: $hash1 != $hash2" >&2
        return 1
    fi
}

# Compare files using all three algorithms
compare_files() {
    local file1="$1"
    local file2="$2"
    
    echo "Comparing files: $file1 vs $file2"
    
    local sha256_match=0
    local sha1_match=0
    local md5_match=0
    
    if compare_files_sha256 "$file1" "$file2"; then
        sha256_match=1
        echo "  ✓ SHA256 match"
    else
        echo "  ✗ SHA256 mismatch"
    fi
    
    if compare_files_sha1 "$file1" "$file2"; then
        sha1_match=1
        echo "  ✓ SHA1 match"
    else
        echo "  ✗ SHA1 mismatch"
    fi
    
    if compare_files_md5 "$file1" "$file2"; then
        md5_match=1
        echo "  ✓ MD5 match"
    else
        echo "  ✗ MD5 mismatch"
    fi
    
    if [ $sha256_match -eq 1 ] && [ $sha1_match -eq 1 ] && [ $md5_match -eq 1 ]; then
        return 0
    else
        return 1
    fi
}

# Get S3 object checksum via AWS CLI
# Note: S3 stores ETag which may not match file checksum for multipart uploads
get_s3_checksum() {
    local s3_url="$1"
    local algorithm="${2:-sha256}"  # sha256, sha1, or md5
    
    # Download to temp file and compute checksum
    local temp_file=$(mktemp)
    aws s3 cp "$s3_url" "$temp_file" --profile minio-test --endpoint-url http://localhost:9000 --no-verify-ssl >/dev/null 2>&1
    
    if [ $? -ne 0 ]; then
        rm -f "$temp_file"
        echo "Error: Failed to download from S3: $s3_url" >&2
        return 1
    fi
    
    local checksum
    case "$algorithm" in
        sha256)
            checksum=$(compute_sha256 "$temp_file")
            ;;
        sha1)
            checksum=$(compute_sha1 "$temp_file")
            ;;
        md5)
            checksum=$(compute_md5 "$temp_file")
            ;;
        *)
            echo "Error: Unknown algorithm: $algorithm" >&2
            rm -f "$temp_file"
            return 1
            ;;
    esac
    
    rm -f "$temp_file"
    echo "$checksum"
}

# Compare local file with S3 object
compare_file_with_s3() {
    local local_file="$1"
    local s3_url="$2"
    local algorithm="${3:-sha256}"
    
    local local_hash
    case "$algorithm" in
        sha256)
            local_hash=$(compute_sha256 "$local_file")
            ;;
        sha1)
            local_hash=$(compute_sha1 "$local_file")
            ;;
        md5)
            local_hash=$(compute_md5 "$local_file")
            ;;
        *)
            echo "Error: Unknown algorithm: $algorithm" >&2
            return 1
            ;;
    esac
    
    local s3_hash=$(get_s3_checksum "$s3_url" "$algorithm")
    
    if [ "$local_hash" = "$s3_hash" ]; then
        echo "✓ $algorithm match: $local_hash"
        return 0
    else
        echo "✗ $algorithm mismatch: local=$local_hash, s3=$s3_hash" >&2
        return 1
    fi
}

