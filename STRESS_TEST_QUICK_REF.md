# Stress Testing Quick Reference

## Setup (One Time)

```bash
# 1. Create test image
python3 create_test_image.py

# 2. Build with failure simulation DISABLED for load tests
# (Comment out failure_simulation_task in src/node.rs line ~63-70)
cargo build --release

# 3. Start 3 server nodes on different machines
```

## TEST 1: Load Testing (10k, 12.5k, 15k, 17.5k, 20k images)

### On Each of 3 Client Machines:

**Machine 1:**
```bash
python3 stress_test_client.py 1 \
  --servers "SERVER1:8001,SERVER2:8002,SERVER3:8003" \
  --username "user1" \
  --target-users "user2,user3" \
  --output "load_client1.json"
```

**Machine 2:**
```bash
python3 stress_test_client.py 2 \
  --servers "SERVER1:8001,SERVER2:8002,SERVER3:8003" \
  --username "user2" \
  --target-users "user1,user3" \
  --output "load_client2.json"
```

**Machine 3:**
```bash
python3 stress_test_client.py 3 \
  --servers "SERVER1:8001,SERVER2:8002,SERVER3:8003" \
  --username "user3" \
  --target-users "user1,user2" \
  --output "load_client3.json"
```

**What happens:** Each client sends 10k→12.5k→15k→17.5k→20k images with 10s gaps

---

## TEST 2: Failure Testing - Baseline (No Failures)

### On Each of 3 Client Machines:

```bash
# Client 1
python3 stress_test_with_failures.py 1 \
  --servers "SERVER1:8001,SERVER2:8002,SERVER3:8003" \
  --username "user1" \
  --target-users "user2,user3" \
  --num-images 15000 \
  --failure-mode none \
  --output "baseline_client1.json"

# Client 2
python3 stress_test_with_failures.py 2 \
  --servers "SERVER1:8001,SERVER2:8002,SERVER3:8003" \
  --username "user2" \
  --target-users "user1,user3" \
  --num-images 15000 \
  --failure-mode none \
  --output "baseline_client2.json"

# Client 3
python3 stress_test_with_failures.py 3 \
  --servers "SERVER1:8001,SERVER2:8002,SERVER3:8003" \
  --username "user3" \
  --target-users "user1,user2" \
  --num-images 15000 \
  --failure-mode none \
  --output "baseline_client3.json"
```

**What happens:** Each client sends 15k images, no failures

---

## TEST 3: One Node Failure

### Setup:
1. Start all 3 server nodes
2. Run clients with `--failure-mode one`
3. **At t=30s:** Stop 1 server node (Ctrl+C)
4. **At t=90s:** Restart that server node
5. Test completes

### On Each of 3 Client Machines:

```bash
# Client 1
python3 stress_test_with_failures.py 1 \
  --servers "SERVER1:8001,SERVER2:8002,SERVER3:8003" \
  --username "user1" \
  --target-users "user2,user3" \
  --num-images 15000 \
  --failure-mode one \
  --output "one_failure_client1.json"

# Client 2
python3 stress_test_with_failures.py 2 \
  --servers "SERVER1:8001,SERVER2:8002,SERVER3:8003" \
  --username "user2" \
  --target-users "user1,user3" \
  --num-images 15000 \
  --failure-mode one \
  --output "one_failure_client2.json"

# Client 3
python3 stress_test_with_failures.py 3 \
  --servers "SERVER1:8001,SERVER2:8002,SERVER3:8003" \
  --username "user3" \
  --target-users "user1,user2" \
  --num-images 15000 \
  --failure-mode one \
  --output "one_failure_client3.json"
```

**What happens:** Each client sends 15k images while 1 node fails for 60s

---

## TEST 4: Two Node Failure

### Setup:
1. Start all 3 server nodes
2. Run clients with `--failure-mode two`
3. **At t=30s:** Stop 2 server nodes (Ctrl+C both)
4. **At t=90s:** Restart both server nodes
5. Test completes

### On Each of 3 Client Machines:

```bash
# Client 1
python3 stress_test_with_failures.py 1 \
  --servers "SERVER1:8001,SERVER2:8002,SERVER3:8003" \
  --username "user1" \
  --target-users "user2,user3" \
  --num-images 15000 \
  --failure-mode two \
  --output "two_failure_client1.json"

# Client 2
python3 stress_test_with_failures.py 2 \
  --servers "SERVER1:8001,SERVER2:8002,SERVER3:8003" \
  --username "user2" \
  --target-users "user1,user3" \
  --num-images 15000 \
  --failure-mode two \
  --output "two_failure_client2.json"

# Client 3
python3 stress_test_with_failures.py 3 \
  --servers "SERVER1:8001,SERVER2:8002,SERVER3:8003" \
  --username "user3" \
  --target-users "user1,user2" \
  --num-images 15000 \
  --failure-mode two \
  --output "two_failure_client3.json"
```

**What happens:** Each client sends 15k images while 2 nodes fail for 60s (only 1 node surviving!)

---

## Analyze Results

```bash
# Analyze all results together
python3 analyze_stress_results.py *.json > final_report.txt

# View report
cat final_report.txt
```

---

## Quick Test (Single Client, Small Load)

```bash
python3 stress_test_client.py 1 \
  --servers "192.168.4.2:8001,192.168.4.3:8002,192.168.4.4:8003" \
  --username "testuser" \
  --target-users "user2" \
  --start 1000 \
  --end 5000 \
  --step 1000 \
  --gap 5
```

Tests: 1k, 2k, 3k, 4k, 5k images with 5s gaps

---

## Troubleshooting

**No test_image.jpg?**
```bash
python3 create_test_image.py
```

**Connection errors?**
```bash
# Test server connectivity
nc -u SERVER_IP 8001
# Type "test" and press Enter
```

**Server not responding?**
```bash
# Check if server is running
ps aux | grep cloud-node

# Restart server
cargo run --release --bin cloud-node 1 0.0.0.0:8001 PEER1:8002,PEER2:8003
```

---

## Expected Results

| Test Type | Images | Expected Success Rate | Notes |
|-----------|--------|----------------------|-------|
| Load 10k | 10,000 | 99-100% | Baseline performance |
| Load 12.5k | 12,500 | 99-100% | Still within capacity |
| Load 15k | 15,000 | 98-99% | Moderate load |
| Load 17.5k | 17,500 | 95-98% | High load |
| Load 20k | 20,000 | 90-95% | Near capacity |
| Baseline | 15,000 | 98-99% | No failures |
| One node fail | 15,000 | 95-98% | Resilient |
| Two nodes fail | 15,000 | 90-95% | Degraded but functional |

---

**Ready?** Start with TEST 1, then proceed to TEST 2, 3, and 4!
