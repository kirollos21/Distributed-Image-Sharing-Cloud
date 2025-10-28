#!/bin/bash

echo "=========================================="
echo "  Cleanup: Stop All Processes & Ports"
echo "=========================================="
echo ""

# Kill all cloud processes
echo "Killing all cloud-node processes..."
pkill cloud-node 2>/dev/null
if [ $? -eq 0 ]; then
    echo "  ✓ Cloud nodes stopped"
else
    echo "  - No cloud nodes running"
fi

echo "Killing all client-gui processes..."
pkill client-gui 2>/dev/null
if [ $? -eq 0 ]; then
    echo "  ✓ Client GUIs stopped"
else
    echo "  - No client GUIs running"
fi

echo "Killing all server-gui processes..."
pkill server-gui 2>/dev/null
if [ $? -eq 0 ]; then
    echo "  ✓ Server monitors stopped"
else
    echo "  - No server monitors running"
fi

echo ""
echo "Waiting for ports to be released..."
sleep 2

# Check if ports are still in use
echo ""
echo "Checking ports..."

for port in 8001 8002 8003; do
    if lsof -i :$port &>/dev/null 2>&1 || sudo lsof -i :$port &>/dev/null 2>&1; then
        echo "  ⚠ Port $port still in use!"
        echo "    Run: sudo lsof -i :$port"
    else
        echo "  ✓ Port $port is free"
    fi
done

# Remove PID files if they exist
if [ -f .demo_pids ]; then
    echo ""
    echo "Removing PID files..."
    rm .demo_pids
    echo "  ✓ PID files removed"
fi

if [ -f .node_pids ]; then
    rm .node_pids
    echo "  ✓ Node PID files removed"
fi

echo ""
echo "=========================================="
echo "  ✅ Cleanup Complete!"
echo "=========================================="
echo ""
echo "All processes stopped. Ready to restart."
echo ""
echo "To start nodes again:"
echo "  cargo run --release --bin cloud-node 1"
echo "  cargo run --release --bin cloud-node 2"
echo "  cargo run --release --bin cloud-node 3"
echo ""
