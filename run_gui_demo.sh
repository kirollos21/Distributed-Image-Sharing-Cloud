#!/bin/bash

echo "========================================="
echo "  Distributed Image Cloud - GUI Demo"
echo "========================================="
echo ""

# Build the GUIs
echo "Building GUIs..."
cargo build --release --bin client-gui --bin server-gui

if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

echo ""
echo "Starting GUI applications..."
echo ""
echo "This script will open:"
echo "  1. Client GUI - for uploading/encrypting images"
echo "  2. Server Monitor GUI - for monitoring Node 2"
echo ""
echo "Make sure cloud nodes are running first!"
echo "  Terminal 1: cargo run --release --bin cloud-node 1"
echo "  Terminal 2: cargo run --release --bin cloud-node 2"
echo "  Terminal 3: cargo run --release --bin cloud-node 3"
echo ""
read -p "Press Enter to launch GUIs (or Ctrl+C to cancel)..."

# Launch Client GUI
echo "Launching Client GUI..."
cargo run --release --bin client-gui &
CLIENT_PID=$!
sleep 2

# Launch Server Monitor for Node 2
echo "Launching Server Monitor (standalone mode)..."
cargo run --release --bin server-gui &
SERVER_PID=$!

echo ""
echo "GUIs launched!"
echo "  Client GUI PID: $CLIENT_PID"
echo "  Server Monitor PID: $SERVER_PID"
echo ""
echo "To close: Press Ctrl+C or close the GUI windows"
echo ""

# Wait for user interrupt
trap "echo ''; echo 'Closing GUIs...'; kill $CLIENT_PID $SERVER_PID 2>/dev/null; exit 0" INT

wait
