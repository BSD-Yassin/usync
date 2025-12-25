#!/bin/bash
# MinIO setup and configuration for S3 testing

set -e

MINIO_CONTAINER="usync-minio-test"
MINIO_ENDPOINT="http://localhost:9000"
MINIO_CONSOLE="http://localhost:9001"
MINIO_ACCESS_KEY="minioadmin"
MINIO_SECRET_KEY="minioadmin"
TEST_BUCKET="test-bucket"
AWS_PROFILE="minio-test"

# Check if Podman or Docker is available
check_container_runtime() {
    # Prefer podman, fallback to docker
    if command -v podman &> /dev/null; then
        if podman info &> /dev/null; then
            CONTAINER_RUNTIME="podman"
            return 0
        fi
    fi
    
    if command -v docker &> /dev/null; then
        if docker info &> /dev/null; then
            CONTAINER_RUNTIME="docker"
            return 0
        fi
    fi
    
    echo "Error: Neither Podman nor Docker is available or running" >&2
    return 1
}

# Check if docker-compose or podman-compose is available
check_docker_compose() {
    # Check for podman compose (subcommand, preferred)
    if command -v podman &> /dev/null && podman compose version &> /dev/null 2>&1; then
        DOCKER_COMPOSE_CMD="podman compose"
        return 0
    # Check for podman-compose (standalone, if available)
    elif command -v podman-compose &> /dev/null; then
        DOCKER_COMPOSE_CMD="podman-compose"
        return 0
    # Check for docker-compose (standalone)
    elif command -v docker-compose &> /dev/null; then
        DOCKER_COMPOSE_CMD="docker-compose"
        return 0
    # Check for docker compose (subcommand)
    elif command -v docker &> /dev/null && docker compose version &> /dev/null 2>&1; then
        DOCKER_COMPOSE_CMD="docker compose"
        return 0
    else
        echo "Error: docker-compose or podman compose is not available" >&2
        return 1
    fi
}

# Wait for MinIO to be ready
wait_for_minio() {
    local max_attempts=30
    local attempt=0
    
    echo "Waiting for MinIO to be ready..."
    while [ $attempt -lt $max_attempts ]; do
        if curl -sf "${MINIO_ENDPOINT}/minio/health/live" > /dev/null 2>&1; then
            echo "✓ MinIO is ready"
            return 0
        fi
        attempt=$((attempt + 1))
        sleep 1
    done
    
    echo "Error: MinIO failed to start after ${max_attempts} seconds" >&2
    return 1
}

# Start MinIO container
start_minio() {
    if ! check_container_runtime; then
        return 1
    fi
    
    if ! check_docker_compose; then
        return 1
    fi
    
    # Check if container is already running
    if [ "$CONTAINER_RUNTIME" = "podman" ]; then
        if podman ps --format '{{.Names}}' | grep -q "^${MINIO_CONTAINER}$"; then
            echo "MinIO container is already running"
            wait_for_minio
            return 0
        fi
    else
        if docker ps --format '{{.Names}}' | grep -q "^${MINIO_CONTAINER}$"; then
            echo "MinIO container is already running"
            wait_for_minio
            return 0
        fi
    fi
    
    echo "Starting MinIO container..."
    cd "$(dirname "$0")"
    $DOCKER_COMPOSE_CMD -f docker-compose.minio-test.yml up -d minio
    
    wait_for_minio
    
    # Create test bucket
    create_test_bucket
    
    echo "✓ MinIO started successfully"
    echo "  API endpoint: ${MINIO_ENDPOINT}"
    echo "  Console: ${MINIO_CONSOLE}"
    echo "  Access Key: ${MINIO_ACCESS_KEY}"
    echo "  Secret Key: ${MINIO_SECRET_KEY}"
}

# Stop MinIO container
stop_minio() {
    if ! check_docker_compose; then
        return 1
    fi
    
    echo "Stopping MinIO container..."
    cd "$(dirname "$0")"
    $DOCKER_COMPOSE_CMD -f docker-compose.minio-test.yml stop minio 2>/dev/null || true
    echo "✓ MinIO stopped"
}

# Remove MinIO container and volumes
cleanup_minio() {
    if ! check_docker_compose; then
        return 1
    fi
    
    echo "Cleaning up MinIO container and volumes..."
    cd "$(dirname "$0")"
    $DOCKER_COMPOSE_CMD -f docker-compose.minio-test.yml down -v 2>/dev/null || true
    echo "✓ MinIO cleaned up"
}

# Create test bucket
create_test_bucket() {
    echo "Creating test bucket: ${TEST_BUCKET}"
    
    # Check if bucket already exists
    if aws s3 ls "s3://${TEST_BUCKET}" \
        --profile "${AWS_PROFILE}" \
        --endpoint-url "${MINIO_ENDPOINT}" \
        --no-verify-ssl \
        &>/dev/null 2>&1; then
        echo "✓ Test bucket already exists: ${TEST_BUCKET}"
        return 0
    fi
    
    # Use AWS CLI to create bucket
    local create_output=$(aws s3 mb "s3://${TEST_BUCKET}" \
        --profile "${AWS_PROFILE}" \
        --endpoint-url "${MINIO_ENDPOINT}" \
        --no-verify-ssl \
        2>&1)
    
    local create_exit=$?
    
    # Check if bucket was created or already exists
    if [ $create_exit -eq 0 ] || echo "$create_output" | grep -q "BucketAlreadyOwnedByYou\|BucketAlreadyExists"; then
        # Verify bucket exists
        if aws s3 ls "s3://${TEST_BUCKET}" \
            --profile "${AWS_PROFILE}" \
            --endpoint-url "${MINIO_ENDPOINT}" \
            --no-verify-ssl \
            &>/dev/null 2>&1; then
            echo "✓ Test bucket created: ${TEST_BUCKET}"
            return 0
        else
            echo "Warning: Bucket creation reported success but bucket not accessible" >&2
            echo "  Output: $create_output" >&2
            return 1
        fi
    else
        echo "Warning: Could not create bucket: $create_output" >&2
        return 1
    fi
}

# Configure AWS CLI for MinIO
configure_aws_cli() {
    if ! command -v aws &> /dev/null; then
        echo "Error: AWS CLI is not installed" >&2
        return 1
    fi
    
    echo "Configuring AWS CLI for MinIO..."
    
    # Configure AWS credentials for MinIO
    aws configure set aws_access_key_id "${MINIO_ACCESS_KEY}" --profile "${AWS_PROFILE}"
    aws configure set aws_secret_access_key "${MINIO_SECRET_KEY}" --profile "${AWS_PROFILE}"
    aws configure set region "us-east-1" --profile "${AWS_PROFILE}"
    
    # Note: AWS CLI doesn't support endpoint-url in profiles directly
    # We'll need to use environment variables or modify usync to pass --endpoint-url
    # For now, export environment variables that can be used
    export AWS_ENDPOINT_URL_S3="${MINIO_ENDPOINT}"
    export AWS_CA_BUNDLE=""  # Disable SSL verification (not ideal, but for local testing)
    
    # Test configuration
    if aws s3 ls \
        --profile "${AWS_PROFILE}" \
        --endpoint-url "${MINIO_ENDPOINT}" \
        --no-verify-ssl \
        &>/dev/null; then
        echo "✓ AWS CLI configured for MinIO"
        echo "  Note: Set AWS_ENDPOINT_URL_S3=${MINIO_ENDPOINT} for usync to use MinIO"
        return 0
    else
        echo "Warning: AWS CLI configuration test failed" >&2
        return 1
    fi
}

# Setup complete MinIO environment
setup_minio() {
    start_minio
    configure_aws_cli
    echo ""
    echo "MinIO setup complete!"
    echo "  Use profile '${AWS_PROFILE}' with AWS CLI"
    echo "  Set AWS_PROFILE=${AWS_PROFILE} for usync tests"
}

# Main execution
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    case "${1:-setup}" in
        start)
            start_minio
            ;;
        stop)
            stop_minio
            ;;
        cleanup)
            cleanup_minio
            ;;
        setup)
            setup_minio
            ;;
        *)
            echo "Usage: $0 {start|stop|cleanup|setup}"
            exit 1
            ;;
    esac
fi

