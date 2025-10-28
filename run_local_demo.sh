#!/bin/bash

echo "=========================================="
echo "  Local Demo: 3 Nodes + 3 Clients"
echo "=========================================="
echo ""

# Check if project is built
if [ ! -f "target/release/cloud-node" ]; then
    echo "Project not built. Building now..."
    cargo build --release
    if [ $? -ne 0 ]; then
        echo "Build failed!"
        exit 1
    fi
fi

echo "Starting 3 cloud nodes and 3 client GUIs..."
echo ""
echo "This will open 7 windows:"
echo "  - 3 Cloud Nodes (Terminals)"
echo "  - 3 Client GUIs (Windows)"
echo "  - 1 Server Monitor (Window)"
echo ""
read -p "Press Enter to start..."

# Function to open new terminal
open_terminal() {
    local title=$1
    local command=$2

    if command -v gnome-terminal &> /dev/null; then
        gnome-terminal --title="$title" -- bash -c "$command; exec bash"
    elif command -v konsole &> /dev/null; then
        konsole --title "$title" -e bash -c "$command; exec bash" &
    elif command -v xterm &> /dev/null; then
        xterm -T "$title" -e bash -c "$command; exec bash" &
    else
        echo "No terminal emulator found. Run manually:"
        echo "  $command"
    fi
}

# Start Node 1
echo "Starting Node 1..."
open_terminal "Cloud Node 1" "cd '$PWD' && cargo run --release --bin cloud-node 1 127.0.0.1:8001 127.0.0.1:8002,127.0.0.1:8003"
sleep 2

# Start Node 2
echo "Starting Node 2..."
open_terminal "Cloud Node 2" "cd '$PWD' && cargo run --release --bin cloud-node 2 127.0.0.1:8002 127.0.0.1:8001,127.0.0.1:8003"
sleep 2

# Start Node 3
echo "Starting Node 3..."
open_terminal "Cloud Node 3" "cd '$PWD' && cargo run --release --bin cloud-node 3 127.0.0.1:8003 127.0.0.1:8001,127.0.0.1:8002"
sleep 3

echo "All nodes started! Waiting for initialization..."
sleep 3

# Start Client GUIs
echo "Starting Client GUI 1..."
cargo run --release --bin client-gui 1 &
CLIENT1_PID=$!
sleep 1

echo "Starting Client GUI 2..."
cargo run --release --bin client-gui 2 &
CLIENT2_PID=$!
sleep 1

echo "Starting Client GUI 3..."
cargo run --release --bin client-gui 3 &
CLIENT3_PID=$!
sleep 1

# Start Server Monitor
echo "Starting Server Monitor..."
cargo run --release --bin server-gui &
MONITOR_PID=$!

echo ""
echo "=========================================="
echo "  ✅ All Systems Running!"
echo "=========================================="
echo ""
echo "Cloud Nodes: Check the 3 terminal windows"
echo "Client GUIs: 3 windows should be open"
echo "Server Monitor: 1 window should be open"
echo ""
echo "To stop everything:"
echo "  - Close all GUI windows"
echo "  - Press Ctrl+C in each node terminal"
echo "  - Or run: pkill cloud-node"
echo ""
echo "PIDs:"
echo "  Client GUIs: $CLIENT1_PID, $CLIENT2_PID, $CLIENT3_PID"
echo "  Monitor: $MONITOR_PID"
echo ""
echo "Try uploading images from any Client GUI!"
echo "Watch the Server Monitor to see processing."
echo ""

# Save PIDs for cleanup
cat > .demo_pids << EOF
$CLIENT1_PID
$CLIENT2_PID
$CLIENT3_PID
$MONITOR_PID
EOF

echo "Press Ctrl+C to stop all GUI processes..."
trap "kill $CLIENT1_PID $CLIENT2_PID $CLIENT3_PID $MONITOR_PID 2>/dev/null; echo 'GUIs stopped'; exit 0" INT

wait
