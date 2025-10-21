# Quick Start Guide

## One-Command Demo

```bash
# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Run the demo
cd "/media/kirollos/Data/Distributed Systems/Cloud Project"
cargo run --release --bin demo
```

That's it! The demo will automatically:
- ✓ Start 3 cloud nodes
- ✓ Run 50 concurrent clients
- ✓ Process 10,000 encryption requests
- ✓ Display real-time metrics
- ✓ Demonstrate fault tolerance

Expected runtime: **1-2 minutes**

---

## What You'll See

### 1. Initialization (10 seconds)
```
============================================================
         DISTRIBUTED IMAGE SHARING CLOUD - PHASE 1 DEMO
============================================================

Starting 3 cloud nodes...
  ✓ Node 1 started on 127.0.0.1:8001
  ✓ Node 2 started on 127.0.0.1:8002
  ✓ Node 3 started on 127.0.0.1:8003

Waiting for initial election...
```

### 2. Stress Test (40-50 seconds)
```
============================================================
                    STARTING STRESS TEST
============================================================

Progress: 2000/10000 (20.0%) | Success: 1980 | Failed: 20 | Throughput: 201.34 req/s
Progress: 4000/10000 (40.0%) | Success: 3960 | Failed: 40 | Throughput: 215.87 req/s
Progress: 6000/10000 (60.0%) | Success: 5940 | Failed: 60 | Throughput: 220.45 req/s
Progress: 8000/10000 (80.0%) | Success: 7920 | Failed: 80 | Throughput: 218.92 req/s
Progress: 10000/10000 (100.0%) | Success: 9900 | Failed: 100 | Throughput: 221.09 req/s
```

### 3. Final Results
```
============================================================
                    STRESS TEST RESULTS
============================================================

Total Duration:        45.23 seconds
Total Requests:        10000
Successful:            9847
Failed:                153
Success Rate:          98.47%
Throughput:            221.09 requests/second

Latency Statistics:
  Average:             524.32 ms
  P95:                 1250 ms

============================================================
```

### 4. Live Fault Tolerance Demo (30 seconds)
```
Demo will continue running for 30 more seconds...
Watch for nodes entering FAILED and RECOVERING states.

Node Status (T+5s):
  Node 1: ACTIVE | Load: 0.65 |
  Node 2: ACTIVE | Load: 0.50 | [COORDINATOR]
  Node 3: FAILED | Load: 0.00 |    ← Node failed!

Node Status (T+10s):
  Node 1: ACTIVE | Load: 0.70 | [COORDINATOR]  ← New coordinator!
  Node 2: ACTIVE | Load: 0.80 |
  Node 3: FAILED | Load: 0.00 |

Node Status (T+15s):
  Node 1: ACTIVE | Load: 0.70 | [COORDINATOR]
  Node 2: ACTIVE | Load: 0.80 |
  Node 3: RECOVERING | Load: 0.00 |    ← Recovering!

Node Status (T+20s):
  Node 1: ACTIVE | Load: 0.65 |
  Node 2: ACTIVE | Load: 0.50 | [COORDINATOR]  ← Re-elected!
  Node 3: ACTIVE | Load: 0.75 |    ← Recovered!
```

---

## Key Features Demonstrated

### ✓ Load-Based Election
```
=== ELECTION RESULT ===
Coordinator: Node 2 (load: 0.50)  ← Lowest load wins!
All node loads:
  Node 2: 0.50 [COORDINATOR]
  Node 1: 0.75
  Node 3: 0.90
=======================
```

### ✓ Fault Tolerance
- Nodes randomly fail for 10-20 seconds
- System detects failure and re-elects
- Node recovers and rejoins
- Service continues with minimal disruption

### ✓ High Concurrency
- 50 clients running simultaneously
- Each node handles multiple requests concurrently
- Non-blocking async/await with Tokio

### ✓ Encryption Service
- LSB steganography embeds metadata
- Usernames and viewing quota hidden in image
- Async processing with simulated delay

---

## Manual Testing

### Run Individual Nodes

**Terminal 1:**
```bash
cargo run --release --bin cloud-node 1
```

**Terminal 2:**
```bash
cargo run --release --bin cloud-node 2
```

**Terminal 3:**
```bash
cargo run --release --bin cloud-node 3
```

**Terminal 4 (wait 5 seconds):**
```bash
# Single client with 10 requests
cargo run --release --bin client 1 10
```

### Run Custom Stress Test

Edit `src/bin/demo.rs`:
```rust
let num_clients = 100;           // Increase clients
let requests_per_client = 100;   // Adjust requests
```

Then run:
```bash
cargo run --release --bin demo
```

---

## Troubleshooting

### "Address already in use"
```bash
pkill cloud-node
sleep 2
cargo run --release --bin demo
```

### "cargo: command not found"
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify
cargo --version
```

### Slow performance
Make sure you use `--release` flag:
```bash
cargo run --release --bin demo  # ✓ Fast (optimized)
cargo run --bin demo            # ✗ Slow (debug build)
```

### High failure rate
This is normal! Nodes randomly fail to demonstrate fault tolerance.
- Expected success rate: **95-99%**
- Failures occur when requests hit failed nodes
- System recovers automatically

---

## Understanding the Output

### Load Values
- **0.50-0.70**: Low load (node can accept more work)
- **0.70-0.90**: Medium load
- **0.90+**: High load (node is busy)

### Node States
- **ACTIVE**: Normal operation
- **FAILED**: Simulated crash (ignoring all messages)
- **RECOVERING**: Syncing state before rejoining

### Success Rate
- **98-99%**: Excellent (few failures during node failures)
- **95-97%**: Good (some failures during elections)
- **<95%**: Unusual (check logs for issues)

### Throughput
- **200+ req/s**: Good (limited by simulated 500ms encryption)
- **100-200 req/s**: Acceptable
- **<100 req/s**: Check if running in debug mode

---

## Next Steps

1. **Read the logs**: Understand election process
2. **Experiment**: Change parameters in `demo.rs`
3. **Explore code**: See implementation details
4. **Read docs**: Check DESIGN_DOCUMENT.md for full analysis

---

## Files Overview

| File | Purpose |
|------|---------|
| `run_demo.sh` | Run complete demo |
| `run_nodes.sh` | Start nodes in background |
| `stop_nodes.sh` | Stop all nodes |
| `README.md` | Comprehensive documentation |
| `SETUP.md` | Installation instructions |
| `DESIGN_DOCUMENT.md` | Full design analysis |
| `QUICKSTART.md` | This file |

---

## Contact

For questions or issues, refer to:
1. README.md - Full documentation
2. SETUP.md - Installation help
3. DESIGN_DOCUMENT.md - Technical details

---

**Enjoy the demo!** 🚀
