# Distributed Image Sharing Cloud - Phase 1

A fault-tolerant, load-balanced cloud service implemented in Rust for CSCE 4411 Term Project.

## Overview

This project implements a distributed cloud system with the following features:

- **Load-Based Bully Election Algorithm**: Nodes elect a coordinator based on the lowest current load
- **Fault Tolerance**: Random node failures (up to 20 seconds) with automatic detection and recovery
- **Image Encryption Service**: LSB steganography to embed usernames and viewing quotas
- **Stress Testing**: Support for tens of concurrent clients and thousands of requests
- **Async/Await with Tokio**: High-performance concurrent request handling

## Architecture

### Components

1. **Cloud Nodes** (3 instances)
   - Handle encryption requests
   - Participate in elections
   - Simulate random failures
   - Recover state from coordinator

2. **Clients** (configurable, default 50)
   - Multicast encryption requests to all nodes
   - Receive encrypted images
   - Track success/failure rates

3. **Election Manager**
   - Implements modified Bully algorithm
   - Selects lowest-load node as coordinator
   - Handles failure detection and re-election

4. **Encryption Service**
   - LSB (Least Significant Bit) steganography
   - Embeds usernames and viewing quota
   - Async processing with simulated delay

### Node States

- **Active**: Normal operation, participating in elections
- **Failed**: Ignoring all communication (simulated failure)
- **Recovering**: Synchronizing state before rejoining

## Project Structure

```
.
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs              # Module exports
│   ├── messages.rs         # Protocol message types
│   ├── encryption.rs       # LSB steganography implementation
│   ├── election.rs         # Bully election algorithm
│   ├── node.rs             # Cloud node implementation
│   ├── client.rs           # Client implementation
│   ├── metrics.rs          # Stress testing metrics
│   └── bin/
│       ├── cloud_node.rs   # Cloud node binary
│       ├── client.rs       # Standalone client binary
│       └── demo.rs         # Full demo runner
├── run_nodes.sh            # Script to start cloud nodes
└── run_demo.sh             # Script to run complete demo
```

## Building

```bash
cargo build --release
```

## Running the Demo

### Option 1: Automated Demo (Recommended)

Run the complete automated demo with 50 clients and 10,000 requests:

```bash
cargo run --release --bin demo
```

This will:
1. Start 3 cloud nodes
2. Perform initial election
3. Run stress test with 50 concurrent clients × 200 requests = 10,000 total requests
4. Display metrics and analysis
5. Continue running to observe fault tolerance

### Option 2: Manual Setup

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

**Terminal 4 - Run clients:**
```bash
# Single client with 5 requests
cargo run --release --bin client 1 5

# Or use the stress test
cargo run --release --bin demo
```

### Using Shell Scripts

```bash
# Make scripts executable
chmod +x run_nodes.sh run_demo.sh

# Start nodes in background
./run_nodes.sh

# Wait a few seconds, then run demo
sleep 5
./run_demo.sh
```

## Configuration

### Stress Test Parameters (in `demo.rs`)

```rust
let num_clients = 50;              // Concurrent clients
let requests_per_client = 200;      // Requests per client
// Total: 50 × 200 = 10,000 requests
```

### Node Addresses

Default configuration uses localhost:
- Node 1: `127.0.0.1:8001`
- Node 2: `127.0.0.1:8002`
- Node 3: `127.0.0.1:8003`

Modify in `src/bin/cloud_node.rs` and `src/bin/demo.rs` for distributed deployment.

### Failure Simulation

Nodes randomly enter Failed state with 20% probability every 30 seconds (configurable in `node.rs`):

```rust
if rng.gen_bool(0.2) {  // 20% chance
    // Enter Failed state for 10-20 seconds
}
```

## Key Features Demonstrated

### 1. Load-Based Election

The modified Bully algorithm selects the node with the **lowest load** as coordinator:

```
=== ELECTION RESULT ===
Coordinator: Node 2 (load: 0.50)
All node loads:
  Node 2: 0.50 [COORDINATOR]
  Node 1: 0.75
  Node 3: 0.90
=======================
```

### 2. Fault Tolerance

Nodes randomly fail and recover:

```
[Node 2] *** Entering FAILED state ***
[Node 2] *** Entering RECOVERING state ***
[Node 2] Recovering state from peers...
[Node 2] *** Returning to ACTIVE state ***
```

### 3. Metrics Collection

Comprehensive stress test analysis:

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

Load Balancing Decisions: 9847
```

## Implementation Details

### Encryption (LSB Steganography)

Embeds metadata into image LSBs:
1. First 32 bits: metadata length
2. Following bits: JSON-encoded metadata (usernames + quota)

```rust
pub struct ImageMetadata {
    pub usernames: Vec<String>,
    pub quota: u32,
}
```

### Message Protocol

Messages exchanged between nodes:
- `Election`: Initiate election
- `Ok`: Response to election
- `Coordinator`: Announce coordinator
- `LoadQuery`/`LoadResponse`: Query node loads
- `EncryptionRequest`/`EncryptionResponse`: Client requests
- `StateSync`/`StateSyncResponse`: Recovery synchronization

### Concurrency Model

**Client Side:**
- Each client runs as a `tokio::task`
- 50 concurrent clients = 50 lightweight tasks

**Cloud Node Side:**
- Each encryption request handled in separate `tokio::task`
- Non-blocking I/O prevents head-of-line blocking
- Accurate load metrics based on queue length

## Testing

Run unit tests:

```bash
cargo test
```

Run with debug logging:

```bash
RUST_LOG=debug cargo run --bin demo
```

## Requirements Checklist - Phase 1

- [x] **Technology Stack**
  - [x] Rust implementation
  - [x] Tokio async runtime
  - [x] 3 independent cloud node processes
  - [x] Pool of concurrent client processes

- [x] **Distributed Election & Load Balancing**
  - [x] Load-based election (modified Bully)
  - [x] Lowest-load node elected as coordinator
  - [x] Client multicast to all nodes
  - [x] Load metrics (queue length, simulated I/O)

- [x] **Fault Tolerance & State Consistency**
  - [x] Random failure simulation (up to 20s)
  - [x] Failure detection via timeout
  - [x] Recovery with state synchronization
  - [x] States: Active → Failed → Recovering → Active

- [x] **Image Encryption Service**
  - [x] `encrypt_image` and `decrypt_image` functions
  - [x] LSB steganography
  - [x] Embed usernames and quota
  - [x] Async with simulated delay

- [x] **Stress Testing & Parallelization**
  - [x] 50 concurrent client tasks
  - [x] 10,000 total requests
  - [x] Each node handles requests concurrently
  - [x] Metrics collection and analysis

- [x] **Output & Logging**
  - [x] Election results with load
  - [x] State transitions (Failed/Recovering)
  - [x] Load balancing decisions
  - [x] Final stress test metrics

## Performance Notes

Typical results:
- Throughput: 200-300 requests/second
- Success rate: 95-99% (failures due to simulated node failures)
- Average latency: 500-600ms (includes simulated processing delay)
- P95 latency: 1000-1500ms

## Future Work (Phase 2)

- Discovery service for peer registration
- P2P image sharing between clients
- Quota management and view tracking
- Offline operation support
- Distributed consistency protocols

## Troubleshooting

**"Address already in use"**
- Kill existing node processes: `pkill cloud-node` or `killall cloud-node`
- Wait a few seconds for ports to be released

**High failure rate**
- Reduce failure probability in `node.rs`
- Increase timeouts in `client.rs`
- Reduce number of concurrent clients

**Slow performance**
- Build with `--release` flag
- Reduce simulated processing delay in `encryption.rs`
- Reduce number of requests or clients

## License

Academic project for CSCE 4411 - Fall 2025

## Authors

Distributed Systems Course Project
