# Concurrent Stress Testing Guide

## Overview

The concurrent stress testing system allows you to simulate multiple simultaneous clients sending requests to your distributed image sharing cloud. This is more realistic than sequential testing because it creates real concurrent load.

## Quick Start

### Using the Menu System

```bash
./run_stress_tests.sh
```

Select option **3) Concurrent Client Test** and you'll be prompted for:
- **Number of concurrent clients (threads)**: How many simultaneous clients to simulate
- **Number of requests per client**: How many requests each client should send

**Example**: 10 clients × 5 requests = 50 total requests sent concurrently

### Direct Command Line Usage

```bash
python3 stress_test_concurrent.py \
  --clients 10 \
  --requests-per-client 5 \
  --servers "10.40.59.43:8001,10.40.44.230:8002" \
  --username "1" \
  --target-users "2,3" \
  --image "test_image.jpg"
```

## Common Test Scenarios

### 1. Light Load Test (10 concurrent clients)
```bash
python3 stress_test_concurrent.py \
  --clients 10 \
  --requests-per-client 5 \
  --servers "10.40.59.43:8001,10.40.44.230:8002" \
  --username "1" \
  --target-users "2,3" \
  --image "test_image.jpg"
```
- **Total requests**: 50
- **Use case**: Quick smoke test
- **Expected duration**: ~5-10 seconds

### 2. Medium Load Test (100 concurrent clients)
```bash
python3 stress_test_concurrent.py \
  --clients 100 \
  --requests-per-client 10 \
  --servers "10.40.59.43:8001,10.40.44.230:8002" \
  --username "1" \
  --target-users "2,3" \
  --image "test_image.jpg"
```
- **Total requests**: 1,000
- **Use case**: Realistic concurrent load
- **Expected duration**: ~30-60 seconds

### 3. Heavy Load Test (1000 concurrent clients)
```bash
python3 stress_test_concurrent.py \
  --clients 1000 \
  --requests-per-client 10 \
  --servers "10.40.59.43:8001,10.40.44.230:8002" \
  --username "1" \
  --target-users "2,3" \
  --image "test_image.jpg"
```
- **Total requests**: 10,000
- **Use case**: Stress test for capacity planning
- **Expected duration**: ~2-5 minutes

### 4. Extreme Load Test (5000 concurrent clients)
```bash
python3 stress_test_concurrent.py \
  --clients 5000 \
  --requests-per-client 2 \
  --servers "10.40.59.43:8001,10.40.44.230:8002" \
  --username "1" \
  --target-users "2,3" \
  --image "test_image.jpg"
```
- **Total requests**: 10,000 (same as above but more concurrent)
- **Use case**: Test system behavior under extreme concurrency
- **Expected duration**: ~3-7 minutes

## Understanding the Results

### Sample Output

```
======================================================================
CONCURRENT STRESS TEST
======================================================================
Number of Clients: 10
Requests per Client: 5
Total Requests: 50
Username: 1
Target Users: ['2', '3']
Servers: ['10.40.59.43:8001', '10.40.44.230:8002']
======================================================================

Starting 10 concurrent clients...

Waiting for all clients to complete...

Progress: 10/10 clients completed

======================================================================
TEST COMPLETE - 10 clients × 5 requests = 50 total
======================================================================

SUCCESS METRICS:
  Success: 50/50 (100.00%)
  Failures: 0

TIMING METRICS:
  Total test time: 3.45s
  Throughput: 14.49 requests/sec

RESPONSE TIME STATISTICS:
  Average: 234.56ms
  Median: 221.34ms
  Min: 187.23ms
  Max: 312.45ms
  Std Dev: 32.11ms
  P95: 289.12ms
  P99: 305.67ms

SERVER DISTRIBUTION:
  10.40.59.43:8001: 25 requests (50.0%)
  10.40.44.230:8002: 25 requests (50.0%)
======================================================================
```

### Key Metrics Explained

1. **Success Rate**: Percentage of requests that completed successfully
   - **Good**: 95%+
   - **Acceptable**: 90-95%
   - **Problem**: < 90%

2. **Throughput**: Requests processed per second
   - Shows system capacity under concurrent load

3. **Response Times**:
   - **Average**: Mean response time
   - **Median**: Middle value (50th percentile)
   - **P95**: 95% of requests completed in this time or less
   - **P99**: 99% of requests completed in this time or less
   - **Std Dev**: Consistency indicator (lower is better)

4. **Server Distribution**: How requests were distributed across nodes
   - Should be relatively balanced for good load balancing
   - Example: 50/50 split for 2 nodes, 33/33/33 for 3 nodes

## Comparison: Concurrent vs Sequential

### Sequential (Old Approach)
```bash
# Sends 50 requests one after another
python3 stress_test_client.py 1 \
  --servers "..." \
  --username "1" \
  --target-users "2,3" \
  --start 50 --end 50 --step 1
```
- **Duration**: Each request waits for previous to complete (~200ms × 50 = 10 seconds)
- **Load pattern**: Steady, predictable
- **Use case**: Throughput testing

### Concurrent (New Approach)
```bash
# Sends 50 requests simultaneously (10 clients × 5 each)
python3 stress_test_concurrent.py \
  --clients 10 \
  --requests-per-client 5 \
  --servers "..." \
  --username "1" \
  --target-users "2,3"
```
- **Duration**: All requests sent at once (~3-5 seconds total)
- **Load pattern**: Burst, realistic
- **Use case**: Stress testing, real-world simulation

## Test Planning

### How to Choose Parameters

**Formula**: `Total Requests = Clients × Requests per Client`

**Guidelines**:

1. **For quick testing**:
   - Use fewer clients (10-50)
   - Use 1-5 requests per client
   - Example: 10 clients × 5 requests = 50 total

2. **For realistic load**:
   - Use moderate clients (100-500)
   - Use 10-20 requests per client
   - Example: 100 clients × 10 requests = 1,000 total

3. **For stress testing**:
   - Use many clients (1000-5000)
   - Use 2-10 requests per client
   - Example: 1000 clients × 10 requests = 10,000 total

**Note**: More concurrent clients creates more simultaneous load, which tests different aspects than the same total requests sent sequentially.

## Integration with Existing Scripts

### Standalone Scripts

You can still use the specialized scripts directly:

```bash
# 10 concurrent clients (1 request each)
python3 stress_test_10.py \
  --servers "10.40.59.43:8001,10.40.44.230:8002" \
  --username "1" \
  --target-users "2,3"

# 1000 concurrent clients (1 request each)
python3 stress_test_1000.py \
  --servers "10.40.59.43:8001,10.40.44.230:8002" \
  --username "1" \
  --target-users "2,3"
```

### Menu-Driven System

```bash
./run_stress_tests.sh
```

Options:
1. Standard Load Test - Sequential, large volume (10k-20k)
2. Custom Load Test - Sequential, custom range
3. **Concurrent Client Test** - Simultaneous clients (NEW!)
4-6. Failure Tests - Node failure scenarios
7. Full Test Suite - All tests
8. Analyze Results - Generate reports

## Troubleshooting

### Issue: All requests fail
- **Check**: Are server nodes running?
- **Check**: Are server addresses correct?
- **Check**: Is the test image file present?

### Issue: Low throughput
- **Check**: Network latency between client and servers
- **Check**: Server CPU/memory usage (might be overloaded)
- **Try**: Reduce concurrent clients, increase requests per client

### Issue: High failure rate under load
- **Check**: UDP buffer sizes (might be dropping packets)
- **Check**: Server queue sizes
- **Try**: Reduce concurrent clients or add delays

### Issue: Uneven server distribution
- **Check**: Load balancing logic in your code
- **Check**: Are all nodes reporting correct load values?
- **Expected**: Some imbalance is normal, should be < 20% difference

## Best Practices

1. **Start small**: Test with 10 clients first, then scale up
2. **Test incrementally**: 10 → 100 → 1000 clients
3. **Monitor servers**: Watch CPU, memory, and network during tests
4. **Compare results**: Run same test multiple times to verify consistency
5. **Save results**: Use `--output` flag to save detailed results
6. **Analyze patterns**: Look at P95/P99 response times, not just averages

## Example Testing Session

```bash
# 1. Start your servers
./run_nodes.sh

# 2. Quick smoke test
python3 stress_test_concurrent.py \
  --clients 10 --requests-per-client 1 \
  --servers "10.40.59.43:8001,10.40.44.230:8002" \
  --username "1" --target-users "2,3" \
  --image "test_image.jpg"

# 3. Increase load gradually
python3 stress_test_concurrent.py \
  --clients 50 --requests-per-client 5 \
  --servers "..." --username "1" --target-users "2,3" --image "test_image.jpg"

python3 stress_test_concurrent.py \
  --clients 100 --requests-per-client 10 \
  --servers "..." --username "1" --target-users "2,3" --image "test_image.jpg"

python3 stress_test_concurrent.py \
  --clients 500 --requests-per-client 10 \
  --servers "..." --username "1" --target-users "2,3" --image "test_image.jpg"

# 4. Stress test
python3 stress_test_concurrent.py \
  --clients 1000 --requests-per-client 10 \
  --servers "..." --username "1" --target-users "2,3" --image "test_image.jpg"
```

## Summary

The concurrent stress testing system gives you fine-grained control over:
- **How many clients** are sending requests simultaneously
- **How many requests** each client sends
- **Total load** on your system

This allows you to test both **throughput** (total requests) and **concurrency** (simultaneous load) independently, giving you better insights into your system's behavior under different load patterns.
