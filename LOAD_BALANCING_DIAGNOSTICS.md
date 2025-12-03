# Load Balancing Diagnostics

## Why Your Load Is Imbalanced

Based on your output:
```
[Node 3] │ Node 2 │ Worker     │     0.00% │          0 │            0 │
[Node 3] │ Node 2 │ NO_RESPONSE │     0.00% │          0 │            0 │
[Node 3] │ Node 3 │ Worker     │     0.00% │        613 │            0 │
```

### **Root Causes:**

## 1. **Node 1 is Missing** ❌
- **Problem**: Node 1 (10.40.59.43:8001) doesn't appear in the table
- **Likely Reasons**:
  - Node 1 is not running
  - Node 1 failed and was detected by heartbeat system
  - Network partition preventing communication

**How to Check:**
```bash
# On machine 10.40.59.43:
ps aux | grep cloud-node

# Or check if port is listening:
netstat -tuln | grep 8001
```

## 2. **Node 2 is Not Responding** ⚠️
- **Problem**: Node 2 shows "NO_RESPONSE" status
- **Meaning**: Node 2 is not answering LoadQuery messages
- **Impact**: Coordinator cannot send work to Node 2 because it doesn't know its load

**Why Node 2 Might Not Respond:**

### a) Node 2 is Overloaded/Crashed
```bash
# On machine 10.40.44.230:
# Check if process is running
ps aux | grep cloud-node

# Check if it's responsive
tail -f /path/to/node2.log
```

### b) Network Issues
```bash
# From Node 3's machine:
ping 10.40.44.230

# Check if UDP port is reachable (use netcat):
nc -u -v 10.40.44.230 8002
```

### c) Socket Buffer Full
Node 2's UDP receive buffer might be full, dropping incoming LoadQuery packets.

```bash
# On Node 2's machine:
# Check for UDP errors
netstat -su | grep -i "receive errors"
```

### d) Node 2's Process is Stuck
The process might be deadlocked or in an infinite loop, not processing incoming messages.

## 3. **All Work Going to Node 3** 🔴
- **613 requests processed by Node 3**
- **0 requests by Node 1 or Node 2**

**Why This Happens:**

### If Node 3 is the Coordinator:
1. Coordinator queries all nodes for load
2. Node 1: No response (failed/missing)
3. Node 2: No response (timeout)
4. Node 3: Responds with its own load
5. **Result**: Only Node 3 has a known load, so all work goes there!

### The Load Balancing Algorithm:
```rust
// In find_lowest_load_node():
// 1. Query all nodes for their load
// 2. Skip nodes that don't respond
// 3. Select node with lowest load
// 4. If only Node 3 responds -> Node 3 gets ALL work
```

---

## How to Fix This:

### **Step 1: Verify All Nodes Are Running**
```bash
# On each machine:
# Machine 1 (10.40.59.43):
cargo run --release --bin cloud-node 1 0.0.0.0:8001 10.40.44.230:8002,10.40.61.206:8003

# Machine 2 (10.40.44.230):
cargo run --release --bin cloud-node 2 0.0.0.0:8002 10.40.59.43:8001,10.40.61.206:8003

# Machine 3 (10.40.61.206):
cargo run --release --bin cloud-node 3 0.0.0.0:8003 10.40.44.230:8002,10.40.59.43:8001
```

### **Step 2: Check Firewall Rules**
```bash
# On each machine, allow UDP traffic:
sudo ufw allow 8001/udp
sudo ufw allow 8002/udp
sudo ufw allow 8003/udp
```

### **Step 3: Verify Network Connectivity**
```bash
# From Node 3, test connectivity to others:
ping 10.40.59.43
ping 10.40.44.230

# Check UDP connectivity (requires netcat):
# On Node 1:
nc -u -l 8001

# On Node 3:
echo "test" | nc -u 10.40.59.43 8001
```

### **Step 4: Check Logs for Errors**
Look for these patterns in the logs:

**Good (Node is Responding):**
```
[Node 3] Received response from Node 2: LOAD_RESPONSE from Node 2 (load: 0.23, queue: 2, processed: 45)
```

**Bad (Timeout):**
```
[Node 3] Timeout waiting for response from Node 2 after 3s
[Node 3] No response from Node 2 after 3 attempts
```

### **Step 5: Check for Heartbeat Failures**
```
[Node 3] FAILURE DETECTED: Node 1 is not responding
```
If you see this, Node 1 is not sending heartbeats.

### **Step 6: Synchronize Clocks**
```bash
# On all machines:
sudo timedatectl set-ntp true
timedatectl status
```

---

## Expected Behavior After Fixes:

With all 3 nodes running and responsive:

```
[Node 3] ════════════════ CLUSTER LOAD DISTRIBUTION ════════════════
[Node 3] Coordinator: Node 2
[Node 3] ┌────────┬────────────┬───────────┬────────────┬──────────────┐
[Node 3] │ Node   │ Status     │ Load      │ Processed  │ Active Reqs  │
[Node 3] ├────────┼────────────┼───────────┼────────────┼──────────────┤
[Node 3] │ Node 1 │ Worker     │     0.00% │        204 │            0 │
[Node 3] │ Node 2 │ COORDINATOR│     0.00% │        212 │            0 │
[Node 3] │ Node 3 │ Worker     │     0.00% │        197 │            0 │
[Node 3] └────────┴────────────┴───────────┴────────────┴──────────────┘

[Node 3] IMBALANCED LOAD DETECTED: One node has 212/613 requests (34%)
```

**Note:** Some imbalance is normal (30-40%), but 100% on one node indicates a problem.

---

## New Warning Messages:

After recompiling, you'll see these warnings when load is imbalanced:

```
[Node 3] IMBALANCED LOAD DETECTED: One node has 613/613 requests (100%)
[Node 3] Node 2 not responding to load queries - cannot route work to it
[Node 3] Node 1 not responding to load queries - cannot route work to it
```

This will help you immediately identify which nodes are problematic.

---

## Common Causes Summary:

| Symptom | Likely Cause | Solution |
|---------|--------------|----------|
| Node missing from table | Failed/not running | Start the node |
| "NO_RESPONSE" status | Network issues or node stuck | Check connectivity, restart node |
| "FAILED" status | Heartbeat timeout | Node crashed, restart it |
| Duplicate entries | Logging bug (now fixed) | Recompile with latest code |
| 100% on one node | Other nodes not responsive | Fix connectivity, verify all nodes running |

---

## Test Connectivity:

Run this on each node to test UDP communication:

```python
# test_udp.py
import socket
import json

# Create UDP socket
sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.settimeout(3)

# Send LoadQuery to Node 2
message = {"LoadQuery": {"from_node": 3}}
sock.sendto(json.dumps(message).encode(), ("10.40.44.230", 8002))

# Wait for response
try:
    data, addr = sock.recvfrom(65535)
    print(f"✓ Received response from {addr}: {data[:100]}")
except socket.timeout:
    print(f"✗ No response (timeout)")
```

---

## Fixes Applied in This Update:

1. ✅ **Fixed duplicate entries** - Added deduplication check
2. ✅ **Added coordinator display** - Shows who is coordinating
3. ✅ **Added imbalance warnings** - Warns when >60% on one node
4. ✅ **Identifies non-responsive nodes** - Shows which nodes aren't responding

Recompile and run to see the improvements:
```bash
cargo build --release
```
