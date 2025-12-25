# S3 Testing with MinIO

This document describes how to run S3 integration tests using MinIO, a local S3-compatible object storage server.

## Overview

The S3 tests use MinIO running in a Docker container to provide a local S3-compatible endpoint for testing usync's S3 functionality without requiring AWS credentials or internet access.

## Prerequisites

- Podman and podman-compose (preferred) or Docker and docker-compose
- AWS CLI installed and configured
- usync binary built (`cargo build --release`)
- MinIO will be automatically started by the test scripts

## Quick Start

### Run All Tests (including S3)

```bash
./tests/test_runner.sh --all
```

### Run Only S3 Tests

```bash
./tests/test_runner.sh --s3
```

### Run S3 Tests Directly

```bash
./tests/s3_tests.sh target/release/usync
```

## MinIO Setup

### Manual Setup

If you need to set up MinIO manually:

```bash
# Start MinIO
./tests/s3_setup.sh start

# Configure AWS CLI for MinIO
./tests/s3_setup.sh setup

# Stop MinIO
./tests/s3_setup.sh stop

# Clean up (remove container and volumes)
./tests/s3_setup.sh cleanup
```

### Docker/Podman Compose

MinIO can also be started using docker-compose or podman-compose:

```bash
# Using podman-compose (preferred)
cd tests
podman-compose -f docker-compose.minio-test.yml up -d minio

# Or using docker-compose
cd tests
docker-compose -f docker-compose.minio-test.yml up -d minio
```

**Note**: The compose file is located in `tests/docker-compose.minio-test.yml` and is specifically for testing with Mock S3, not for production use.

## Configuration

### MinIO Credentials

Default credentials (for testing only):
- Access Key: `minioadmin`
- Secret Key: `minioadmin`
- Endpoint: `http://localhost:9000`
- Console: `http://localhost:9001`

### AWS CLI Profile

The tests use a separate AWS profile (`minio-test`) to avoid interfering with your regular AWS configuration. This profile is automatically configured by the setup script.

## Test Coverage

The S3 test suite includes:

### File Operations
- **S3-1**: Copy local file to S3
- **S3-2**: Copy file from S3 to local
- **S3-3**: Verify file checksums (SHA256, SHA1, MD5) after copy

### Directory Operations
- **S3-4**: Sync local directory to S3
- **S3-5**: Sync directory from S3 to local
- **S3-6**: Verify checksums for all files in directory

### Move Operations
- **S3-7**: Move local file to S3 (source deleted)
- **S3-8**: Move file from S3 to local
- **S3-9**: Verify file checksum after move operation

### Advanced Features
- **S3-10**: Copy files using wildcard patterns

## Checksum Verification

All file operations are verified using multiple checksum algorithms:
- **SHA256**: Most secure, recommended
- **SHA1**: Faster, good for verification
- **MD5**: Fastest, good for quick checks

The checksum utilities (`tests/checksum_utils.sh`) provide cross-platform support for macOS and Linux.

## Test Data

Test data is sourced from `tests/input/` directory:
- Single files: `file1.txt`, `file2.txt`
- Directories: `subdir/` with nested files
- Binary files: `binary.bin`

## Troubleshooting

### MinIO Not Starting

```bash
# Check if Podman/Docker is running
podman info  # or docker info

# Check if port 9000 is already in use
lsof -i :9000

# View MinIO logs
podman logs usync-minio-test  # or docker logs usync-minio-test
```

### AWS CLI Configuration Issues

```bash
# Verify AWS CLI is configured
aws configure list --profile minio-test

# Test connection to MinIO
aws s3 ls --profile minio-test --endpoint-url http://localhost:9000 --no-verify-ssl
```

### Checksum Mismatches

If checksum verification fails:
1. Verify MinIO is running and accessible
2. Check file permissions
3. Ensure test files haven't been modified
4. Try cleaning up and re-running tests

### Permission Issues

If you encounter permission issues:
```bash
# Make scripts executable
chmod +x tests/*.sh

# Check Podman/Docker permissions
podman ps  # or docker ps
```

## Environment Variables

The tests use the following environment variables:
- `AWS_PROFILE=minio-test` - AWS CLI profile for MinIO
- `AWS_ENDPOINT_URL_S3=http://localhost:9000` - MinIO endpoint

These are automatically set by the test scripts.

## Cleanup

After running tests, you can clean up:

```bash
# Stop MinIO
./tests/s3_setup.sh stop

# Remove container and volumes
./tests/s3_setup.sh cleanup

# Clean test output
rm -rf tests/s3_output
```

## Integration with CI/CD

For CI/CD pipelines, ensure:
1. Podman or Docker is available
2. AWS CLI is installed
3. MinIO container can be started
4. Ports 9000 and 9001 are available

Example CI configuration:
```yaml
services:
  - podman  # or docker

before_script:
  - cd tests && podman-compose -f docker-compose.minio-test.yml up -d minio
  - ./tests/s3_setup.sh setup

script:
  - ./tests/test_runner.sh --all
```

## Notes

- MinIO data is persisted in a container volume (`minio-data`)
- The compose file (`tests/docker-compose.minio-test.yml`) is for testing only - it provides a Mock S3 server
- The test bucket (`test-bucket`) is created automatically
- All S3 operations use the `minio-test` AWS profile
- SSL verification is disabled for local testing (`--no-verify-ssl`)
- Podman is preferred over Docker for better security and no daemon requirement

