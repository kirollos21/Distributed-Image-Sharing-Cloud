#!/bin/bash

# Script to start all cloud nodes in separate terminals

echo "Starting 3 Cloud Nodes..."

# Build first
cargo build --release

# Start nodes in background with logging
echo "Starting Node 1..."
cargo run --release --bin cloud-node 1 > node1.log 2>&1 &
NODE1_PID=$!

sleep 1

echo "Starting Node 2..."
cargo run --release --bin cloud-node 2 > node2.log 2>&1 &
NODE2_PID=$!

sleep 1

echo "Starting Node 3..."
cargo run --release --bin cloud-node 3 > node3.log 2>&1 &
NODE3_PID=$!

sleep 1

echo ""
echo "All nodes started!"
echo "  Node 1 (PID: $NODE1_PID) - log: node1.log"
echo "  Node 2 (PID: $NODE2_PID) - log: node2.log"
echo "  Node 3 (PID: $NODE3_PID) - log: node3.log"
echo ""
echo "To view logs:"
echo "  tail -f node1.log"
echo "  tail -f node2.log"
echo "  tail -f node3.log"
echo ""
echo "To stop all nodes:"
echo "  kill $NODE1_PID $NODE2_PID $NODE3_PID"
echo "  or run: pkill cloud-node"
echo ""

# Save PIDs to file for easy cleanup
echo "$NODE1_PID $NODE2_PID $NODE3_PID" > .node_pids

echo "PIDs saved to .node_pids"
echo "Nodes are running in background. Press Ctrl+C to exit this script (nodes will continue running)."
echo ""

# Wait for user interrupt
trap "echo 'Script exiting (nodes still running)...'; exit 0" INT
wait
