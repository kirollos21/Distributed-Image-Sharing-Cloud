#!/bin/bash

# Script to start complete distributed system on localhost:
# - 3 Cloud Nodes (in separate terminals)
# - 1 Server Monitor GUI
# - 3 Client GUIs
# Uses UDP protocol for communication

PROJECT_DIR="/media/kirollos/Data/Distributed Systems/Cloud Project"
cd "$PROJECT_DIR"

echo "=========================================="
echo "  Distributed Image Cloud - Local Demo"
echo "  3 Nodes + 1 Monitor + 3 Clients"
echo "=========================================="
echo ""

# Function to open new terminal with command
open_terminal() {
    local title=$1
    local command=$2

    if command -v gnome-terminal &> /dev/null; then
        gnome-terminal --title="$title" --geometry=100x30 -- bash -c "$command; echo ''; echo 'Press Enter to close...'; read; exit"
    elif command -v konsole &> /dev/null; then
        konsole --title "$title" -e bash -c "$command; echo ''; echo 'Press Enter to close...'; read; exit" &
    elif command -v xterm &> /dev/null; then
        xterm -T "$title" -geometry 100x30 -e bash -c "$command; echo ''; echo 'Press Enter to close...'; read; exit" &
    elif command -v x-terminal-emulator &> /dev/null; then
        x-terminal-emulator -T "$title" -e bash -c "$command; echo ''; echo 'Press Enter to close...'; read; exit" &
    else
        echo "⚠️  No terminal emulator found!"
        echo "Run this command manually in a new terminal:"
        echo "  $command"
        echo ""
    fi
    sleep 0.5
}

# Check if binaries exist
if [ ! -f "target/release/cloud-node" ] || [ ! -f "target/release/client-gui" ] || [ ! -f "target/release/server-gui" ]; then
    echo "❌ Some binaries not found. Building project..."
    echo "   This may take 5-10 minutes the first time..."
    cargo build --release --bin cloud-node --bin client-gui --bin server-gui
    if [ $? -ne 0 ]; then
        echo "❌ Build failed! Please fix errors and try again."
        exit 1
    fi
    echo "✅ Build complete!"
fi

echo "✅ All binaries found"
echo ""

# Kill any existing processes
echo "🧹 Cleaning up old processes..."
pkill -f cloud-node 2>/dev/null
pkill -f client-gui 2>/dev/null
pkill -f server-gui 2>/dev/null
sleep 2

echo ""
echo "🚀 Starting Cloud Nodes..."
echo ""

# Start Node 1
echo "  Starting Node 1 on 127.0.0.1:8001..."
open_terminal "Cloud Node 1" "cd '$PROJECT_DIR' && ./target/release/cloud-node 1 127.0.0.1:8001 127.0.0.1:8002,127.0.0.1:8003"
sleep 2

# Start Node 2
echo "  Starting Node 2 on 127.0.0.1:8002..."
open_terminal "Cloud Node 2" "cd '$PROJECT_DIR' && ./target/release/cloud-node 2 127.0.0.1:8002 127.0.0.1:8001,127.0.0.1:8003"
sleep 2

# Start Node 3
echo "  Starting Node 3 on 127.0.0.1:8003..."
open_terminal "Cloud Node 3" "cd '$PROJECT_DIR' && ./target/release/cloud-node 3 127.0.0.1:8003 127.0.0.1:8001,127.0.0.1:8002"
sleep 3

echo ""
echo "⏳ Waiting for nodes to initialize..."
sleep 2

echo ""
echo "📊 Starting Server Monitor GUI..."
echo ""

# Start Server Monitor
if [ -f "target/release/server-gui" ]; then
    echo "  Starting Server Monitor..."
    ./target/release/server-gui &
    MONITOR_PID=$!
    sleep 1.5
else
    echo "  ⚠️  Server GUI not built. Run: cargo build --release --bin server-gui"
    MONITOR_PID=""
fi

echo ""
echo "🖼️  Starting Client GUIs..."
echo ""

# Start Client 1
echo "  Starting Client 1..."
./target/release/client-gui 1 &
CLIENT1_PID=$!
sleep 1.5

# Start Client 2
echo "  Starting Client 2..."
./target/release/client-gui 2 &
CLIENT2_PID=$!
sleep 1.5

# Start Client 3
echo "  Starting Client 3..."
./target/release/client-gui 3 &
CLIENT3_PID=$!
sleep 1

echo ""
echo "=========================================="
echo "  ✅ ALL SYSTEMS RUNNING!"
echo "=========================================="
echo ""
echo "📊 System Status:"
echo "  • 3 Cloud Nodes running in separate terminals"
echo "  • 3 Client GUIs running (PID: $CLIENT1_PID, $CLIENT2_PID, $CLIENT3_PID)"
if [ -n "$MONITOR_PID" ]; then
    echo "  • 1 Server Monitor GUI (PID: $MONITOR_PID)"
fi
echo ""
echo "🖼️  Open Windows:"
echo "  • 3 Terminal windows (Node 1, 2, 3)"
echo "  • 3 Client GUI windows"
if [ -n "$MONITOR_PID" ]; then
    echo "  • 1 Server Monitor window"
fi
echo ""
echo "🧪 Test the System:"
echo "  1. Check Server Monitor to see cluster status"
echo "  2. Login to each client with different usernames"
echo "     (e.g., 'alice', 'bob', 'charlie')"
echo "  3. Upload and encrypt images"
echo "  4. Watch the node terminals for activity"
echo "  5. View logs and metrics in Server Monitor"
echo ""
echo "🛑 To Stop Everything:"
echo "  • Close each terminal window"
echo "  • Or run: pkill cloud-node; pkill client-gui; pkill server-gui"
echo ""
echo "💡 Tips:"
echo "  • Try closing a node terminal to test fault tolerance"
echo "  • Watch load distribution across nodes"
echo "  • Test username uniqueness (try same username twice)"
echo "  • Server Monitor shows real-time cluster status"
echo ""

# Trap Ctrl+C to cleanup
if [ -n "$MONITOR_PID" ]; then
    trap "echo ''; echo '🛑 Stopping all processes...'; pkill cloud-node; kill $CLIENT1_PID $CLIENT2_PID $CLIENT3_PID $MONITOR_PID 2>/dev/null; echo '✅ Cleanup complete'; exit 0" INT SIGTERM
else
    trap "echo ''; echo '🛑 Stopping all processes...'; pkill cloud-node; kill $CLIENT1_PID $CLIENT2_PID $CLIENT3_PID 2>/dev/null; echo '✅ Cleanup complete'; exit 0" INT SIGTERM
fi

# Keep script running
echo "Press Ctrl+C to stop all processes..."
wait
