# Production Deployment Guide - Ubuntu 24.04 LTS

## Real Distributed Setup: 3 Servers + 3 Clients

This guide configures a **production-ready** distributed image sharing cloud with:
- **3 physical server machines** running cloud nodes
- **3 physical client machines** running GUI clients
- **No simulation** - real network, real failures handled gracefully

---

## 📋 Prerequisites

### Hardware Requirements
- **6 Ubuntu 24.04 LTS machines** (can be laptops, desktops, or VMs)
  - 3 designated as **servers**
  - 3 designated as **clients**
- **Same network** (WiFi or LAN)
- Minimum 2GB RAM per machine (4GB recommended)
- Stable network connection

### Software Requirements
All 6 machines need:
- Ubuntu 24.04 LTS (fresh install)
- Internet connection (for initial setup)
- SSH access (recommended for remote management)

---

## 🚀 Part 1: Initial Setup (All 6 Machines)

### Step 1: Install Rust on All Machines

Run on **each of the 6 machines**:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Select option 1 (default installation)
# Then reload environment
source $HOME/.cargo/env

# Verify installation
cargo --version
rustc --version
```

### Step 2: Install Build Dependencies

```bash
# On all 6 machines
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev git
```

### Step 3: Clone Project on All Machines

```bash
# On all 6 machines
cd ~
git clone https://github.com/kirollos21/Distributed-Image-Sharing-Cloud.git
cd Distributed-Image-Sharing-Cloud

# Build the project (this takes 2-5 minutes first time)
cargo build --release
```

---

## 🔧 Part 2: Network Configuration

### Step 1: Assign Machine Roles

Decide which machines are servers and clients:

```
SERVER 1: node1-hostname    (will run Node 1)
SERVER 2: node2-hostname    (will run Node 2)
SERVER 3: node3-hostname    (will run Node 3)

CLIENT 1: client1-hostname  (will run Client GUI)
CLIENT 2: client2-hostname  (will run Client GUI)
CLIENT 3: client3-hostname  (will run Client GUI)
```

### Step 2: Find IP Addresses

On **each of the 3 server machines**, run:

```bash
ip addr show | grep "inet " | grep -v "127.0.0.1"
# or
hostname -I | awk '{print $1}'
```

Example output:
```
SERVER 1: 192.168.1.101
SERVER 2: 192.168.1.102
SERVER 3: 192.168.1.103
```

**Write these down!** You'll need them for configuration.

### Step 3: Configure Firewall (Server Machines Only)

On **each of the 3 server machines**:

```bash
# Enable firewall
sudo ufw enable

# Allow SSH (important!)
sudo ufw allow ssh

# Allow node ports (UDP)
sudo ufw allow 8001/udp
sudo ufw allow 8002/udp
sudo ufw allow 8003/udp

# Verify rules
sudo ufw status

# Should show:
# 8001/udp    ALLOW       Anywhere
# 8002/udp    ALLOW       Anywhere
# 8003/udp    ALLOW       Anywhere
```

### Step 4: Test Connectivity

From **any client machine**, test if you can reach all servers:

```bash
# Test network connectivity
ping -c 3 192.168.1.101   # SERVER 1
ping -c 3 192.168.1.102   # SERVER 2
ping -c 3 192.168.1.103   # SERVER 3

# All should respond successfully
```

---

## 🖥️ Part 3: Configure & Start Server Nodes

### Configuration for Your Network

Let's say your server IPs are:
```
SERVER 1: 192.168.1.101
SERVER 2: 192.168.1.102
SERVER 3: 192.168.1.103
```

### Start Node 1 (on SERVER 1)

```bash
cd ~/Distributed-Image-Sharing-Cloud

cargo run --release --bin cloud-node 1 \
  0.0.0.0:8001 \
  192.168.1.102:8002,192.168.1.103:8003
```

**Explanation:**
- `1` = Node ID
- `0.0.0.0:8001` = Bind to all network interfaces on port 8001
- `192.168.1.102:8002,192.168.1.103:8003` = Peer addresses (other 2 nodes)

### Start Node 2 (on SERVER 2)

```bash
cd ~/Distributed-Image-Sharing-Cloud

cargo run --release --bin cloud-node 2 \
  0.0.0.0:8002 \
  192.168.1.101:8001,192.168.1.103:8003
```

### Start Node 3 (on SERVER 3)

```bash
cd ~/Distributed-Image-Sharing-Cloud

cargo run --release --bin cloud-node 3 \
  0.0.0.0:8003 \
  192.168.1.101:8001,192.168.1.102:8002
```

### Verify Servers Are Running

You should see output like:
```
[2025-10-29T10:30:45Z INFO  distributed_image_cloud::node] [Node 1] Starting on 0.0.0.0:8001
[2025-10-29T10:30:45Z INFO  distributed_image_cloud::node] [Node 1] Listening on 0.0.0.0:8001 (UDP)
[2025-10-29T10:30:50Z INFO  distributed_image_cloud::election] === ELECTION RESULT ===
[2025-10-29T10:30:50Z INFO  distributed_image_cloud::election] Coordinator: Node 1 (load: 0.50)
```

---

## 💻 Part 4: Configure & Start Client GUIs

### Edit Client Configuration (on Each Client Machine)

On **CLIENT 1, CLIENT 2, and CLIENT 3**, edit the source code:

```bash
cd ~/Distributed-Image-Sharing-Cloud
nano src/gui_client.rs
```

Find lines **101-103** (around line 101):
```rust
cloud_addresses: vec![
    "127.0.0.1:8001".to_string(),
    "127.0.0.1:8002".to_string(),
    "127.0.0.1:8003".to_string(),
],
```

**Replace with your actual server IPs:**
```rust
cloud_addresses: vec![
    "192.168.1.101:8001".to_string(),
    "192.168.1.102:8002".to_string(),
    "192.168.1.103:8003".to_string(),
],
```

Save and exit (`Ctrl+X`, `Y`, `Enter`)

### Rebuild on Client Machines

```bash
# On each client machine after editing
cargo build --release
```

### Start Client GUI (on Each Client Machine)

```bash
cd ~/Distributed-Image-Sharing-Cloud

# On CLIENT 1
cargo run --release --bin client-gui

# On CLIENT 2 (in a separate terminal on that machine)
cargo run --release --bin client-gui

# On CLIENT 3 (in a separate terminal on that machine)
cargo run --release --bin client-gui
```

Each client will open a GUI window!

---

## 🎯 Part 5: Testing the Production System

### Test 1: Register Users

On **Client 1 GUI**:
1. Enter username: `alice`
2. Click "Login"
3. Should see: "✓ Logged in as alice"

On **Client 2 GUI**:
1. Enter username: `bob`
2. Click "Login"

On **Client 3 GUI**:
1. Enter username: `charlie`
2. Click "Login"

### Test 2: Upload and Share Image

On **Client 1 (alice)**:
1. Click "📂 Choose Image File"
2. Select any image
3. Enter recipients: `bob, charlie`
4. Set quota: `3`
5. Click "🚀 Encrypt Image"
6. Wait for encryption (processed by one of the 3 servers)
7. Click "📤 Send to Recipients"

### Test 3: View Received Images

On **Client 2 (bob)**:
1. Click "Received Images" tab
2. Click "🔄 Refresh"
3. Should see image from alice
4. Click "👁️ View"
5. Image displays (quota decrements)

### Test 4: Fault Tolerance

**Kill one server** (e.g., press Ctrl+C on SERVER 2):
```
[Node 2] Shutting down...
```

**The system should still work!**
- Clients can still upload (nodes 1 and 3 handle requests)
- After 10-20 seconds, restart SERVER 2
- It will automatically rejoin and sync

---

## 🔄 Part 6: Production Management

### Option A: Run Servers as Systemd Services (Recommended)

Create service files for automatic startup and management.

**On SERVER 1**, create `/etc/systemd/system/cloud-node1.service`:

```bash
sudo nano /etc/systemd/system/cloud-node1.service
```

Add this content (replace `YOUR_USERNAME` with your actual username):
```ini
[Unit]
Description=Distributed Image Cloud - Node 1
After=network.target

[Service]
Type=simple
User=YOUR_USERNAME
WorkingDirectory=/home/YOUR_USERNAME/Distributed-Image-Sharing-Cloud
ExecStart=/home/YOUR_USERNAME/.cargo/bin/cargo run --release --bin cloud-node 1 0.0.0.0:8001 192.168.1.102:8002,192.168.1.103:8003
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

**Enable and start the service:**
```bash
sudo systemctl daemon-reload
sudo systemctl enable cloud-node1.service
sudo systemctl start cloud-node1.service

# Check status
sudo systemctl status cloud-node1.service

# View logs
sudo journalctl -u cloud-node1.service -f
```

**Repeat for SERVER 2 and SERVER 3** with appropriate node IDs and peer addresses.

### Option B: Run with Screen (Simple)

**On each server:**
```bash
# Install screen
sudo apt install screen

# Start node in screen session
screen -S node1
cargo run --release --bin cloud-node 1 0.0.0.0:8001 192.168.1.102:8002,192.168.1.103:8003

# Detach: Press Ctrl+A, then D
# Reattach: screen -r node1
```

---

## 📊 Part 7: Monitoring the System

### Check Node Status

From **any client machine**:
```bash
cd ~/Distributed-Image-Sharing-Cloud
cargo run --release --bin server-gui 1
# or
cargo run --release --bin server-gui 2
# or
cargo run --release --bin server-gui 3
```

This opens a monitoring GUI showing:
- Node states (Active/Failed/Recovering)
- Current loads
- Queue lengths
- Processed requests
- Election results

### View Server Logs

**On server machines:**
```bash
# If running with systemd
sudo journalctl -u cloud-node1.service -f

# If running in terminal
# Logs appear in the terminal where you started the node
```

---

## 🛠️ Troubleshooting Production Issues

### Issue: "Connection refused" on clients

**Cause:** Firewall blocking or wrong IPs

**Fix:**
```bash
# On servers, verify firewall
sudo ufw status

# Test UDP connectivity from client
nc -u -v 192.168.1.101 8001
# Type "hello" and press Enter
# Should connect (even if no response)
```

### Issue: "No route to host"

**Cause:** Machines on different networks

**Fix:**
```bash
# Check if machines can ping each other
ping 192.168.1.101

# Verify they're on same subnet
ip route show
```

### Issue: High failure rate

**Cause:** Network latency or packet loss

**Fix:**
```bash
# Check network quality
ping -c 100 192.168.1.101
# Should have <1% packet loss

# Check UDP buffer sizes
sysctl net.core.rmem_max
sysctl net.core.wmem_max

# Increase if needed
sudo sysctl -w net.core.rmem_max=26214400
sudo sysctl -w net.core.wmem_max=26214400
```

### Issue: Node doesn't see peers

**Cause:** Peer addresses misconfigured

**Fix:**
- Double-check IP addresses in startup command
- Ensure no typos in comma-separated peer list
- Restart nodes in order: 1, 2, 3 (with 5 second delays)

---

## 📈 Performance Tuning

### Optimize for Production Load

Edit `src/node.rs` line ~444 to disable/reduce simulated failures:

```bash
nano src/node.rs
```

Find this section (around line 444):
```rust
// Random chance to enter Failed state
if rng.gen_bool(0.2) {  // 20% chance every 30 seconds
```

**For production, change to:**
```rust
// Disable simulated failures in production
if rng.gen_bool(0.0) {  // 0% chance - no simulation
```

Or reduce frequency:
```rust
if rng.gen_bool(0.05) {  // 5% chance every 30 seconds
```

**Rebuild on all servers:**
```bash
cargo build --release
```

### Adjust Election Frequency

Edit `src/node.rs` line ~495:
```rust
let mut interval = interval(Duration::from_secs(15));  // Every 15 seconds
```

For production:
```rust
let mut interval = interval(Duration::from_secs(60));  // Every 60 seconds
```

---

## 🔒 Security Considerations

### Basic Security Setup

```bash
# On all server machines

# 1. Only allow connections from known client IPs (optional)
sudo ufw allow from 192.168.1.0/24 to any port 8001 proto udp
sudo ufw allow from 192.168.1.0/24 to any port 8002 proto udp
sudo ufw allow from 192.168.1.0/24 to any port 8003 proto udp

# 2. Disable default deny for others
sudo ufw default deny incoming
sudo ufw default allow outgoing

# 3. Enable rate limiting on SSH
sudo ufw limit ssh
```

---

## 📝 Quick Reference Commands

### Server Management
```bash
# Start node manually
cargo run --release --bin cloud-node <ID> 0.0.0.0:<PORT> <PEER_ADDRESSES>

# Start as systemd service
sudo systemctl start cloud-node<ID>.service

# Stop service
sudo systemctl stop cloud-node<ID>.service

# View logs
sudo journalctl -u cloud-node<ID>.service -f
```

### Client Management
```bash
# Start GUI client
cargo run --release --bin client-gui

# Start monitor
cargo run --release --bin server-gui <NODE_ID>
```

### Network Diagnostics
```bash
# Test connectivity
ping <SERVER_IP>

# Check open ports
sudo netstat -tulpn | grep 800

# Test UDP
nc -u -v <SERVER_IP> 8001

# Check firewall
sudo ufw status verbose
```

---

## 🎉 Summary

You now have a **production-ready** distributed image sharing cloud running on:
- ✅ 3 physical server machines
- ✅ 3 physical client machines  
- ✅ Real network communication
- ✅ Fault tolerance (node failures handled)
- ✅ Load balancing (automatic leader election)
- ✅ No simulation - actual distributed service!

**Next Steps:**
- Monitor system performance
- Tune election intervals
- Add more clients as needed
- Set up automated backups
- Configure SSL/TLS for encryption (future enhancement)

---

## 📞 Support Checklist

Before asking for help, verify:
1. ✅ All 6 machines have Rust installed
2. ✅ Project built successfully on all machines
3. ✅ Server IPs are correct in client code
4. ✅ Firewall rules configured on servers
5. ✅ Machines can ping each other
6. ✅ Nodes started with correct peer addresses
7. ✅ Logs show "Listening on 0.0.0.0:800X"

---

**Deployment Date:** October 29, 2025  
**Target OS:** Ubuntu 24.04 LTS  
**Architecture:** 3 Servers + 3 Clients (Production)
