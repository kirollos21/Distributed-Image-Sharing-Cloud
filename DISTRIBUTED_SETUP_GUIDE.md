# Distributed 3x3 Setup Guide (Multi-Machine with GUI)

This guide explains how to run the Distributed Image Cloud across **3 different machines** with **3 client GUIs**.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Distributed System Layout                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Machine 1 (10.40.59.43)          Machine 2 (10.40.61.206)     │
│  ┌──────────────┐                 ┌──────────────┐             │
│  │  Cloud Node 1│◄───────────────►│  Cloud Node 2│             │
│  │  Port: 8001  │                 │  Port: 8002  │             │
│  └──────────────┘                 └──────────────┘             │
│         ▲                                  ▲                    │
│         │                                  │                    │
│         └──────────────┬───────────────────┘                    │
│                        │                                        │
│                        ▼                                        │
│             Machine 3 (10.40.44.230)                            │
│             ┌──────────────┐                                    │
│             │  Cloud Node 3│                                    │
│             │  Port: 8002  │                                    │
│             └──────────────┘                                    │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Client GUIs (Any Machine)                  │   │
│  │  ┌─────────┐    ┌─────────┐    ┌─────────┐             │   │
│  │  │Client 1 │    │Client 2 │    │Client 3 │             │   │
│  │  └─────────┘    └─────────┘    └─────────┘             │   │
│  │      Connect to any node to access the cluster          │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Prerequisites

### On Each Machine:
1. **Rust toolchain** installed (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
2. **Project copied** to each machine
3. **Network connectivity** between all machines (UDP ports 8001-8003 open)
4. **Same project version** on all machines

### Network Requirements:
- All machines must be on the same network or have firewall rules allowing UDP traffic
- Ports **8001, 8002, 8003** must be **open for UDP** on all nodes
- Test connectivity: `nc -u -v <IP> <PORT>` or `nmap -sU -p 8001-8003 <IP>`

## Step 1: Configure IP Addresses

You need to configure the IP addresses based on your network setup.

### 1.1 Find Your Machine IPs

On each machine, run:
```bash
# Linux
ip addr show | grep "inet "

# Or
hostname -I
```

Example output:
- Machine 1: `10.40.59.43`
- Machine 2: `10.40.61.206`
- Machine 3: `10.40.44.230`

### 1.2 Update Client GUI Configuration

**IMPORTANT:** The client GUI has hardcoded node addresses that need to be updated.

Edit `src/gui_client.rs` (around line 100):

```rust
cloud_addresses: vec![
    "10.40.59.43:8001".to_string(),    // Machine 1 IP:Port
    "10.40.61.206:8002".to_string(),   // Machine 2 IP:Port
    "10.40.44.230:8003".to_string(),   // Machine 3 IP:Port - NOTE: Port 8002 on this machine!
],
```

**Replace these IPs with your actual machine IPs!**

### 1.3 Rebuild the Project

After changing IPs, rebuild on the machine where you'll run the client GUI:

```bash
cargo build --release --bin client-gui
```

## Step 2: Build on All Machines

On **each machine**, compile the cloud node:

```bash
cd "/path/to/Distributed Systems/Cloud Project"
cargo build --release --bin cloud-node
```

This will take 5-10 minutes the first time.

## Step 3: Start Cloud Nodes

Start **one node per machine** using the commands below.

### Machine 1 (e.g., 10.40.59.43):
```bash
cd "/path/to/Distributed Systems/Cloud Project"
cargo run --release --bin cloud-node 1 0.0.0.0:8001 10.40.61.206:8002,10.40.44.230:8002
```

**Explanation:**
- `1` = Node ID
- `0.0.0.0:8001` = Bind to all interfaces on port 8001
- `10.40.61.206:8002,10.40.44.230:8002` = Peer node addresses (Machine 2 and Machine 3)

### Machine 2 (e.g., 10.40.61.206):
```bash
cd "/path/to/Distributed Systems/Cloud Project"
cargo run --release --bin cloud-node 2 0.0.0.0:8002 10.40.59.43:8001,10.40.44.230:8002
```

**Explanation:**
- `2` = Node ID
- `0.0.0.0:8002` = Bind to all interfaces on port 8002
- `10.40.59.43:8001,10.40.44.230:8002` = Peer node addresses (Machine 1 and Machine 3)

### Machine 3 (e.g., 10.40.44.230):
```bash
cd "/path/to/Distributed Systems/Cloud Project"
cargo run --release --bin cloud-node 3 0.0.0.0:8002 10.40.59.43:8001,10.40.61.206:8002
```

**Explanation:**
- `3` = Node ID
- `0.0.0.0:8002` = Bind to all interfaces on port 8002 (NOTE: Port 8002, not 8003!)
- `10.40.59.43:8001,10.40.61.206:8002` = Peer node addresses (Machine 1 and Machine 2)

**NOTE:** Based on your logs, Node 3 uses port **8002** (same as Node 2), not 8003!

## Step 4: Verify Cluster Formation

After starting all nodes, check the logs. You should see:

```
[Node 1] Starting on 0.0.0.0:8001
[Node 1] Listening on 0.0.0.0:8001 (UDP)
...
[Node 1] New COORDINATOR: Node X (load: 0.00)
...
[Node 1] ════════════════ CLUSTER LOAD DISTRIBUTION ════════════════
[Node 1] │ Node 1 │ Worker     │     0.00% │          0 │            0 │
[Node 1] │ Node 2 │ Worker     │     0.00% │          0 │            0 │
[Node 1] │ Node 3 │ COORDINATOR│     0.00% │          0 │            0 │
```

**Good signs:**
- ✅ All nodes show "Worker" or "COORDINATOR" status
- ✅ No "NO_HEARTBEAT" or "FAILED" status
- ✅ Heartbeats being received

**Bad signs:**
- ❌ "NO_HEARTBEAT" - Node can't reach peers (firewall/network issue)
- ❌ "FAILED" - Node marked as failed
- ❌ Node appears multiple times - Likely IP configuration error

## Step 5: Start Client GUIs

On **any machine** (doesn't have to be one of the node machines):

### Option A: Start Multiple Clients (Same Machine)

```bash
cd "/path/to/Distributed Systems/Cloud Project"

# Start Client 1
./target/release/client-gui 1 &

# Start Client 2
./target/release/client-gui 2 &

# Start Client 3
./target/release/client-gui 3 &
```

### Option B: Start One Client Per Machine

On Machine 1:
```bash
./target/release/client-gui alice &
```

On Machine 2:
```bash
./target/release/client-gui bob &
```

On Machine 3:
```bash
./target/release/client-gui charlie &
```

## Step 6: Use the Client GUI

### 6.1 Login
1. Enter a unique username (e.g., "alice", "bob", "charlie")
2. Click "Login"
3. Wait for "✅ Logged in as: <username>"

### 6.2 Encrypt and Upload Image
1. Click "Browse Image" and select an image
2. Set "Viewing Quota" (how many times the image can be viewed)
3. Click "Add Username" to add recipients
4. For each recipient:
   - Enter their username
   - Click "Check Availability" to verify it exists
   - Check the box to select them
5. Click "Encrypt Image"
6. After encryption completes, click "Send Encrypted Image"

### 6.3 View Received Images
1. Click "Received Images" tab
2. Click "Refresh"
3. Select an image from the list
4. Click "View Image"
5. Image will display with remaining views

## Step 7: Monitor the System (Optional)

### Option 1: Server Monitor GUI

On any machine:
```bash
./target/release/server-gui &
```

The monitor shows:
- All nodes in the cluster
- Current coordinator
- Load distribution
- Request counts

### Option 2: Watch Node Logs

Simply watch the terminal output from each node. You'll see:
- Load distribution table every 10 seconds
- Request handling logs
- Election logs
- Heartbeat failures (if any)

## Troubleshooting

### Issue 1: Client Can't Connect to Nodes

**Symptoms:**
- Login button does nothing or times out
- No response from cluster

**Solution:**
1. Verify node IPs in `src/gui_client.rs` lines 100-104
2. Rebuild client-gui: `cargo build --release --bin client-gui`
3. Test connectivity: `nc -u -v <NODE_IP> <PORT>`
4. Check firewall rules

### Issue 2: Nodes Show "NO_HEARTBEAT"

**Symptoms:**
```
[Node 1] │ Node 2 │ NO_HEARTBEAT │     0.00% │          0 │            0 │
```

**Solution:**
1. Verify peer addresses in node startup commands
2. Check firewall: `sudo ufw status` (Ubuntu) or `sudo firewall-cmd --list-all` (CentOS)
3. Open UDP ports:
   ```bash
   sudo ufw allow 8001:8003/udp
   ```
4. Test with netcat: `nc -u -v <PEER_IP> <PORT>`

### Issue 3: Nodes Show "FAILED"

**Symptoms:**
```
[Node 1] FAILURE DETECTED: Node 2 is not responding
```

**Solution:**
1. Check if the node process is actually running
2. Check network connectivity
3. Look for crashes in the node's terminal output
4. Restart the failed node

### Issue 4: All Requests Go to One Node

**Symptoms:**
```
[Node 1] │ Node 1 │ Worker     │     0.00% │          0 │            0 │
[Node 1] │ Node 2 │ Worker     │     0.00% │          0 │            0 │
[Node 1] │ Node 3 │ Worker     │   100.00% │        500 │           10 │
```

**Solution:**
- This is normal if other nodes aren't responding
- Check that all nodes show "Worker" or "COORDINATOR" status
- If they do, the load balancing is working - requests naturally go to the least loaded node

### Issue 5: Duplicate Nodes in Table

**Symptoms:**
```
[Node 1] │ Node 2 │ Worker     │     0.00% │          0 │            0 │
[Node 1] │ Node 2 │ NO_RESPONSE│     0.00% │          0 │            0 │
```

**Solution:**
- This indicates an IP configuration issue
- Verify each node has unique IP:Port combination
- Check peer addresses don't include the node's own address

## Network Configuration Examples

### Example 1: University Network
```
Machine 1: 10.40.59.43:8001
Machine 2: 10.40.61.206:8002
Machine 3: 10.40.44.230:8002

All on same subnet: 10.40.0.0/16
No firewall between machines (or ports opened)
```

### Example 2: Home Network
```
Machine 1: 192.168.1.10:8001
Machine 2: 192.168.1.20:8002
Machine 3: 192.168.1.30:8003

All on same subnet: 192.168.1.0/24
Router forwards UDP ports if needed
```

### Example 3: Cloud VMs (AWS/Azure/GCP)
```
Machine 1: 10.0.1.10:8001
Machine 2: 10.0.1.20:8002
Machine 3: 10.0.1.30:8003

Security groups must allow:
- Inbound UDP 8001-8003 from VPC
- Outbound UDP 8001-8003 to VPC
```

## Quick Start Commands

### Machine 1:
```bash
# Start Node 1
cargo run --release --bin cloud-node 1 0.0.0.0:8001 MACHINE2_IP:8002,MACHINE3_IP:8002
```

### Machine 2:
```bash
# Start Node 2
cargo run --release --bin cloud-node 2 0.0.0.0:8002 MACHINE1_IP:8001,MACHINE3_IP:8002
```

### Machine 3:
```bash
# Start Node 3
cargo run --release --bin cloud-node 3 0.0.0.0:8002 MACHINE1_IP:8001,MACHINE2_IP:8002
```

### Any Machine (for clients):
```bash
# After updating src/gui_client.rs with correct IPs and rebuilding
./target/release/client-gui alice &
./target/release/client-gui bob &
./target/release/client-gui charlie &
```

## Performance Notes

With the recent optimizations:
- **Heartbeat overhead:** 0.8 messages/sec (between all nodes)
- **Load balancing overhead:** 0 messages/sec (uses heartbeat cache)
- **Cache freshness:** 5-10 seconds (acceptable for load balancing)
- **Failure detection:** 20 seconds maximum

## Testing the System

1. **Test Load Balancing:**
   - Upload 10-20 images from different clients
   - Watch the load distribution table
   - Verify requests spread across nodes

2. **Test Fault Tolerance:**
   - Stop one node (Ctrl+C)
   - Watch other nodes detect failure (20 sec max)
   - Verify requests continue on remaining nodes
   - Restart the node and watch it rejoin

3. **Test Coordinator Election:**
   - Stop the coordinator node
   - Watch election happen
   - Verify new coordinator elected

4. **Test Image Sharing:**
   - Login as "alice" on Client 1
   - Upload and encrypt image for "bob"
   - Login as "bob" on Client 2
   - View the received image

## Stopping the System

### Stop Individual Nodes:
Press `Ctrl+C` in each node's terminal

### Stop All Clients:
```bash
pkill client-gui
```

### Stop Everything:
```bash
pkill cloud-node
pkill client-gui
pkill server-gui
```

## Configuration Summary

| Component | Location | What to Change |
|-----------|----------|----------------|
| Node addresses | Command line args | Peer IPs and ports |
| Client node list | `src/gui_client.rs:100-104` | All node IPs and ports |
| Heartbeat interval | `src/node.rs:1308` | `Duration::from_secs(5)` |
| Cache TTL | `src/node.rs:967` | `Duration::from_secs(10)` |
| Failure timeout | `src/node.rs:1353` | `Duration::from_secs(20)` |

## Additional Resources

- `LOAD_BALANCING_OPTIMIZATION.md` - Details on overhead reduction
- `NODE_COMMUNICATION_FIXES.md` - Information on retry logic and timeouts
- `LOAD_BALANCING_DIAGNOSTICS.md` - Troubleshooting load distribution issues
