#!/bin/bash
# Master Stress Testing Script
# Orchestrates different types of stress tests

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_DIR="stress_test_results_$(date +%Y%m%d_%H%M%S)"

mkdir -p "$RESULTS_DIR"

echo "========================================================================"
echo "  DISTRIBUTED IMAGE CLOUD - MASTER STRESS TEST"
echo "========================================================================"
echo ""
echo "This script helps you run comprehensive stress tests"
echo "Results will be saved to: $RESULTS_DIR/"
echo ""

# Function to display menu
show_menu() {
    echo ""
    echo "========================================================================"
    echo "  DISTRIBUTED IMAGE CLOUD - STRESS TESTING MENU"
    echo "========================================================================"
    echo ""
    echo "  1) Standard Load Test (10k to 20k images, step 2.5k)"
    echo "  2) Custom Load Test (specify range and step)"
    echo "  3) Concurrent Client Test (flexible: N clients × M requests each)"
    echo "  4) Failure Test: Baseline (no failures)"
    echo "  5) Failure Test: One Node Failure"
    echo "  6) Failure Test: Two Node Failures"
    echo "  7) Full Test Suite (all of the above)"
    echo "  8) Analyze Results"
    echo "  9) Exit"
    echo ""
    echo "========================================================================"
    echo -n "Enter your choice (1-9): "
}

# Function to get test parameters
get_test_params() {
    echo ""
    echo "Enter test parameters:"
    echo ""
    
    read -p "Number of clients (1-3): " NUM_CLIENTS
    read -p "Server addresses (comma-separated, e.g., 192.168.4.2:8001,192.168.4.3:8002,192.168.4.4:8003): " SERVERS
    
    echo ""
    echo "Usernames for each client:"
    for i in $(seq 1 $NUM_CLIENTS); do
        read -p "  Client $i username: " username
        USERNAMES[$i]=$username
    done
    
    echo ""
    read -p "Target users (comma-separated, users who will receive images): " TARGET_USERS
    read -p "Test image path (default: test_image.jpg): " TEST_IMAGE
    TEST_IMAGE=${TEST_IMAGE:-test_image.jpg}
    
    if [ ! -f "$TEST_IMAGE" ]; then
        echo ""
        echo "ERROR: Test image not found: $TEST_IMAGE"
        echo "Creating a small test image..."
        python3 create_test_image.py
        TEST_IMAGE="test_image.jpg"
    fi
}

# Function to run load test
run_load_test() {
    local start=$1
    local end=$2
    local step=$3
    local gap=$4
    
    echo ""
    echo "========================================================================"
    echo "  LOAD TEST: ${start}k to ${end}k images"
    echo "========================================================================"
    echo ""
    
    for i in $(seq 1 $NUM_CLIENTS); do
        local client_output="$RESULTS_DIR/load_test_client${i}.json"
        local username="${USERNAMES[$i]}"
        
        echo "Starting Client $i (${username})..."
        echo "  Command: python3 stress_test_client.py $i \\"
        echo "    --servers \"$SERVERS\" \\"
        echo "    --username \"$username\" \\"
        echo "    --target-users \"$TARGET_USERS\" \\"
        echo "    --image \"$TEST_IMAGE\" \\"
        echo "    --start $start \\"
        echo "    --end $end \\"
        echo "    --step $step \\"
        echo "    --gap $gap \\"
        echo "    --output \"$client_output\""
        echo ""
    done
    
    echo "Run the above commands in separate terminals (one per client)"
    echo "Press Enter when all tests are complete..."
    read
}

# Function to run concurrent client test
run_concurrent_test() {
    local num_clients=$1
    local requests_per_client=$2
    
    echo ""
    echo "========================================================================"
    echo "  CONCURRENT CLIENT TEST"
    echo "========================================================================"
    echo "  Clients: $num_clients concurrent threads"
    echo "  Requests per client: $requests_per_client"
    echo "  Total requests: $((num_clients * requests_per_client))"
    echo "========================================================================"
    echo ""
    
    for i in $(seq 1 $NUM_CLIENTS); do
        local username="${USERNAMES[$i]}"
        local timestamp=$(date +%Y%m%d_%H%M%S)
        local client_output="$RESULTS_DIR/concurrent_${num_clients}x${requests_per_client}_user${username}_${timestamp}.json"
        
        echo "User $i (${username}) command:"
        echo "  python3 stress_test_concurrent.py \\"
        echo "    --clients $num_clients \\"
        echo "    --requests-per-client $requests_per_client \\"
        echo "    --servers \"$SERVERS\" \\"
        echo "    --username \"$username\" \\"
        echo "    --target-users \"$TARGET_USERS\" \\"
        echo "    --image \"$TEST_IMAGE\" \\"
        echo "    --output \"$client_output\""
        echo ""
    done
    
    echo "INSTRUCTIONS:"
    echo "1. Start all server nodes in separate terminals"
    echo "2. Run the above command(s) in separate terminal(s)"
    echo "3. Each command will simulate $num_clients concurrent clients"
    echo "4. Each client will send $requests_per_client requests"
    echo "5. Results will show throughput, response times, and load distribution"
    echo ""
    
    echo "Press Enter when test is complete..."
    read
}

# Function to run failure test
run_failure_test() {
    local mode=$1
    local num_images=${2:-15000}
    
    echo ""
    echo "========================================================================"
    echo "  FAILURE TEST: Mode=$mode, Images=$num_images"
    echo "========================================================================"
    echo ""
    
    for i in $(seq 1 $NUM_CLIENTS); do
        local client_output="$RESULTS_DIR/failure_test_${mode}_client${i}.json"
        local username="${USERNAMES[$i]}"
        
        echo "Client $i (${username}) command:"
        echo "  python3 stress_test_with_failures.py $i \\"
        echo "    --servers \"$SERVERS\" \\"
        echo "    --username \"$username\" \\"
        echo "    --target-users \"$TARGET_USERS\" \\"
        echo "    --image \"$TEST_IMAGE\" \\"
        echo "    --num-images $num_images \\"
        echo "    --failure-mode $mode \\"
        echo "    --output \"$client_output\""
        echo ""
    done
    
    if [ "$mode" != "none" ]; then
        echo "IMPORTANT INSTRUCTIONS:"
        echo "1. Start all 3 server nodes in separate terminals"
        echo "2. Run the above client commands in separate terminals"
        echo "3. After 30 seconds, you'll be prompted to STOP node(s)"
        echo "4. For mode='one': Stop 1 node with Ctrl+C"
        echo "5. For mode='two': Stop 2 nodes with Ctrl+C"
        echo "6. After 60 seconds, RESTART the stopped node(s)"
        echo "7. Let the test complete"
        echo ""
    fi
    
    echo "Press Enter when all tests are complete..."
    read
}

# Main script
echo "First, let's configure the test environment..."
get_test_params

while true; do
    show_menu
    read choice
    
    case $choice in
        1)
            echo ""
            echo "Running standard load test (10k to 20k, step 2.5k)..."
            run_load_test 10000 20000 2500 10
            ;;
        2)
            echo ""
            read -p "Start number of images: " start
            read -p "End number of images: " end
            read -p "Step size: " step
            read -p "Gap between tests (seconds): " gap
            run_load_test $start $end $step $gap
            ;;
        3)
            echo ""
            read -p "Number of concurrent clients (threads): " num_clients
            read -p "Number of requests per client: " requests_per_client
            run_concurrent_test $num_clients $requests_per_client
            ;;
        4)
            read -p "Number of images (default 15000): " num
            num=${num:-15000}
            run_failure_test "none" $num
            ;;
        5)
            read -p "Number of images (default 15000): " num
            num=${num:-15000}
            run_failure_test "one" $num
            ;;
        6)
            read -p "Number of images (default 15000): " num
            num=${num:-15000}
            run_failure_test "two" $num
            ;;
        7)
            echo ""
            echo "========================================================================"
            echo "  FULL TEST SUITE"
            echo "========================================================================"
            echo ""
            echo "This will run:"
            echo "  1. Load test: 10k, 12.5k, 15k, 17.5k, 20k images"
            echo "  2. Failure test: baseline (no failures)"
            echo "  3. Failure test: one node failure"
            echo "  4. Failure test: two node failures"
            echo ""
            read -p "Continue? (y/n): " confirm
            if [ "$confirm" = "y" ]; then
                echo ""
                echo "Step 1: Load Test"
                run_load_test 10000 20000 2500 10
                
                echo ""
                echo "Step 2: Baseline (no failures)"
                run_failure_test "none" 15000
                
                echo ""
                echo "Step 3: One node failure"
                run_failure_test "one" 15000
                
                echo ""
                echo "Step 4: Two node failures"
                run_failure_test "two" 15000
                
                echo ""
                echo "Full test suite complete!"
            fi
            ;;
        8)
            echo ""
            echo "Analyzing results in $RESULTS_DIR..."
            if ls "$RESULTS_DIR"/*.json 1> /dev/null 2>&1; then
                python3 analyze_stress_results.py "$RESULTS_DIR"/*.json | tee "$RESULTS_DIR/ANALYSIS_REPORT.txt"
                echo ""
                echo "Analysis saved to: $RESULTS_DIR/ANALYSIS_REPORT.txt"
            else
                echo "No result files found in $RESULTS_DIR"
            fi
            ;;
        9)
            echo ""
            echo "Exiting..."
            exit 0
            ;;
        *)
            echo "Invalid choice. Please enter 1-9."
            ;;
    esac
done
