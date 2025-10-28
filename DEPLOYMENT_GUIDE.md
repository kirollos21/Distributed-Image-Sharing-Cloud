# Deployment Guide: Multi-Machine Setup

This guide explains how to run the Distributed Image Cloud across multiple laptops/machines.

## Table of Contents
1. [Single Laptop Setup (All Local)](#single-laptop-setup)
2. [Multi-Laptop Setup (Distributed)](#multi-laptop-setup)
3. [Network Configuration](#network-configuration)
4. [Troubleshooting](#troubleshooting)

---

## Single Laptop Setup (All Local)

Run all 3 servers and 3 clients on the same machine for testing.

### Step 1: Build the Project

```bash
cd "/media/kirollos/Data/Distributed Systems/Cloud Project"


```

### Step 2: Start the Cloud Nodes

Open **3 separate terminals** on your laptop:

**Terminal 1 - Node 1:**
```bash
cargo run --release --bin cloud-node 1
```

**Terminal 2 - Node 2:**
```bash
cargo run --release --bin cloud-node 2
```

**Terminal 3 - Node 3:**
```bash
cargo run --release --bin cloud-node 3
```

You should see each node starting:
```
[Node 1] Starting on 127.0.0.1:8001
[Node 1] Listening on 127.0.0.1:8001
```

### Step 3: Start Client GUIs

Open **3 more terminals**:

**Terminal 4 - Client 1:**
```bash
cargo run --release --bin client-gui
```

**Terminal 5 - Client 2:**
```bash
cargo run --release --bin client-gui
```

**Terminal 6 - Client 3:**
```bash
cargo run --release --bin client-gui
```

Each will open a separate GUI window.

### Step 4: (Optional) Start Server Monitor

**Terminal 7 - Monitor Node 2:**
```bash
cargo run --release --bin server-gui 2
```

### Step 5: Test the System

In **any Client GUI window**:
1. Click "📂 Choose Image File"
2. Select an image (PNG, JPG, etc.)
3. Enter authorized users: `alice, bob, charlie`
4. Set quota: `5`
5. Click "🚀 Encrypt Image"
6. Watch it process in the Server Monitor
7. Click "Save Encrypted Image"

**You now have 3 clients and 3 servers running on one laptop!** ✅

---

## Multi-Laptop Setup (Distributed)

Run nodes on different machines across a network.

### Prerequisites

You need:
- **3 laptops** (for 3 cloud nodes)
- **All connected to the same network** (WiFi or LAN)
- **Rust installed on all 3 laptops**
- **Project code on all 3 laptops**

### Architecture

```
Laptop 1 (IP: 192.168.1.10)  →  Node 1 + Client GUI
Laptop 2 (IP: 192.168.1.11)  →  Node 2 + Server Monitor
Laptop 3 (IP: 192.168.1.12)  →  Node 3 + Client GUI
```

---

## Step-by-Step: Multi-Laptop Setup

### STEP 1: Find IP Addresses of All Laptops

On **each laptop**, run:

**Linux/macOS:**
```bash
hostname -I
# or
ip addr show | grep "inet "
```

**Windows:**
```cmd
ipconfig
```

Write down the IP addresses:
```
Laptop 1: 192.168.1.10
Laptop 2: 192.168.1.11
Laptop 3: 192.168.1.12
```

> **Note:** Use your actual IP addresses! These are examples.

---

### STEP 2: Configure Firewall on All Laptops

Allow incoming connections on ports **8001, 8002, 8003**.

**Ubuntu/Debian:**
```bash
sudo ufw allow 8001/udp
sudo ufw allow 8002/udp
sudo ufw allow 8003/udp
sudo ufw reload
```

**Fedora/RHEL:**
```bash
sudo firewall-cmd --add-port=8001/udp --permanent
sudo firewall-cmd --add-port=8002/udp --permanent
sudo firewall-cmd --add-port=8003/udp --permanent
sudo firewall-cmd --reload
```

**macOS:**
```bash
# macOS firewall usually allows local network by default
# If blocked, go to: System Preferences → Security & Privacy → Firewall → Options
```

**Windows:**
```cmd
# Windows Defender Firewall → Advanced Settings → Inbound Rules → New Rule
# Allow UDP ports 8001, 8002, 8003
```

---

### STEP 3: Copy Project to All Laptops

On **each laptop**, clone or copy the project:

```bash
# Option 1: Git (if you have it in a repo)
git clone <your-repo-url>

# Option 2: USB/SCP
# Copy the entire project folder to each laptop
```

Build on each laptop:
```bash
cd "/path/to/Cloud Project"
cargo build --release
```

---

### STEP 4: Understand the Command-Line Format

**No need to edit source code!** The cloud nodes now accept addresses as command-line arguments:

```bash
./cloud-node <node_id> <bind_address> <peer_addresses>
```

**Parameters:**
- `<node_id>`: 1, 2, or 3
- `<bind_address>`: Address to bind to (use `0.0.0.0:PORT` for multi-device)
- `<peer_addresses>`: Comma-separated list of other nodes

**Examples:**

**Single Laptop (localhost):**
```bash
./cloud-node 1 127.0.0.1:8001 127.0.0.1:8002,127.0.0.1:8003
```

**Multi-Device (use 0.0.0.0 to accept from all interfaces):**
```bash
# On Laptop 1 (IP: 192.168.1.10)
./cloud-node 1 0.0.0.0:8001 192.168.1.11:8002,192.168.1.12:8003

# On Laptop 2 (IP: 192.168.1.11)
./cloud-node 2 0.0.0.0:8002 192.168.1.10:8001,192.168.1.12:8003

# On Laptop 3 (IP: 192.168.1.12)
./cloud-node 3 0.0.0.0:8003 192.168.1.10:8001,192.168.1.11:8002
```

**⚠️ Important:**
- Use `0.0.0.0` as the bind address to accept connections from all network interfaces
- Use actual IPs of other machines in the peer list
- The protocol uses **UDP** (not TCP), so ensure firewall rules allow UDP traffic

#### Edit Client GUI Configuration (Still Required)

The client GUI still has hardcoded addresses that need updating.

On **all laptops with Client GUI**, edit:

```bash
nano src/gui_client.rs
```

**Find (around line 84):**

```rust
cloud_addresses: vec![
    "127.0.0.1:8001".to_string(),
    "127.0.0.1:8002".to_string(),
    "127.0.0.1:8003".to_string(),
],
```

**Replace with your actual IPs:**

```rust
cloud_addresses: vec![
    "192.168.1.10:8001".to_string(),  // Laptop 1
    "192.168.1.11:8002".to_string(),  // Laptop 2
    "192.168.1.12:8003".to_string(),  // Laptop 3
],
```

**Save and rebuild:**
```bash
cargo build --release --bin client-gui
```

---

### STEP 5: Start Cloud Nodes (One Per Laptop)

Now start the nodes on their respective laptops using the command-line format:

**On Laptop 1 (192.168.1.10):**
```bash
cargo run --release --bin cloud-node 1 0.0.0.0:8001 192.168.1.11:8002,192.168.1.12:8003
```

**On Laptop 2 (192.168.1.11):**
```bash
cargo run --release --bin cloud-node 2 0.0.0.0:8002 192.168.1.10:8001,192.168.1.12:8003
```

**On Laptop 3 (192.168.1.12):**
```bash
cargo run --release --bin cloud-node 3 0.0.0.0:8003 192.168.1.10:8001,192.168.1.11:8002
```

You should see:
```
[Node 1] Starting Cloud Node 1
[Node 1] Address: 0.0.0.0:8001
[Node 1] Peers: {2: "192.168.1.11:8002", 3: "192.168.1.12:8003"}
[Node 1] Listening on 0.0.0.0:8001 (UDP)
```

---

### STEP 6: Verify Network Connectivity

From **Laptop 1**, test UDP connectivity to other nodes:

```bash
# Test Node 2 (UDP)
nc -u -v 192.168.1.11 8002

# Test Node 3 (UDP)
nc -u -v 192.168.1.12 8003
```

**Note:** `telnet` doesn't work for UDP. Use `nc` (netcat) with `-u` flag for UDP testing.

If you get "Connection refused" or timeout:
- Is the node running on that laptop?
- Is the firewall configured correctly for UDP?
- Are all laptops on the same network?
- Try: `sudo ufw status` to verify ports are open

---

### STEP 7: Start Client GUIs

On **any laptop** (or all of them):

```bash
cargo run --release --bin client-gui
```

The GUI will connect to all 3 nodes across the network!

---

### STEP 8: Start Server Monitor

On **Laptop 2** (monitoring Node 2):

```bash
cargo run --release --bin server-gui 2
```

---

### STEP 9: Test the Distributed System

1. **On Laptop 1**: Use Client GUI to upload an image
2. **On Laptop 2**: Watch Server Monitor to see it processing
3. **On Laptop 3**: Use Client GUI to upload another image
4. **Observe**: Requests are load-balanced across all 3 nodes!

---

## Network Configuration Details

### Binding to Network Interface

By default, nodes bind to `127.0.0.1` (localhost only). To accept external connections:

**Option 1: Bind to Specific IP**
```rust
let my_address = "192.168.1.10:8001".to_string();
```

**Option 2: Bind to All Interfaces (0.0.0.0)**
```rust
let my_address = "0.0.0.0:8001".to_string();
```

> **Warning:** `0.0.0.0` allows connections from any network interface. Use with caution on public networks.

### Port Requirements

Each node needs **one port**:
- Node 1: Port 8001
- Node 2: Port 8002
- Node 3: Port 8003

Make sure these ports are:
✅ Not in use by other applications
✅ Allowed through firewall
✅ Not blocked by router/network admin

---

## Quick Reference Commands

### Check if Port is Open

**Linux:**
```bash
sudo netstat -tulpn | grep 8001
```

**Test Connection:**
```bash
nc -zv 192.168.1.11 8002
# or
telnet 192.168.1.11 8002
```

### Check Firewall Status

**Ubuntu:**
```bash
sudo ufw status
```

**Fedora:**
```bash
sudo firewall-cmd --list-all
```

---

## Troubleshooting

### Problem: "Connection refused"

**Cause:** Node not running or firewall blocking

**Solution:**
1. Verify node is running: `ps aux | grep cloud-node`
2. Check firewall: `sudo ufw status`
3. Test port: `nc -zv <IP> <PORT>`
4. Restart node and try again

---

### Problem: "Address already in use"

**Cause:** Port already taken

**Solution:**
```bash
# Find process using port
sudo lsof -i :8001

# Kill it
kill <PID>

# Or use different port
```

---

### Problem: Nodes can't find each other

**Cause:** Wrong IP addresses or network segmentation

**Solution:**
1. Ping each laptop: `ping 192.168.1.11`
2. Ensure all on same subnet (e.g., 192.168.1.x)
3. Check router settings (no AP isolation)
4. Verify IP addresses in code match actual IPs

---

### Problem: GUI shows "Failed to connect"

**Cause:** Client can't reach nodes

**Solution:**
1. Verify node addresses in `gui_client.rs`
2. Test each node: `telnet <IP> <PORT>`
3. Check client is using correct IPs
4. Restart Client GUI after code changes

---

### Problem: High latency between nodes

**Cause:** Network congestion or WiFi issues

**Solution:**
1. Use wired Ethernet if possible
2. Ensure all laptops on same WiFi band (2.4GHz or 5GHz)
3. Move closer to router
4. Check network speed: `ping -c 10 192.168.1.11`

---

## Example Scenarios

### Scenario 1: Lab Demo (3 Laptops)

**Setup:**
- Laptop A: Node 1 (192.168.1.10:8001)
- Laptop B: Node 2 (192.168.1.11:8002) + Server Monitor
- Laptop C: Node 3 (192.168.1.12:8003)
- Any laptop: Client GUIs (multiple instances)

**Steps:**
1. Configure all IPs in source code
2. Build on all laptops
3. Start nodes (one per laptop)
4. Start Server Monitor on Laptop B
5. Start Client GUIs on any laptop
6. Demo encryption requests
7. Show load balancing in Server Monitor
8. Trigger node failure to demo fault tolerance

---

### Scenario 2: Single Laptop Development

**Setup:**
- All nodes and clients on one machine
- Use `127.0.0.1` (localhost)

**Steps:**
1. No code changes needed
2. Start 3 nodes in 3 terminals
3. Start multiple Client GUIs
4. Start Server Monitor
5. Test everything locally

---

### Scenario 3: Mixed Setup (Some Local, Some Remote)

**Setup:**
- Laptop 1: Node 1 (local) + Node 2 (local)
- Laptop 2: Node 3 (remote)

**Configuration:**
```rust
let node_addresses = vec![
    (1, "127.0.0.1:8001".to_string()),        // Local
    (2, "127.0.0.1:8002".to_string()),        // Local
    (3, "192.168.1.12:8003".to_string()),     // Remote
];
```

---

## Performance Tips

### For Best Performance:

1. **Use Wired Connections** - Ethernet is faster and more stable than WiFi
2. **Same Subnet** - Keep all machines on same network segment
3. **Reduce Firewall Overhead** - Allow specific ports rather than all traffic
4. **Build with --release** - Always use release mode for better performance
5. **Close Unused Apps** - Free up CPU and network bandwidth

### Expected Performance:

| Setup | Expected Throughput | Latency |
|-------|-------------------|---------|
| Single Laptop (Local) | 500+ req/s | 2-5 ms |
| Multi-Laptop (WiFi) | 100-200 req/s | 20-50 ms |
| Multi-Laptop (Ethernet) | 300-400 req/s | 5-15 ms |

---

## Security Considerations

⚠️ **Important Security Notes:**

1. **No Encryption**: Network traffic is **not encrypted**. Don't use on untrusted networks.
2. **No Authentication**: Any client can connect. Secure your network.
3. **Firewall**: Only allow ports on trusted networks.
4. **Production Use**: This is for educational purposes. Not production-ready.

For production deployment, add:
- TLS/SSL encryption
- Authentication tokens
- Rate limiting
- Input validation

---

## Complete Example: 3 Laptops Setup

### Laptop 1 (Alice's Computer - 192.168.1.10)

```bash
# Edit client GUI configuration
nano src/gui_client.rs
# Change cloud_addresses to: 192.168.1.10:8001, 192.168.1.11:8002, 192.168.1.12:8003

# Build
cargo build --release --bin cloud-node

# Configure firewall
sudo ufw allow 8001/udp

# Run Node 1 (binds to 0.0.0.0 to accept from all interfaces)
cargo run --release --bin cloud-node 1 0.0.0.0:8001 192.168.1.11:8002,192.168.1.12:8003
```

### Laptop 2 (Bob's Computer - 192.168.1.11)

```bash
# Edit client GUI configuration (same as Laptop 1)
nano src/gui_client.rs

# Build
cargo build --release --bin cloud-node --bin server-gui

# Configure firewall
sudo ufw allow 8002/udp

# Run Node 2
cargo run --release --bin cloud-node 2 0.0.0.0:8002 192.168.1.10:8001,192.168.1.12:8003

# In another terminal: Run Monitor
cargo run --release --bin server-gui 2
```

### Laptop 3 (Charlie's Computer - 192.168.1.12)

```bash
# Edit client GUI configuration (same as others)
nano src/gui_client.rs

# Build
cargo build --release --bin cloud-node --bin client-gui

# Configure firewall
sudo ufw allow 8003/udp

# Run Node 3
cargo run --release --bin cloud-node 3 0.0.0.0:8003 192.168.1.10:8001,192.168.1.11:8002

# In another terminal: Run Client
cargo run --release --bin client-gui 1
```

### Expected Output

**On all nodes, you should see:**
```
[Node 1] Starting on 192.168.1.10:8001
[Node 1] Listening on 192.168.1.10:8001
[Node 1] Peers: {2: "192.168.1.11:8002", 3: "192.168.1.12:8003"}

=== ELECTION RESULT ===
Coordinator: Node 2 (load: 0.50)
All node loads:
  Node 2: 0.50 [COORDINATOR]
  Node 1: 0.65
  Node 3: 0.75
=======================
```

**Success! Your distributed cloud is running across 3 laptops!** 🎉

---

## Cleanup

To stop everything:

```bash
# On each laptop, press Ctrl+C in each terminal
# Or kill all processes:
pkill cloud-node
pkill client-gui
pkill server-gui
```

---

## Summary Checklist

Before running distributed setup:

- [ ] All laptops on same network
- [ ] IP addresses identified
- [ ] Firewall configured (ports 8001-8003)
- [ ] Code updated with actual IPs
- [ ] Project built on all laptops
- [ ] Network connectivity tested
- [ ] Nodes started (one per laptop)
- [ ] GUIs launched

**You're now ready to run a fully distributed cloud system!** 🚀

---

## Need Help?

- Check logs: Look at terminal output for errors
- Verify connectivity: Use `ping`, `telnet`, `nc`
- Review firewall: Ensure ports are open
- Test locally first: Get it working on one laptop before distributing

**Good luck with your distributed system!** 💪
