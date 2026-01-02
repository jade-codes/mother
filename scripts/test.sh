#!/bin/bash
# Test script for remember

set -e

echo "=== Remember Test Script ==="

# Check if Neo4j is running
if ! docker ps | grep -q neo4j; then
    echo "Starting Neo4j..."
    docker-compose up -d
    echo "Waiting for Neo4j to start..."
    sleep 10
fi

# Build
echo "Building..."
cargo build

# Run unit tests
echo "Running unit tests..."
cargo test

# Scan this repo (if Neo4j is ready)
echo "Scanning this repository..."
cargo run -- scan . \
  --neo4j-uri bolt://localhost:7687 \
  --neo4j-user neo4j \
  --neo4j-password remember_dev_password \
  --version "test-$(date +%Y%m%d-%H%M%S)"

echo "=== Done ==="
echo "View results at http://localhost:7474"
