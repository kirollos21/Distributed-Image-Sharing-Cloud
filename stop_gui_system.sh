#!/bin/bash
# Stop script for the Distributed Image Cloud GUI system

echo "🛑 Stopping Distributed Image Cloud system..."
echo ""

# Check if .pids file exists
if [ -f ".pids" ]; then
    echo "Stopping processes from .pids file..."
    while read pid; do
        if ps -p $pid > /dev/null 2>&1; then
            echo "   Killing process $pid..."
            kill $pid 2>/dev/null || true
        fi
    done < .pids
    rm .pids
    echo "   ✓ Processes stopped"
else
    echo "No .pids file found. Trying to kill by name..."
    pkill -f 'cloud-node|server-gui|client-gui' 2>/dev/null || true
    echo "   ✓ Done"
fi

echo ""
echo "✅ System stopped"
