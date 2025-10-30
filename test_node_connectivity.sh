#!/bin/bash

echo "=========================================="
echo "  Node Connectivity Test"
echo "=========================================="
echo ""

# Check if node is running
NODE_PID=$(pgrep -f "cloud-node")
if [ -z "$NODE_PID" ]; then
    echo "❌ No cloud-node process found!"
    echo "   Start a node first with:"
    echo "   cargo run --release --bin cloud-node 1 0.0.0.0:8001 <peers>"
    exit 1
fi

echo "✅ Cloud node is running (PID: $NODE_PID)"
echo ""

# Check if ports are listening
echo "Checking listening ports..."
for port in 8001 8002 8003; do
    if sudo ss -ulnp | grep -q ":$port "; then
        echo "✅ Port $port is listening"
    else
        echo "⚠️  Port $port is not listening (might be on another machine)"
    fi
done
echo ""

# Check recent UDP activity
echo "Checking UDP activity (last 5 seconds)..."
timeout 5 sudo tcpdump -i any "udp and (port 8001 or port 8002 or port 8003)" -c 10 2>/dev/null &
TCPDUMP_PID=$!

sleep 5

if ps -p $TCPDUMP_PID > /dev/null 2>&1; then
    echo "⚠️  No UDP packets detected"
    echo "   This could mean:"
    echo "   - Other nodes haven't been started yet"
    echo "   - Network connectivity issues"
    echo "   - Firewall blocking traffic"
    kill $TCPDUMP_PID 2>/dev/null
else
    echo "✅ UDP packets detected!"
fi

echo ""
echo "=========================================="
echo "To see live logs, run on the node:"
echo "  journalctl -f | grep cloud-node"
echo ""
echo "Or check for election messages:"
echo "  tail -f /path/to/node/output | grep 'ELECTION'"
echo "=========================================="
