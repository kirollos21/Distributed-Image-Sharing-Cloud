# Setup and Installation Guide

## Prerequisites

This project requires Rust to be installed on your system.

### Installing Rust

**On Linux/macOS:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**On Windows:**
Download and run [rustup-init.exe](https://rustup.rs/)

After installation, restart your terminal or run:
```bash
source $HOME/.cargo/env
```

Verify installation:
```bash
cargo --version
rustc --version
```

You should see output like:
```
cargo 1.75.0
rustc 1.75.0
```

## Quick Start

Once Rust is installed, follow these steps:

### 1. Build the Project

```bash
cd "/media/kirollos/Data/Distributed Systems/Cloud Project"
cargo build --release
```

This will download dependencies and compile the project (first build takes 2-5 minutes).

### 2. Run the Complete Demo

**Option A: Automated All-in-One Demo (Recommended)**

```bash
./run_demo.sh
# or
cargo run --release --bin demo
```

This single command will:
- Start 3 cloud nodes automatically
- Run 50 concurrent clients
- Execute 10,000 encryption requests
- Display real-time progress
- Show comprehensive metrics and analysis
- Continue running to demonstrate fault tolerance
- Press Ctrl+C when done

**Option B: Manual Multi-Terminal Setup**

Terminal 1:
```bash
cargo run --release --bin cloud-node 1
```

Terminal 2:
```bash
cargo run --release --bin cloud-node 2
```

Terminal 3:
```bash
cargo run --release --bin cloud-node 3
```

Terminal 4 (wait 5 seconds after starting nodes):
```bash
# Single client test
cargo run --release --bin client 1 10

# Or run stress test
cargo run --release --bin demo
```

**Option C: Background Node Execution**

```bash
# Start nodes in background
./run_nodes.sh

# Wait 5 seconds for initialization
sleep 5

# Run stress test
./run_demo.sh

# Stop all nodes when done
./stop_nodes.sh
```

## Verification

### Test Compilation

```bash
cargo check
```

### Run Unit Tests

```bash
cargo test
```

### Run with Debug Logging

```bash
RUST_LOG=debug cargo run --release --bin demo
```

## Troubleshooting

### Build Errors

**Error: "failed to fetch"**
```bash
# Update Rust
rustup update

# Clear cache and rebuild
cargo clean
cargo build --release
```

**Error: "linking with `cc` failed"**
```bash
# On Ubuntu/Debian
sudo apt-get install build-essential

# On Fedora/RHEL
sudo dnf install gcc

# On macOS (install Xcode Command Line Tools)
xcode-select --install
```

### Runtime Errors

**Error: "Address already in use"**
```bash
# Kill existing processes
./stop_nodes.sh

# Or manually
pkill cloud-node
killall cloud-node

# Wait a few seconds
sleep 3

# Try again
./run_demo.sh
```

**Error: "Connection refused"**
- Make sure cloud nodes are started before running clients
- Wait 5-10 seconds after starting nodes for initialization
- Check if nodes are running: `ps aux | grep cloud-node`

**High failure rate in stress test**
- This is expected! Nodes randomly fail to simulate fault tolerance
- Success rate of 95-99% is normal
- Failed requests are usually due to a node being in "FAILED" state

### Performance Issues

**Slow compilation**
- First build takes 2-5 minutes (downloads dependencies)
- Subsequent builds are much faster
- Use `--release` flag for optimized builds

**Slow runtime**
- Make sure you're using `--release` flag
- Debug builds are 10-100x slower
- Example: `cargo run --release --bin demo`

## Expected Output

### Successful Build

```
   Compiling distributed-image-cloud v0.1.0
    Finished release [optimized] target(s) in 45.23s
```

### Successful Demo Run

```
============================================================
         DISTRIBUTED IMAGE SHARING CLOUD - PHASE 1 DEMO
              Bully Election Algorithm Implementation
============================================================

Configuration:
  Cloud Nodes:           3
  Concurrent Clients:    50
  Requests per Client:   200
  Total Requests:        10000

Starting 3 cloud nodes...
  ✓ Node 1 started on 127.0.0.1:8001
  ✓ Node 2 started on 127.0.0.1:8002
  ✓ Node 3 started on 127.0.0.1:8003

All cloud nodes started successfully!

[Elections and request processing...]

============================================================
                    STRESS TEST RESULTS
============================================================

Total Duration:        45.23 seconds
Total Requests:        10000
Successful:            9847
Failed:                153
Success Rate:          98.47%
Throughput:            221.09 requests/second

[More metrics...]
```

## System Requirements

- **OS**: Linux, macOS, or Windows
- **RAM**: 2GB minimum, 4GB recommended
- **CPU**: Any modern processor (2+ cores recommended)
- **Disk**: 500MB for Rust toolchain + dependencies
- **Network**: Localhost networking (no external network needed)

## Architecture Validation

The implementation fulfills all Phase 1 requirements:

✅ **Technology Stack**
- Rust programming language
- Tokio async runtime for concurrency
- 3 independent cloud node processes
- Pool of concurrent client processes

✅ **Distributed Election & Load Balancing**
- Modified Bully algorithm (load-based)
- Lowest-load node elected as coordinator
- Client multicast to all nodes
- Load metrics tracking

✅ **Fault Tolerance**
- Random failure simulation (up to 20 seconds)
- Failure detection and re-election
- State recovery and consistency

✅ **Encryption Service**
- LSB steganography implementation
- Async processing with simulated delay
- Embeds usernames and viewing quota

✅ **Stress Testing**
- 50+ concurrent clients supported
- 10,000+ request load testing
- Comprehensive metrics collection

✅ **Logging & Output**
- Election results with load information
- State transitions logged
- Load balancing decisions tracked
- Final metrics summary

## Next Steps

After successful installation and demo:

1. **Experiment with parameters** in `src/bin/demo.rs`:
   - Adjust `num_clients` (concurrent clients)
   - Adjust `requests_per_client` (load per client)
   - Modify failure probability in `src/node.rs`

2. **Review logs** to understand the system:
   - Election processes
   - Failure and recovery cycles
   - Load distribution

3. **Run tests**:
   ```bash
   cargo test
   cargo test -- --nocapture  # See test output
   ```

4. **Read the code**:
   - `src/election.rs` - Bully algorithm
   - `src/node.rs` - Fault tolerance
   - `src/encryption.rs` - Steganography
   - `src/client.rs` - Stress testing

## Support

If you encounter issues:

1. Check this SETUP.md file
2. Review README.md for architecture details
3. Ensure Rust is properly installed: `cargo --version`
4. Try a clean build: `cargo clean && cargo build --release`
5. Check system requirements

## Additional Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Documentation](https://tokio.rs/)
- [Cargo Guide](https://doc.rust-lang.org/cargo/)
