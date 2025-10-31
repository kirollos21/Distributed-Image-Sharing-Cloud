# Comprehensive Stress Testing Guide

## Overview

This guide covers different types of stress tests for the Distributed Image Cloud system:

1. **Load Testing**: Test system performance with increasing loads (10k, 12.5k, 15k, 17.5k, 20k images)
2. **Failure Testing**: Test fault tolerance with node failures (no failures, 1 node, 2 nodes)

## Test Scripts

- `stress_test_client.py`: Load testing with configurable image counts
- `stress_test_with_failures.py`: Failure simulation testing
- `analyze_stress_results.py`: Analyze and generate reports
- `run_stress_tests.sh`: Master orchestration script
- `create_test_image.py`: Generate small test images for UDP

## Prerequisites

### 1. Create Test Image

```bash
python3 create_test_image.py
```

This creates a small (~8KB) test image suitable for UDP transmission.

### 2. Start Server Nodes

Make sure failure simulation is **DISABLED** for load testing, **ENABLED** for failure testing.

**For Load Testing (no failures):**
```bash
# Edit src/node.rs and comment out failure_simulation_task
cargo build --release

# Terminal 1:
cargo run --release --bin cloud-node 1 0.0.0.0:8001 <peer2>:8002,<peer3>:8003

# Terminal 2:
cargo run --release --bin cloud-node 2 0.0.0.0:8002 <peer1>:8001,<peer3>:8003

# Terminal 3:
cargo run --release --bin cloud-node 3 0.0.0.0:8003 <peer1>:8001,<peer2>:8002
```

**For Failure Testing:**
```bash
# Edit src/node.rs and uncomment failure_simulation_task
cargo build --release
# Then start nodes same as above
```

## Test Type 1: Load Testing (No Failures)

### Quick Start

```bash
./run_stress_tests.sh
# Select option 1 (Load Test)
```

### Manual Execution

Run on each of 3 client machines:

**Client 1:**
```bash
python3 stress_test_client.py 1 \
  --servers "192.168.4.2:8001,192.168.4.3:8002,192.168.4.4:8003" \
  --username "user1" \
  --target-users "user2,user3" \
  --image "test_image.jpg" \
  --start 10000 \
  --end 20000 \
  --step 2500 \
  --gap 10 \
  --output "stress_results_client1.json"
```

**Client 2:**
```bash
python3 stress_test_client.py 2 \
  --servers "192.168.4.2:8001,192.168.4.3:8002,192.168.4.4:8003" \
  --username "user2" \
  --target-users "user1,user3" \
  --image "test_image.jpg" \
  --start 10000 \
  --end 20000 \
  --step 2500 \
  --gap 10 \
  --output "stress_results_client2.json"
```

**Client 3:**
```bash
python3 stress_test_client.py 3 \
  --servers "192.168.4.2:8001,192.168.4.3:8002,192.168.4.4:8003" \
  --username "user3" \
  --target-users "user1,user2" \
  --image "test_image.jpg" \
  --start 10000 \
  --end 20000 \
  --step 2500 \
  --gap 10 \
  --output "stress_results_client3.json"
```

### What This Does

Each client will send:
1. **10,000 images** → wait 10 seconds
2. **12,500 images** → wait 10 seconds
3. **15,000 images** → wait 10 seconds
4. **17,500 images** → wait 10 seconds
5. **20,000 images** → done

### Custom Ranges

Want different numbers? Use custom parameters:

```bash
# Test 5k, 10k, 15k, 20k, 25k
python3 stress_test_client.py 1 \
  --start 5000 \
  --end 25000 \
  --step 5000 \
  --gap 15

# Test only specific points
python3 stress_test_client.py 1 \
  --start 15000 \
  --end 15000 \
  --step 1 \
  --gap 0
```

## Test Type 2: Failure Testing

### Test 2A: Baseline (No Failures)

This establishes baseline performance before introducing failures.

**Client 1:**
```bash
python3 stress_test_with_failures.py 1 \
  --servers "192.168.4.2:8001,192.168.4.3:8002,192.168.4.4:8003" \
  --username "user1" \
  --target-users "user2,user3" \
  --num-images 15000 \
  --failure-mode none \
  --output "baseline_client1.json"
```

Repeat for clients 2 and 3.

### Test 2B: One Node Failure

Tests system resilience when one server fails.

**Setup:**
1. Start all 3 server nodes
2. Start all 3 clients with `--failure-mode one`
3. After 30 seconds, **manually stop 1 server node** (Ctrl+C)
4. After 60 seconds, **restart that node**
5. Let test complete

**Client 1:**
```bash
python3 stress_test_with_failures.py 1 \
  --servers "192.168.4.2:8001,192.168.4.3:8002,192.168.4.4:8003" \
  --username "user1" \
  --target-users "user2,user3" \
  --num-images 15000 \
  --failure-mode one \
  --failure-start 30 \
  --failure-duration 60 \
  --output "one_failure_client1.json"
```

**Timeline:**
- `t=0s`: Test starts, all 3 nodes running
- `t=30s`: Prompt appears → Stop 1 node (e.g., Node 2)
- `t=90s`: Restart the stopped node
- Test continues until all 15k images sent

### Test 2C: Two Node Failure

Tests system resilience when two servers fail simultaneously.

**Setup:**
1. Start all 3 server nodes
2. Start all 3 clients with `--failure-mode two`
3. After 30 seconds, **manually stop 2 server nodes**
4. After 60 seconds, **restart both nodes**
5. Let test complete

**Client 1:**
```bash
python3 stress_test_with_failures.py 1 \
  --servers "192.168.4.2:8001,192.168.4.3:8002,192.168.4.4:8003" \
  --username "user1" \
  --target-users "user2,user3" \
  --num-images 15000 \
  --failure-mode two \
  --failure-start 30 \
  --failure-duration 60 \
  --output "two_failure_client1.json"
```

**Timeline:**
- `t=0s`: Test starts, all 3 nodes running
- `t=30s`: Prompt appears → Stop 2 nodes (e.g., Node 2 and Node 3)
- `t=90s`: Restart both stopped nodes
- System now has only 1 node for 60 seconds
- Test continues until all 15k images sent

## Analyzing Results

### Analyze Single Test

```bash
python3 analyze_stress_results.py stress_results_client1.json
```

### Analyze All Clients Together

```bash
python3 analyze_stress_results.py stress_results_client*.json
```

### Analyze All Tests

```bash
python3 analyze_stress_results.py results_dir/*.json > full_analysis.txt
```

### Sample Output

```
================================================================================
STRESS TEST ANALYSIS REPORT
================================================================================
Generated: 2025-10-30 15:30:45
Clients: 3
================================================================================

────────────────────────────────────────────────────────────────────────────────
CLIENT 1 - 5 tests
────────────────────────────────────────────────────────────────────────────────
Images     Success    Failures   Rate       Avg Time        Throughput     
────────────────────────────────────────────────────────────────────────────────
10000      9998       2          99.98%       125.50ms        79.84 req/s
12500      12485      15         99.88%       135.20ms        92.34 req/s
15000      14950      50         99.67%       145.80ms        102.63 req/s
17500      17350      150        99.14%       158.30ms        109.75 req/s
20000      19650      350        98.25%       172.90ms        113.50 req/s

════════════════════════════════════════════════════════════════════════════════
AGGREGATE ANALYSIS (All Clients Combined)
════════════════════════════════════════════════════════════════════════════════
Images     Total Sent   Successes    Failures     Rate       Avg Time       
────────────────────────────────────────────────────────────────────────────────
10000      30000        29994        6            99.98%       125.45ms
12500      37500        37455        45           99.88%       135.15ms
15000      45000        44850        150          99.67%       145.75ms
17500      52500        52050        450          99.14%       158.25ms
20000      60000        58950        1050         98.25%       172.85ms
```

## Metrics Collected

For each test, the system collects:

1. **Success Count**: Number of images successfully encrypted/sent
2. **Failure Count**: Number of failed requests
3. **Success Rate**: Percentage of successful requests
4. **Average Response Time**: Mean time per request (milliseconds)
5. **Min/Max Response Time**: Range of response times
6. **Throughput**: Requests per second
7. **Total Test Time**: Duration of entire test

For failure tests, additionally:
- **Failures During Node Failure**: Requests that failed while nodes were down
- **Recovery Time**: Time for system to stabilize after node recovery

## Expected Behavior

### Load Testing

- **10k images**: ~99.9% success rate, fast response times
- **12.5k images**: ~99.8% success rate, slightly slower
- **15k images**: ~99.5% success rate, moderate slowdown
- **17.5k images**: ~98-99% success rate, increased latency
- **20k images**: ~95-98% success rate, significant latency

**Why failures increase:**
- UDP packet drops under high load
- Queue saturation on servers
- Network congestion
- Timeout issues

### Failure Testing

**Baseline (no failures):**
- Should match load test results at 15k images
- Establishes performance without disruption

**One node failure:**
- Brief spike in failures when node goes down (t=30s)
- System recovers using remaining 2 nodes
- Overall success rate: ~95-98%
- Throughput drops during failure, recovers after

**Two node failure:**
- More failures when 2 nodes go down (t=30s)
- System survives on 1 node (degraded performance)
- Overall success rate: ~90-95%
- Significant throughput drop during failure
- Full recovery after nodes restart

## Troubleshooting

### Issue: Too many failures even at low loads

**Solution:**
- Check network connectivity between clients and servers
- Verify UDP ports are open (8001, 8002, 8003)
- Reduce image size: `python3 create_test_image.py --size 5000`
- Increase timeout in stress_test_client.py line 28: `sock.settimeout(10.0)`

### Issue: Test hangs or times out

**Solution:**
- Check if server nodes are running
- Verify server addresses are correct
- Test connectivity: `nc -u <server_ip> <port>`
- Check server logs for errors

### Issue: Results show 0% success rate

**Solution:**
- Servers might be refusing connections
- Check if usernames are registered (run client GUI first to register)
- Verify target users exist in the system
- Check server logs for "username not found" errors

### Issue: Different clients show very different results

**Solution:**
- Check network quality for each client
- Verify all clients use same server list
- Check for network congestion on specific clients
- Run tests sequentially instead of parallel

## Advanced Testing Scenarios

### Scenario 1: Find Breaking Point

Gradually increase load until success rate drops below 90%:

```bash
python3 stress_test_client.py 1 \
  --start 5000 \
  --end 50000 \
  --step 5000 \
  --gap 10
```

### Scenario 2: Sustained Load

Test system stability under constant load:

```bash
python3 stress_test_client.py 1 \
  --start 15000 \
  --end 15000 \
  --step 1 \
  --gap 60  # Run same test multiple times
```

### Scenario 3: Cascading Failures

1. Start with 3 nodes
2. Begin sending 20k images
3. Stop node 1 at t=30s
4. Stop node 2 at t=60s
5. Restart node 1 at t=90s
6. Restart node 2 at t=120s

Observe how system degrades and recovers.

## Collecting Data for Research

### Complete Test Suite (Recommended)

Run in this order:

```bash
# 1. Load testing (no failures)
./run_stress_tests.sh  # Option 1

# 2. Baseline (15k, no failures)
./run_stress_tests.sh  # Option 3

# 3. One node failure (15k)
./run_stress_tests.sh  # Option 4

# 4. Two node failures (15k)
./run_stress_tests.sh  # Option 5

# 5. Analyze all results
./run_stress_tests.sh  # Option 7
```

This gives you:
- Performance under varying loads
- Baseline without failures
- Impact of single node failure
- Impact of multiple node failures
- Comparison data for research paper

---

**Ready to start?** Create your test image and run `./run_stress_tests.sh`!
