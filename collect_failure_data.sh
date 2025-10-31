#!/bin/bash
# Data Collection Script for Distributed Image Cloud
# Monitors system behavior during failure simulation

set -e

COLLECTION_DIR="failure_data_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$COLLECTION_DIR"

echo "================================================"
echo "  Failure Simulation Data Collection"
echo "================================================"
echo ""
echo "This script will collect system metrics while nodes"
echo "simulate failures and recoveries."
echo ""
echo "Data will be saved to: $COLLECTION_DIR/"
echo ""

# Function to monitor a single node's output
monitor_node() {
    local node_id=$1
    local port=$2
    local logfile="$COLLECTION_DIR/node_${node_id}.log"
    
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Node $node_id monitoring started" >> "$logfile"
}

# Function to extract metrics from logs
analyze_logs() {
    echo ""
    echo "Analyzing collected data..."
    
    local summary_file="$COLLECTION_DIR/SUMMARY.txt"
    
    cat > "$summary_file" << EOF
================================================
  FAILURE SIMULATION ANALYSIS SUMMARY
================================================
Generated: $(date)

EOF
    
    # Count failures, recoveries, elections per node
    for log in "$COLLECTION_DIR"/node_*.log; do
        if [ -f "$log" ]; then
            local node_name=$(basename "$log" .log)
            echo "--- $node_name ---" >> "$summary_file"
            
            local failures=$(grep -c "FAILED state" "$log" 2>/dev/null || echo "0")
            local recoveries=$(grep -c "RECOVERING state" "$log" 2>/dev/null || echo "0")
            local active=$(grep -c "ACTIVE state" "$log" 2>/dev/null || echo "0")
            local elections=$(grep -c "Starting election" "$log" 2>/dev/null || echo "0")
            local coordinator=$(grep -c "COORDINATOR" "$log" 2>/dev/null || echo "0")
            
            echo "  Failures: $failures" >> "$summary_file"
            echo "  Recoveries: $recoveries" >> "$summary_file"
            echo "  Returns to Active: $active" >> "$summary_file"
            echo "  Elections Initiated: $elections" >> "$summary_file"
            echo "  Times as Coordinator: $coordinator" >> "$summary_file"
            echo "" >> "$summary_file"
        fi
    done
    
    # System-wide metrics
    echo "--- SYSTEM METRICS ---" >> "$summary_file"
    local total_failures=$(grep -h "FAILED state" "$COLLECTION_DIR"/node_*.log 2>/dev/null | wc -l || echo "0")
    local total_elections=$(grep -h "Starting election" "$COLLECTION_DIR"/node_*.log 2>/dev/null | wc -l || echo "0")
    
    echo "  Total Node Failures: $total_failures" >> "$summary_file"
    echo "  Total Elections: $total_elections" >> "$summary_file"
    echo "" >> "$summary_file"
    
    # Election timeline
    echo "--- ELECTION TIMELINE ---" >> "$summary_file"
    grep -h "All node loads" "$COLLECTION_DIR"/node_*.log 2>/dev/null | tail -20 >> "$summary_file" || true
    
    cat "$summary_file"
    echo ""
    echo "Full report saved to: $summary_file"
}

# Trap Ctrl+C to perform cleanup and analysis
cleanup() {
    echo ""
    echo "Stopping data collection..."
    analyze_logs
    echo ""
    echo "Data collection complete!"
    echo "Files saved in: $COLLECTION_DIR/"
    exit 0
}

trap cleanup SIGINT SIGTERM

# Instructions for manual monitoring
echo "MONITORING INSTRUCTIONS:"
echo "========================"
echo ""
echo "1. Start your nodes in separate terminals:"
echo ""
echo "   Terminal 1:"
echo "   cargo run --release --bin cloud-node 1 0.0.0.0:8001 <peer2>:8002,<peer3>:8003 2>&1 | tee $COLLECTION_DIR/node_1.log"
echo ""
echo "   Terminal 2:"
echo "   cargo run --release --bin cloud-node 2 0.0.0.0:8002 <peer1>:8001,<peer3>:8003 2>&1 | tee $COLLECTION_DIR/node_2.log"
echo ""
echo "   Terminal 3:"
echo "   cargo run --release --bin cloud-node 3 0.0.0.0:8003 <peer1>:8001,<peer2>:8002 2>&1 | tee $COLLECTION_DIR/node_3.log"
echo ""
echo "2. Let the system run for at least 5-10 minutes to collect meaningful data"
echo ""
echo "3. Watch for these events in the logs:"
echo "   - '*** Entering FAILED state ***' - Node failure"
echo "   - '*** Entering RECOVERING state ***' - Recovery begins"
echo "   - '*** Returning to ACTIVE state ***' - Node fully recovered"
echo "   - 'Starting election' - New election triggered"
echo "   - 'New COORDINATOR' - Leader change"
echo ""
echo "4. Press Ctrl+C in THIS terminal when done to analyze results"
echo ""
echo "Waiting for Ctrl+C to analyze data..."
echo "(Start your nodes in other terminals now)"
echo ""

# Keep script running until user stops it
while true; do
    sleep 5
    # Check if any log files exist
    if ls "$COLLECTION_DIR"/node_*.log 1> /dev/null 2>&1; then
        # Show live summary
        local total_events=$(cat "$COLLECTION_DIR"/node_*.log 2>/dev/null | wc -l || echo "0")
        echo -ne "\rEvents logged: $total_events | Press Ctrl+C to stop and analyze"
    fi
done
