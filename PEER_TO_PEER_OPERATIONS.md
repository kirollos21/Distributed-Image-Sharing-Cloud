# Peer-to-Peer Operations - Distributed Image Cloud

## Overview

This document details all peer-to-peer operations, algorithms, and protocols used in the distributed image cloud system for node coordination, failure detection, and load distribution.

---

## Table of Contents

1. [System Architecture](#system-architecture)
2. [Node Discovery & Initialization](#node-discovery--initialization)
3. [Leader Election Protocol](#leader-election-protocol)
4. [Heartbeat & Failure Detection](#heartbeat--failure-detection)
5. [Load Balancing](#load-balancing)
6. [Request Forwarding](#request-forwarding)
7. [Data Replication](#data-replication)
8. [Network Communication](#network-communication)
9. [Fault Tolerance](#fault-tolerance)
10. [State Synchronization](#state-synchronization)

---

## System Architecture

### Network Topology

```
┌─────────────────────────────────────────────────────────────┐
│                     Distributed Cloud Cluster               │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│     ┌───────────┐          ┌───────────┐                  │
│     │  Node 1   │◄────────►│  Node 2   │                  │
│     │ :8001     │          │ :8002     │                  │
│     │ ACTIVE    │          │ ACTIVE    │                  │
│     │ Load: 45% │          │ Load: 60% │                  │
│     └─────▲─────┘          └─────▲─────┘                  │
│           │                      │                         │
│           │                      │                         │
│           │    ┌───────────┐     │                         │
│           └────│  Node 3   │─────┘                         │
│                │ :8003     │                               │
│                │ ACTIVE    │                               │
│                │ Load: 30% │ ★ COORDINATOR                │
│                └───────────┘                               │
│                                                             │
│  ★ = Current Coordinator (elected via Bully Algorithm)    │
└─────────────────────────────────────────────────────────────┘
```

### Architecture Type
- **Topology**: Fully connected mesh
- **Communication**: Direct peer-to-peer UDP
- **Coordination**: Leader-based (Bully algorithm)
- **State**: Distributed (no centralized storage)

### Node Roles

1. **Coordinator Node**
   - Highest-ID active node (via Bully election)
   - Manages load balancing decisions
   - Forwards requests to least-loaded peers
   - Monitors cluster health

2. **Worker Nodes**
   - Process encryption/decryption requests
   - Report load to coordinator
   - Participate in elections
   - Maintain peer connections

3. **All Nodes** (regardless of role)
   - Send/receive heartbeats
   - Store encrypted images locally
   - Manage user sessions
   - Handle client requests

---

## Node Discovery & Initialization

### Bootstrap Process

```
┌──────────────────────────────────────────────────────────────┐
│ Node Startup Sequence                                        │
└──────────────────────────────────────────────────────────────┘

1. Parse command-line arguments
   ├─ Node ID (must be unique)
   ├─ UDP port (default: 8000 + node_id)
   └─ Peer addresses (optional, uses defaults)

2. Initialize UDP socket
   └─ Bind to 0.0.0.0:<port>

3. Create peer connections
   ├─ For each peer address:
   │  └─ Create PeerNode struct with address
   └─ Add to peers list

4. Initialize data structures
   ├─ Session manager (username → client_id mapping)
   ├─ Received images (image storage)
   ├─ Request cache (deduplication)
   └─ Metrics collector

5. Start background tasks
   ├─ Heartbeat sender (every 5s)
   ├─ Heartbeat monitor (check peers every 5s)
   └─ Message receiver (continuous)

6. Trigger initial election
   └─ Start Bully algorithm to elect coordinator

7. Enter ACTIVE state
   └─ Begin processing client requests
```

### Peer Configuration

**Static Configuration** (current implementation):
```rust
// Node 1 knows about Node 2 and Node 3
let peer_addresses = vec![
    "127.0.0.1:8002".to_string(),
    "127.0.0.1:8003".to_string(),
];
```

**Command-line Specification**:
```bash
# Node 1
cloud-node 1 8001

# Node 2 (with custom peers)
cloud-node 2 8002 127.0.0.1:8001 127.0.0.1:8003

# Node 3
cloud-node 3 8003
```

### Peer Data Structure

```rust
pub struct PeerNode {
    pub id: usize,                    // Peer node ID
    pub address: String,              // UDP address
    pub state: NodeState,             // ACTIVE/FAILED/RECOVERING
    pub last_heartbeat: Instant,      // Last seen timestamp
    pub load: u32,                    // Current load (0-100%)
}

pub enum NodeState {
    ACTIVE,      // Operational
    ELECTION,    // Participating in election
    FAILED,      // Detected as down
    RECOVERING,  // Rejoining cluster
}
```

---

## Leader Election Protocol

### Bully Algorithm Implementation

The system uses the **Bully Algorithm** for leader election, where the highest-ID active node becomes the coordinator.

#### Algorithm Overview

```
┌────────────────────────────────────────────────────────────┐
│ Bully Election Process                                     │
└────────────────────────────────────────────────────────────┘

TRIGGER: Coordinator heartbeat timeout (15 seconds)

STEP 1: Initiate Election
   Node N detects coordinator failure
   └─ Sends ELECTION message to all nodes with ID > N

STEP 2: Wait for Responses (2 seconds)
   ├─ If ANY higher node responds:
   │  └─ Give up, wait for COORDINATOR announcement
   └─ If NO responses:
      └─ Proceed to STEP 3 (I am the highest)

STEP 3: Declare Victory
   Winner sends COORDINATOR message to ALL nodes
   └─ All nodes update coordinator_id = winner's ID

STEP 4: Resume Normal Operation
   All nodes transition to ACTIVE state
```

#### Detailed Election Flow

```
Time │ Node 1 (ID=1)   │ Node 2 (ID=2)   │ Node 3 (ID=3)
─────┼─────────────────┼─────────────────┼──────────────────
  0s │ ACTIVE          │ ACTIVE          │ ACTIVE (Coord)
     │                 │                 │
 10s │ Heartbeat ──────┼────────────────►│ (Received)
     │                 │                 │
 15s │ Heartbeat ──────┼─────────X       │ ** CRASH **
     │                 │                 │
 30s │ "No HB for 15s" │ "No HB for 15s" │ (Down)
     │ START ELECTION  │ START ELECTION  │
     │                 │                 │
 31s │ ELECTION ───────┼────────────────►│ (Down)
     │      (to ID>1)  │                 │
     │                 │ ELECTION ───────┼►(to ID>2, none)
     │                 │                 │
 32s │ (Waiting...)    │ ELECTION_RESP ─►│ (From Node 2)
     │◄────────────────┼─                │
     │                 │                 │
 33s │ (Give up,       │ "I'm highest!"  │ (Down)
     │  Node 2 higher) │ COORDINATOR ────┼►ALL nodes
     │◄────────────────┼─                │
     │                 │                 │
 34s │ ACTIVE          │ ACTIVE (Coord)  │ (Down)
     │ Coordinator=2   │ Coordinator=2   │
```

#### Message Protocol

**1. ELECTION Message**
```rust
Message::Election {
    candidate_id: usize,  // ID of node starting election
}
```
Sent to all nodes with ID > candidate_id

**2. ELECTION_RESPONSE Message**
```rust
Message::ElectionResponse {
    node_id: usize,  // ID of responding node
}
```
Response from higher-ID node indicating it's alive

**3. COORDINATOR Message**
```rust
Message::Coordinator {
    coordinator_id: usize,  // ID of new coordinator
}
```
Broadcast to all nodes announcing winner

#### Election States

```rust
match node.state {
    NodeState::ACTIVE => {
        // Normal operation
        if coordinator_timeout() {
            start_election();  // Transition to ELECTION
        }
    }

    NodeState::ELECTION => {
        // Election in progress
        if received_election_from_higher_node() {
            respond();  // Send ELECTION_RESPONSE
            wait_for_coordinator();
        }

        if election_timeout() {  // 2 seconds
            declare_self_coordinator();  // I won!
            broadcast_coordinator();
            transition_to_active();
        }
    }

    NodeState::FAILED => {
        // Node is down, not participating
    }

    NodeState::RECOVERING => {
        // Node is rejoining
        transition_to_active();
        trigger_election();  // May need new coordinator
    }
}
```

#### Timeouts

| Event | Timeout | Action |
|-------|---------|--------|
| Coordinator heartbeat | 15 seconds | Trigger election |
| Election response | 2 seconds | Declare victory if no response |
| Coordinator announcement | N/A | Immediate state update |

---

## Heartbeat & Failure Detection

### Heartbeat Protocol

#### Heartbeat Transmission

Every node sends heartbeats to all peers every 5 seconds:

```rust
// Background task running continuously
async fn send_heartbeats(&self) {
    loop {
        let current_load = self.calculate_load().await;

        let heartbeat = Message::Heartbeat {
            node_id: self.id,
            load: current_load,
        };

        // Send to all peers
        for peer in &self.peers {
            self.send_message_to_peer(peer, heartbeat.clone()).await;
        }

        sleep(Duration::from_secs(5)).await;
    }
}
```

#### Heartbeat Message

```rust
Message::Heartbeat {
    node_id: usize,  // Sender's ID
    load: u32,       // Current load percentage (0-100)
}
```

**Payload**: ~50 bytes
**Frequency**: Every 5 seconds
**Destination**: All peer nodes

### Failure Detection

#### Detection Algorithm

```rust
async fn check_peer_health(&self) {
    let now = Instant::now();

    for peer in &self.peers {
        let elapsed = now.duration_since(peer.last_heartbeat);

        if elapsed > Duration::from_secs(15) {  // 3 missed heartbeats
            // Mark as failed
            peer.state = NodeState::FAILED;

            warn!("[Node {}] Peer {} failed (no heartbeat for {:?})",
                  self.id, peer.id, elapsed);

            // If failed node was coordinator, trigger election
            if peer.id == self.coordinator_id {
                warn!("[Node {}] Coordinator {} failed, starting election",
                      self.id, peer.id);
                self.start_election().await;
            }
        }
    }
}
```

#### Failure Timeline

```
0s   │ Node sends heartbeat
5s   │ Node sends heartbeat
10s  │ Node sends heartbeat
15s  │ ** NODE CRASHES **
20s  │ Peer: "Expected heartbeat, none received"
25s  │ Peer: "Expected heartbeat, none received"
30s  │ Peer: "Expected heartbeat, none received"
     │ Peer: "3 missed heartbeats = FAILED"
     │ Peer: Mark node as FAILED
     │ Peer: If coordinator, trigger election
```

#### Recovery Detection

When a failed node recovers:

```rust
// Node sends heartbeat after recovery
Message::Heartbeat { node_id: 2, load: 0 }

// Receiving peer updates state
if peer.state == NodeState::FAILED {
    info!("[Node {}] Peer {} recovered!", self.id, peer.id);
    peer.state = NodeState::RECOVERING;
    peer.last_heartbeat = Instant::now();

    // Optionally trigger election if needed
    if self.coordinator_id == peer.id {
        self.start_election().await;
    }
}
```

### Heartbeat Metrics

Heartbeats also carry load information for load balancing:

```rust
let current_load = (queue_length * 100) / MAX_QUEUE_SIZE;

Message::Heartbeat {
    node_id: self.id,
    load: current_load,  // Piggybacked load metric
}
```

---

## Load Balancing

### Load-Based Request Forwarding

The coordinator forwards encryption requests to the least-loaded peer node.

#### Load Calculation

```rust
fn calculate_load(&self) -> u32 {
    let queue_len = self.request_queue.len();
    let max_queue = 100;

    let load_percentage = (queue_len * 100) / max_queue;
    load_percentage.min(100) as u32
}
```

**Factors**:
- Queue length (primary metric)
- Capped at 100%

#### Load Query Protocol

When coordinator receives encryption request:

```
1. Coordinator receives EncryptionRequest from client

2. Check own load
   ├─ If load < 50%: Process locally
   └─ If load >= 50%: Query peers

3. Query all peers
   └─ Send LoadQuery { request_id } to each peer

4. Collect LoadResponse messages
   └─ Timeout: 3 seconds

5. Select least-loaded peer
   └─ Forward request to peer with min(load)

6. Peer processes request
   └─ Sends EncryptionResponse back to client
```

#### Load Query Messages

**Query**:
```rust
Message::LoadQuery {
    request_id: String,  // For tracking
}
```

**Response**:
```rust
Message::LoadResponse {
    node_id: usize,  // Responding node's ID
    load: u32,       // Current load (0-100%)
}
```

#### Forwarding Decision

```rust
async fn forward_to_least_loaded_peer(&self, request: EncryptionRequest) {
    let request_id = uuid::Uuid::new_v4().to_string();

    // Query all active peers
    let load_query = Message::LoadQuery { request_id };

    for peer in &self.peers {
        if peer.state == NodeState::ACTIVE {
            self.send_message_to_peer(peer, load_query.clone()).await;
        }
    }

    // Wait for responses (3 seconds timeout)
    sleep(Duration::from_secs(3)).await;

    // Find least-loaded peer
    let mut min_load = u32::MAX;
    let mut selected_peer = None;

    for peer in &self.peers {
        if peer.state == NodeState::ACTIVE && peer.load < min_load {
            min_load = peer.load;
            selected_peer = Some(peer);
        }
    }

    // Forward request
    if let Some(peer) = selected_peer {
        info!("[Node {}] Forwarding to Node {} (load: {}%)",
              self.id, peer.id, peer.load);

        let mut forwarded_request = request.clone();
        forwarded_request.forwarded = true;

        self.send_message_to_peer(peer, forwarded_request).await;
    } else {
        // All peers busy, process locally
        warn!("[Node {}] All peers busy, processing locally", self.id);
        self.process_encryption_request(request).await;
    }
}
```

### Load Distribution Example

```
┌─────────────────────────────────────────────────────────────┐
│ Request Distribution Scenario                               │
└─────────────────────────────────────────────────────────────┘

Client sends 10 encryption requests to Coordinator (Node 3)

Initial State:
  Node 1: Load = 20%
  Node 2: Load = 60%
  Node 3: Load = 40% (Coordinator)

Request 1:
  - Coordinator load = 40% (< 50%)
  - Process locally
  - Node 3: Load = 50%

Request 2:
  - Coordinator load = 50% (>= 50%)
  - Query peers: Node 1 = 20%, Node 2 = 60%
  - Forward to Node 1 (least loaded)
  - Node 1: Load = 30%

Request 3:
  - Coordinator load = 50%
  - Query peers: Node 1 = 30%, Node 2 = 60%
  - Forward to Node 1
  - Node 1: Load = 40%

Request 4:
  - Coordinator load = 50%
  - Query peers: Node 1 = 40%, Node 2 = 60%
  - Forward to Node 1
  - Node 1: Load = 50%

Request 5:
  - Coordinator load = 50%
  - Query peers: Node 1 = 50%, Node 2 = 60%
  - Forward to Node 1 (still lowest)
  - Node 1: Load = 60%

Request 6:
  - Coordinator load = 50%
  - Query peers: Node 1 = 60%, Node 2 = 60%
  - Process locally (all peers >= own load)
  - Node 3: Load = 60%

Final Distribution:
  Node 1: Processed 4 requests (40%)
  Node 2: Processed 0 requests (0%)
  Node 3: Processed 2 requests (20%)
  Total: 10 requests balanced efficiently
```

---

## Request Forwarding

### Forwarding Protocol

#### Forwarding Flow

```
┌──────────────────────────────────────────────────────────┐
│ Request Forwarding Sequence                              │
└──────────────────────────────────────────────────────────┘

1. Client → Node 3 (Coordinator)
   └─ EncryptionRequest { forwarded: false, client_address: None }

2. Node 3: Check load
   └─ Load >= 50%, need to forward

3. Node 3: Query peers
   ├─ LoadQuery → Node 1
   └─ LoadQuery → Node 2

4. Peers respond
   ├─ Node 1: LoadResponse { load: 30% }
   └─ Node 2: LoadResponse { load: 70% }

5. Node 3: Select Node 1 (lowest load)
   └─ EncryptionRequest { forwarded: true, client_address: "192.168.1.100:5555" }

6. Node 1: Process request
   └─ Call encryption::encrypt_image()

7. Node 1 → Client (directly)
   └─ EncryptionResponse { encrypted_image: [...] }
```

#### Forwarded Request Handling

```rust
async fn handle_encryption_request(&self, mut request: EncryptionRequest) {
    if request.forwarded {
        // Request forwarded from coordinator, process immediately
        info!("[Node {}] Processing forwarded request", self.id);
        self.process_encryption_request(request).await;
    } else {
        // Request from client
        if self.am_i_coordinator() {
            // I'm coordinator, decide: process or forward
            if self.calculate_load().await < 50 {
                self.process_encryption_request(request).await;
            } else {
                self.forward_to_least_loaded_peer(request).await;
            }
        } else {
            // I'm not coordinator, forward to coordinator
            warn!("[Node {}] Not coordinator, forwarding to Node {}",
                  self.id, self.coordinator_id);
            self.forward_to_coordinator(request).await;
        }
    }
}
```

### Forwarding Metadata

**Original Request**:
```rust
EncryptionRequest {
    request_id: "req_123",
    client_username: "alice",
    image_data: [binary data],
    usernames: ["bob", "charlie"],
    quota: 5,
    forwarded: false,
    client_address: None,  // Will be filled by first receiver
}
```

**Forwarded Request**:
```rust
EncryptionRequest {
    request_id: "req_123",
    client_username: "alice",
    image_data: [binary data],
    usernames: ["bob", "charlie"],
    quota: 5,
    forwarded: true,  // Marked as forwarded
    client_address: Some("192.168.1.100:5555"),  // Original client
}
```

---

## Data Replication

### Session Data

Currently, sessions are **not replicated**. Each node maintains its own session map:

```rust
// Node-local storage
sessions: Arc<RwLock<HashMap<String, String>>>
// Key: username, Value: client_id
```

**Registration Flow**:
1. Client sends `SessionRegister` to any node
2. Node checks local session map
3. If username available locally, registers
4. **No cross-node synchronization**

**Limitation**: A username could theoretically be registered on multiple nodes simultaneously.

**Future Enhancement**: Implement session replication via coordinator or gossip protocol.

### Image Storage

Encrypted images are stored **locally** on the node that receives the `SendImage` request:

```rust
// Node-local storage
received_images: Arc<RwLock<HashMap<String, Vec<ReceivedImage>>>>
// Key: username, Value: list of images for that user
```

**Storage Flow**:
1. Client sends `SendImage` request
2. Receiving node stores image locally
3. **No replication to other nodes**

**Query Flow**:
1. Client queries received images
2. Query is sent to multiple nodes (multicast)
3. First response is used

**Limitation**: If node fails, images stored on it are lost.

**Future Enhancement**:
- Replicate to N nodes (e.g., N=2 for redundancy)
- Use consistent hashing for image distribution

### Request Cache

Each node maintains a request cache to prevent duplicate processing:

```rust
// Node-local storage
processed_requests_cache: Arc<RwLock<HashMap<String, Instant>>>
// Key: request_id, Value: timestamp
```

**Cache Entry Lifetime**: 60 seconds

**Deduplication Logic**:
```rust
if let Some(timestamp) = cache.get(&request_id) {
    if timestamp.elapsed() < Duration::from_secs(60) {
        // Duplicate request, ignore
        return;
    }
}
```

---

## Network Communication

### UDP Protocol Details

#### Socket Configuration

```rust
let socket = UdpSocket::bind("0.0.0.0:8001").await?;
```

- **Protocol**: UDP (User Datagram Protocol)
- **Port Range**: 8001-8003 (default)
- **Max Packet Size**: 65,507 bytes (65,535 - 8 byte UDP header - 20 byte IP header)

#### Message Serialization

```rust
// Serialize to JSON
let message_bytes = serde_json::to_vec(&message)?;

// Check size
if message_bytes.len() > 65507 {
    return Err("Message too large for UDP");
}

// Send
socket.send_to(&message_bytes, peer_address).await?;
```

#### Message Reception

```rust
let mut buffer = vec![0u8; 65535];

// Receive with timeout
let (n, src) = tokio::time::timeout(
    Duration::from_secs(10),
    socket.recv_from(&mut buffer)
).await??;

// Deserialize
let message: Message = serde_json::from_slice(&buffer[..n])?;
```

### Chunking for Large Messages

Messages > 45KB are chunked:

```rust
const CHUNK_SIZE: usize = 45_000;  // 45KB per chunk

if message_bytes.len() > CHUNK_SIZE {
    let chunks = ChunkedMessage::fragment(message_bytes);

    for chunk in chunks {
        let chunk_bytes = serde_json::to_vec(&chunk)?;
        socket.send_to(&chunk_bytes, address).await?;

        // 2ms delay between chunks
        sleep(Duration::from_millis(2)).await;
    }
}
```

**Reassembly**:
```rust
let mut reassembler = ChunkReassembler::new();

loop {
    let (n, _) = socket.recv_from(&mut buffer).await?;

    if let Ok(chunk) = serde_json::from_slice::<ChunkedMessage>(&buffer[..n]) {
        if let Some(complete_data) = reassembler.process_chunk(chunk) {
            let message = serde_json::from_slice::<Message>(&complete_data)?;
            return Ok(message);
        }
    }
}
```

### Message Priorities

Different message types have different timeout values:

| Message Type | Timeout |
|-------------|---------|
| Heartbeat | 1s |
| Election | 2s |
| LoadQuery | 3s |
| EncryptionRequest | 30s |
| SessionRegister | 1s |

---

## Fault Tolerance

### Failure Scenarios

#### Scenario 1: Single Node Failure

```
Initial: Node 1 (ACTIVE), Node 2 (ACTIVE), Node 3 (ACTIVE, Coordinator)

Node 2 Crashes
└─ Nodes 1 & 3 detect via heartbeat timeout (15s)
   └─ Mark Node 2 as FAILED
      └─ Coordinator (Node 3) adjusts load balancing
         └─ Only forwards to Node 1 now
            └─ System continues with reduced capacity
```

**Impact**: Reduced capacity, no data loss (except images on Node 2)

#### Scenario 2: Coordinator Failure

```
Initial: Node 1 (ACTIVE), Node 2 (ACTIVE), Node 3 (ACTIVE, Coordinator)

Node 3 (Coordinator) Crashes
└─ Nodes 1 & 2 detect coordinator timeout (15s)
   └─ Both trigger election
      └─ Node 2 sends ELECTION to higher nodes (none)
         └─ Node 2 wins, announces COORDINATOR
            └─ Node 1 updates coordinator_id = 2
               └─ System resumes with Node 2 as coordinator
```

**Impact**: ~15s disruption, then full recovery

#### Scenario 3: Network Partition

```
Initial: Node 1 (ACTIVE), Node 2 (ACTIVE), Node 3 (ACTIVE, Coordinator)

Network splits: {Node 1, Node 2} | {Node 3}

Partition A: Nodes 1 & 2
└─ Detect Node 3 failure
   └─ Node 2 becomes coordinator
      └─ Continue processing requests

Partition B: Node 3
└─ Detects Nodes 1 & 2 failure
   └─ Continues as coordinator (self-elected)
      └─ Processes requests alone

Result: Split-brain scenario
```

**Impact**: Possible data inconsistency, duplicate coordinators

**Mitigation** (Future): Implement quorum-based consensus (e.g., Raft)

#### Scenario 4: Cascading Failures

```
Node 3 crashes → Load shifts to Nodes 1 & 2
└─ Node 1 overwhelmed, crashes
   └─ Node 2 alone, overwhelmed
      └─ System may become unresponsive
```

**Mitigation**:
- Implement request queuing with backpressure
- Reject requests when load > 95%

### Recovery Mechanisms

#### Crash Recovery

```rust
// Node restarts after crash
1. Reinitialize all data structures (empty)
2. Bind to same UDP port
3. Send heartbeats to peers
4. Peers detect recovery via heartbeat
5. Update state to RECOVERING → ACTIVE
6. Participate in next election if needed
```

**Lost Data**:
- Session registrations (need to re-register)
- Received images (permanently lost)
- Request cache (rebuilt from new requests)

#### State Synchronization

Currently, **no state synchronization** on recovery. Node starts with empty state.

**Future Enhancement**:
- Implement state transfer protocol
- Request current sessions from coordinator
- Request image list from peers

---

## State Synchronization

### Current State Distribution

```
┌─────────────────────────────────────────────────────────┐
│ Distributed State Overview                             │
└─────────────────────────────────────────────────────────┘

Per-Node State (not shared):
  • Session registrations
  • Received images
  • Request cache
  • Metrics (local)

Shared State (via messages):
  • Peer health (via heartbeats)
  • Peer load (via heartbeats)
  • Coordinator ID (via election)
  • Node states (ACTIVE/FAILED)
```

### Eventual Consistency

The system achieves eventual consistency through:

1. **Heartbeat Propagation**
   - Every 5s, nodes exchange health/load info
   - Peer state converges within 5-15s

2. **Election Convergence**
   - Election completes within 2-5s
   - All nodes agree on coordinator

3. **Failure Detection**
   - Failures detected within 15-30s
   - All nodes update peer state

### Consistency Guarantees

**Strong Consistency**: None (no consensus protocol)

**Eventual Consistency**:
- Peer health status (within 15s)
- Coordinator identity (within 5s)

**No Consistency**:
- Session data (node-local only)
- Image storage (node-local only)

---

## Performance Characteristics

### Latency Metrics

| Operation | Typical Latency | Max Latency |
|-----------|----------------|-------------|
| Heartbeat | 1-2ms | 5ms |
| Election | 50-100ms | 2s |
| Load Query | 5-10ms | 3s |
| Request Forward | 10-20ms | 100ms |
| Failure Detection | 15-30s | 30s |

### Throughput

| Metric | Value |
|--------|-------|
| Heartbeats/sec | 0.6 per peer (every 5s) |
| Max concurrent encryptions | ~100 per node |
| Network bandwidth (heartbeats) | ~0.1 KB/s per peer |
| Network bandwidth (requests) | Variable (depends on image size) |

---

## Configuration Parameters

```rust
// Timing Constants
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(15);
const ELECTION_TIMEOUT: Duration = Duration::from_secs(2);
const LOAD_QUERY_TIMEOUT: Duration = Duration::from_secs(3);

// Capacity Constants
const MAX_QUEUE_SIZE: usize = 100;
const LOAD_THRESHOLD_FOR_FORWARD: u32 = 50;  // 50%

// Network Constants
const MAX_UDP_PACKET_SIZE: usize = 65507;
const CHUNK_SIZE: usize = 45000;  // 45KB
const CHUNK_DELAY_MS: u64 = 2;

// Cache Constants
const REQUEST_CACHE_TTL: Duration = Duration::from_secs(60);
```

---

**Last Updated**: 2025-11-23
**Version**: 1.0 (Bully Algorithm + UDP P2P)
