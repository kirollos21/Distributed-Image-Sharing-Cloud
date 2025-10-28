# Quick Start Guide

## Option 1: Everything on ONE Laptop (Easiest)

### Automatic Setup
```bash
./run_local_demo.sh
```

This automatically opens:
- 3 cloud node terminals
- 3 client GUI windows
- 1 server monitor window

**Done!** Start uploading images from any client GUI.

---

### Manual Setup (One Laptop)

**Terminal 1-3: Start Nodes**
```bash
cargo run --release --bin cloud-node 1
cargo run --release --bin cloud-node 2
cargo run --release --bin cloud-node 3
```

**Terminal 4-6: Start Clients**
```bash
cargo run --release --bin client-gui  # Run 3 times
```

**Terminal 7: Monitor**
```bash
cargo run --release --bin server-gui
```

---

## Option 2: Distributed Across 3 Laptops

### Prerequisites
- All laptops on **same WiFi network**
- Project installed on all 3 laptops
- Know each laptop's IP address

### On Each Laptop

**Step 1: Run Setup Script**
```bash
./setup_distributed.sh
```

This will:
1. Detect your IP address
2. Ask which node (1, 2, or 3)
3. Ask for other nodes' IPs
4. Update configuration
5. Configure firewall
6. Build the project

**Step 2: Start Your Node**
```bash
# On Laptop 1
cargo run --release --bin cloud-node 1

# On Laptop 2
cargo run --release --bin cloud-node 2

# On Laptop 3
cargo run --release --bin cloud-node 3
```

**Step 3: Start GUIs (on any laptop)**
```bash
# Client
cargo run --release --bin client-gui

# Monitor
cargo run --release --bin server-gui
```

---

## Quick Commands

### Build Everything
```bash
cargo build --release
```

### Start a Node
```bash
cargo run --release --bin cloud-node <1|2|3>
```

### Start Client GUI
```bash
cargo run --release --bin client-gui
```

### Start Server Monitor
```bash
cargo run --release --bin server-gui [node-id]
```

### Stop Everything
```bash
pkill cloud-node
pkill client-gui
pkill server-gui
```

---

## Network Configuration

### Find Your IP Address

**Linux/macOS:**
```bash
hostname -I
```

**Windows:**
```cmd
ipconfig
```

### Test Connectivity
```bash
# From any laptop, test connection to Node 2
telnet 192.168.1.11 8002
```

### Open Firewall Ports

**Ubuntu:**
```bash
sudo ufw allow 8001/tcp
sudo ufw allow 8002/tcp
sudo ufw allow 8003/tcp
```

---

## Troubleshooting

### "Address already in use"
```bash
pkill cloud-node
# Wait 2 seconds, then restart
```

### "Connection refused"
1. Is the node running? → Start it
2. Is firewall blocking? → Open ports
3. Wrong IP address? → Check IPs

### GUI won't start
```bash
# Install dependencies (Ubuntu/Debian)
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

---

## Example Setups

### Lab Demo (3 Laptops)
```
Laptop A (192.168.1.10) → Node 1
Laptop B (192.168.1.11) → Node 2 + Monitor
Laptop C (192.168.1.12) → Node 3 + 2 Clients
```

### Development (1 Laptop)
```
Same Laptop → All 3 Nodes + Multiple Clients + Monitor
```

---

## Complete File Reference

| File | Purpose |
|------|---------|
| `run_local_demo.sh` | Auto-start everything locally |
| `setup_distributed.sh` | Configure for multi-laptop |
| `DEPLOYMENT_GUIDE.md` | Full deployment documentation |
| `GUI_README.md` | GUI user guide |
| `README.md` | Project overview |

---

## Need More Help?

📖 **Read the full guides:**
- `DEPLOYMENT_GUIDE.md` - Detailed multi-laptop setup
- `GUI_README.md` - GUI features and usage
- `README.md` - Project architecture

🔧 **Common Commands:**
```bash
# Check if nodes are running
ps aux | grep cloud-node

# View logs
cargo run --release --bin cloud-node 1

# Test network
ping <other-laptop-ip>
telnet <other-laptop-ip> 8001
```

---

**Good luck!** 🚀
