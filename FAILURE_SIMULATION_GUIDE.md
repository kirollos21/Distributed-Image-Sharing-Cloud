# Failure Simulation for Data Collection

## Overview

The system now has **failure simulation enabled** to collect data on fault tolerance and leader election behavior. Nodes will randomly fail and recover to test the distributed system's resilience.

## Failure Simulation Parameters

- **Failure Probability:** 20% chance every 30 seconds
- **Failure Duration:** Random between 10-20 seconds
- **Recovery Process:** Automatic state synchronization with coordinator
- **States:** Active → Failed → Recovering → Active

## Expected Behaviors to Observe

### 1. Node Failures
- Node enters `FAILED` state (stops processing requests)
- Other nodes detect missing node during election
- System continues operating with remaining nodes

### 2. Leader Re-election
- When coordinator fails, remaining nodes elect new leader
- Election based on lowest load (not highest ID)
- New coordinator takes over immediately

### 3. Node Recovery
- Failed node enters `RECOVERING` state
- Synchronizes state from current coordinator
- Returns to `ACTIVE` state and rejoins elections

### 4. Load Balancing
- Clients automatically retry failed coordinators
- Requests distributed across active nodes
- Load metrics update in real-time

## Running Failure Simulation

### Option 1: Automated Data Collection (Recommended)

1. **Rebuild with failure simulation enabled:**
   ```bash
   cargo build --release
   ```

2. **Start data collection script:**
   ```bash
   ./collect_failure_data.sh
   ```

3. **In separate terminals, start nodes with logging:**
   
   **Terminal 1 (Node 1):**
   ```bash
   cargo run --release --bin cloud-node 1 0.0.0.0:8001 <peer2_ip>:8002,<peer3_ip>:8003 2>&1 | tee failure_data_*/node_1.log
   ```
   
   **Terminal 2 (Node 2):**
   ```bash
   cargo run --release --bin cloud-node 2 0.0.0.0:8002 <peer1_ip>:8001,<peer3_ip>:8003 2>&1 | tee failure_data_*/node_2.log
   ```
   
   **Terminal 3 (Node 3):**
   ```bash
   cargo run --release --bin cloud-node 3 0.0.0.0:8003 <peer1_ip>:8001,<peer2_ip>:8002 2>&1 | tee failure_data_*/node_3.log
   ```

4. **Let it run for 5-10 minutes** to collect meaningful data

5. **Stop collection:** Press Ctrl+C in the data collection terminal

6. **View analysis:** Check the generated `failure_data_*/SUMMARY.txt`

### Option 2: Manual Monitoring

Start nodes and manually observe the logs for these patterns:

```bash
# Look for failure events:
[Node X] *** Entering FAILED state ***

# Look for recovery:
[Node X] *** Entering RECOVERING state ***
[Node X] Recovering state from peers...
[Node X] *** Returning to ACTIVE state ***

# Look for election changes:
[Node X] Starting election with load: 0.25
[Node X] New COORDINATOR: Node Y (load: 0.15)

# Look for election results:
[Node X] All node loads: Node 1: 0.10, Node 2: 0.25 [COORDINATOR], Node 3: 0.30
```

## Data Collection Metrics

The system will automatically track:

1. **Per Node:**
   - Number of failures
   - Number of recoveries
   - Times elected as coordinator
   - Elections initiated

2. **System-Wide:**
   - Total failures across all nodes
   - Total elections triggered
   - Election timeline and coordinator changes
   - Load distribution over time

3. **Performance:**
   - Client request success rate during failures
   - Recovery time after failures
   - Leader election latency

## Testing Scenarios

### Scenario 1: Single Node Failure
**Objective:** Observe system handling one node failure

**What to watch:**
- Remaining 2 nodes continue operating
- New coordinator elected if failed node was leader
- Failed node recovers and rejoins

**Expected outcome:** Zero downtime, automatic recovery

### Scenario 2: Multiple Cascading Failures
**Objective:** Stress test with multiple failures

**What to watch:**
- Multiple nodes may fail simultaneously (random)
- Frequent re-elections
- System degrades gracefully

**Expected outcome:** System remains operational with at least 1 active node

### Scenario 3: Coordinator Failure During Request
**Objective:** Test request handling during coordinator failure

**What to do:**
1. Start client GUI
2. Upload an image for encryption
3. Observe if request completes despite failures

**Expected outcome:** Client retries and completes request

### Scenario 4: Network Partition Simulation
**Objective:** Observe split-brain prevention

**Manual test:**
1. Disconnect one machine's network temporarily
2. Observe remaining nodes elect new coordinator
3. Reconnect network
4. Observe rejoining behavior

## Client Testing with Failures

### Test 1: Image Encryption During Failures
```bash
# Start 3 clients
cargo run --release --bin client-gui 1
cargo run --release --bin client-gui 2
cargo run --release --bin client-gui 3
```

Actions:
1. Client 1 uploads an image while nodes are failing
2. Observe automatic retry logic
3. Verify encrypted image is stored successfully

### Test 2: Image Viewing During Recovery
1. Send an image to another user
2. Wait for a node failure
3. Try to view the image during recovery
4. Verify system finds the image on available nodes

## Expected Log Patterns

### Healthy Operation (No Failures)
```
[Node 1] All node loads: Node 1: 0.05 [COORDINATOR], Node 2: 0.10, Node 3: 0.08
[Node 2] All node loads: Node 1: 0.05 [COORDINATOR], Node 2: 0.10, Node 3: 0.08
[Node 3] All node loads: Node 1: 0.05 [COORDINATOR], Node 2: 0.10, Node 3: 0.08
```

### Node Failure Sequence
```
[Node 2] *** Entering FAILED state ***
[Node 1] All node loads: Node 1: 0.05 [COORDINATOR], Node 3: 0.08
[Node 3] All node loads: Node 1: 0.05 [COORDINATOR], Node 3: 0.08
... (10-20 seconds later)
[Node 2] *** Entering RECOVERING state ***
[Node 2] Recovering state from peers...
[Node 2] *** Returning to ACTIVE state ***
[Node 1] All node loads: Node 1: 0.05 [COORDINATOR], Node 2: 0.10, Node 3: 0.08
```

### Coordinator Failure & Re-election
```
[Node 1] *** Entering FAILED state ***  (Node 1 was coordinator)
[Node 2] Starting election with load: 0.15
[Node 3] Starting election with load: 0.20
[Node 2] New COORDINATOR: Node 2 (load: 0.15)  (Node 2 wins, lowest load)
[Node 3] New COORDINATOR: Node 2 (load: 0.15)
... (Node 1 recovers)
[Node 1] *** Entering RECOVERING state ***
[Node 1] Recovering state from peers...
[Node 1] *** Returning to ACTIVE state ***
[Node 1] New COORDINATOR: Node 2 (load: 0.15)  (Node 1 learns current coordinator)
```

## Data Analysis

After collection, analyze:

1. **Failure Recovery Time:** Time from FAILED → ACTIVE
2. **Election Frequency:** Elections per minute
3. **Coordinator Stability:** Average time as coordinator
4. **System Availability:** % of time with at least 2 active nodes
5. **Request Success Rate:** % of client requests that succeed

## Disabling Failure Simulation

To run without failures again (production mode):

1. Edit `src/node.rs` line ~63-70
2. Comment out the failure_simulation_task spawn:
   ```rust
   // DATA COLLECTION MODE: Failure simulation disabled
   // let self_clone = self.clone();
   // tokio::spawn(async move {
   //     self_clone.failure_simulation_task().await;
   // });
   ```
3. Rebuild: `cargo build --release`

## Troubleshooting

**Issue:** Nodes don't see each other even with failures enabled

**Solution:** Make sure nodes are on the same network and can communicate via UDP. Run connectivity test first.

**Issue:** Too many failures, system constantly restarting

**Solution:** The 20% chance every 30 seconds is already conservative. To reduce further, edit line ~444 in `src/node.rs`:
```rust
if rng.gen_bool(0.1) {  // Change to 10% instead of 20%
```

**Issue:** Not enough failures to collect data

**Solution:** Increase probability or decrease interval:
```rust
let mut interval = interval(Duration::from_secs(20)); // Check every 20s instead of 30s
if rng.gen_bool(0.3) { // 30% chance
```

## Example Results Format

After running for 10 minutes, expect output like:

```
================================================
  FAILURE SIMULATION ANALYSIS SUMMARY
================================================

--- node_1 ---
  Failures: 3
  Recoveries: 3
  Returns to Active: 3
  Elections Initiated: 8
  Times as Coordinator: 12

--- node_2 ---
  Failures: 4
  Recoveries: 4
  Returns to Active: 4
  Elections Initiated: 9
  Times as Coordinator: 15

--- node_3 ---
  Failures: 2
  Recoveries: 2
  Returns to Active: 2
  Elections Initiated: 7
  Times as Coordinator: 10

--- SYSTEM METRICS ---
  Total Node Failures: 9
  Total Elections: 24
  Average Recovery Time: 12.5 seconds
  System Uptime: 98.5%
```

---

**Ready to start?** Run `./collect_failure_data.sh` and start your nodes!
