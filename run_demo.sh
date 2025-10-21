#!/bin/bash

# Script to run the complete Phase 1 demo

echo "========================================"
echo "  Phase 1 Demo - Quick Start"
echo "========================================"
echo ""

# Build the project
echo "Building project..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

echo ""
echo "Running complete demo with:"
echo "  - 3 Cloud Nodes"
echo "  - 50 Concurrent Clients"
echo "  - 10,000 Total Requests"
echo "  - Automatic failure simulation"
echo ""
echo "This will take approximately 1-2 minutes..."
echo ""

# Run the demo
cargo run --release --bin demo

echo ""
echo "Demo completed!"
