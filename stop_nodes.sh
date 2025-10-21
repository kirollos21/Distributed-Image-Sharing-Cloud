#!/bin/bash

# Script to stop all running cloud nodes

echo "Stopping all cloud nodes..."

if [ -f .node_pids ]; then
    PIDS=$(cat .node_pids)
    echo "Killing processes: $PIDS"
    kill $PIDS 2>/dev/null
    rm .node_pids
    echo "Nodes stopped (from .node_pids)"
else
    echo "No .node_pids file found, trying pkill..."
    pkill -f "cloud-node" || killall cloud-node 2>/dev/null
    echo "Attempted to kill all cloud-node processes"
fi

sleep 2

# Check if any processes are still running
if pgrep -f "cloud-node" > /dev/null; then
    echo "Warning: Some cloud-node processes may still be running"
    echo "Running processes:"
    pgrep -af "cloud-node"
else
    echo "All cloud nodes stopped successfully!"
fi
