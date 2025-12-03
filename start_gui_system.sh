#!/bin/bash
# Startup script for the Distributed Image Cloud GUI system
# This script starts 3 cloud nodes, 1 server GUI, and 3 client GUIs

set -e

PROJECT_DIR="/media/kirollos/Data/Distributed Systems/Cloud Project"
cd "$PROJECT_DIR"

echo "========================================"
echo "  Distributed Image Cloud - GUI System"
echo "========================================"
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
if [ ! -f "target/release/cloud-node" ]; then
    echo "❌ Error: cloud-node binary not found. Please build first:"
    echo "   cargo build --release --bin cloud-node"
    exit 1
fi

if [ ! -f "target/release/server-gui" ]; then
    echo "❌ Error: server-gui binary not found. Please build first:"
    echo "   cargo build --release --bin server-gui"
    exit 1
fi

if [ ! -f "target/release/client-gui" ]; then
    echo "❌ Error: client-gui binary not found. Please build first:"
    echo "   cargo build --release --bin client-gui"
    exit 1
fi

# Check if encrypction_key.jpg exists
if [ ! -f "encrypction_key.jpg" ]; then
    echo "⚠️  Warning: encrypction_key.jpg not found!"
    echo "   The encryption system needs this file as the encryption key."
    echo "   Please add an image file named 'encrypction_key.jpg' to this directory."
    echo ""
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Kill any existing processes
echo "🧹 Cleaning up old processes..."
pkill -f cloud-node 2>/dev/null || true
pkill -f client-gui 2>/dev/null || true
pkill -f server-gui 2>/dev/null || true
sleep 2

echo "🚀 Starting cloud nodes in separate terminals..."
echo ""

# Start Node 1 in terminal
echo "   Starting Node 1 on 127.0.0.1:8001..."
open_terminal "Cloud Node 1" "cd '$PROJECT_DIR' && RUST_LOG=info ./target/release/cloud-node 1 127.0.0.1:8001 127.0.0.1:8002,127.0.0.1:8003"
sleep 1

# Start Node 2 in terminal
echo "   Starting Node 2 on 127.0.0.1:8002..."
open_terminal "Cloud Node 2" "cd '$PROJECT_DIR' && RUST_LOG=info ./target/release/cloud-node 2 127.0.0.1:8002 127.0.0.1:8001,127.0.0.1:8003"
sleep 1

# Start Node 3 in terminal
echo "   Starting Node 3 on 127.0.0.1:8003..."
open_terminal "Cloud Node 3" "cd '$PROJECT_DIR' && RUST_LOG=info ./target/release/cloud-node 3 127.0.0.1:8003 127.0.0.1:8001,127.0.0.1:8002"
sleep 3

echo ""
echo "⏳ Waiting for nodes to elect coordinator..."
sleep 2

echo ""
echo "🖥️  Starting Server GUI..."
RUST_LOG=info target/release/server-gui 127.0.0.1:8001 127.0.0.1:8002 127.0.0.1:8003 &
SERVER_PID=$!
echo "   ✓ Server GUI started (PID: $SERVER_PID)"
sleep 2

echo ""
echo "🖼️  Starting Client GUIs..."
echo ""

# Start Client 1
echo "   Starting Client 1..."
target/release/client-gui 1 127.0.0.1:8001 127.0.0.1:8002 127.0.0.1:8003 &
CLIENT1_PID=$!
sleep 1.5

# Start Client 2
echo "   Starting Client 2..."
target/release/client-gui 2 127.0.0.1:8001 127.0.0.1:8002 127.0.0.1:8003 &
CLIENT2_PID=$!
sleep 1.5

# Start Client 3
echo "   Starting Client 3..."
target/release/client-gui 3 127.0.0.1:8001 127.0.0.1:8002 127.0.0.1:8003 &
CLIENT3_PID=$!
sleep 1

echo ""
echo "========================================"
echo "  ✅ System Started Successfully!"
echo "========================================"
echo ""
echo "Running processes:"
echo "   • Cloud Node 1 - 127.0.0.1:8001 (in terminal)"
echo "   • Cloud Node 2 - 127.0.0.1:8002 (in terminal)"
echo "   • Cloud Node 3 - 127.0.0.1:8003 (in terminal)"
echo "   • Server GUI   (PID: $SERVER_PID)"
echo "   • Client 1 GUI (PID: $CLIENT1_PID)"
echo "   • Client 2 GUI (PID: $CLIENT2_PID)"
echo "   • Client 3 GUI (PID: $CLIENT3_PID)"
echo ""
echo "🖼️  Open Windows:"
echo "   • 3 Terminal windows (Node 1, 2, 3)"
echo "   • 1 Server Monitor GUI"
echo "   • 3 Client GUI windows"
echo ""
echo "🧪 Quick Start:"
echo "   1. Each client GUI will open - login with different usernames"
echo "      (e.g., 'alice', 'bob', 'charlie')"
echo "   2. Upload an image in any client"
echo "   3. Add other usernames to share with"
echo "   4. Encrypt and send!"
echo "   5. Check 'Received Images' tab in other clients"
echo ""
echo "🛑 To stop all processes:"
echo "   • Close the terminal windows for nodes"
echo "   • Or use: pkill -f 'cloud-node|server-gui|client-gui'"
echo "   • Or press Ctrl+C in this terminal"
echo ""
echo "💡 Tips:"
echo "   • Watch node terminals for real-time activity"
echo "   • Use Server Monitor to see cluster status"
echo "   • Try closing a node terminal to test fault tolerance"
echo ""

# Save PIDs to file for easy cleanup
echo "$SERVER_PID" > .pids
echo "$CLIENT1_PID" >> .pids
echo "$CLIENT2_PID" >> .pids
echo "$CLIENT3_PID" >> .pids

# Trap Ctrl+C to cleanup
trap "echo ''; echo '🛑 Stopping all processes...'; pkill -f cloud-node; kill $SERVER_PID $CLIENT1_PID $CLIENT2_PID $CLIENT3_PID 2>/dev/null; rm .pids 2>/dev/null; echo '✅ Cleanup complete'; exit 0" INT SIGTERM

# Keep script running
echo "Press Ctrl+C to stop all processes..."
wait
