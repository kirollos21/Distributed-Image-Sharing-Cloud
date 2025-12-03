# Service Directory - Distributed Image Cloud

## Overview

This document provides a comprehensive directory of all services, components, and their interfaces in the Distributed Image Cloud system.

---

## Table of Contents

1. [Core Services](#core-services)
2. [Client Services](#client-services)
3. [Node Services](#node-services)
4. [Support Services](#support-services)
5. [Message Protocol](#message-protocol)
6. [API Reference](#api-reference)

---

## Core Services

### 1. Cloud Node Service
**Binary**: `cloud-node`
**Location**: `src/bin/cloud_node.rs`, `src/node.rs`
**Ports**: 8001-8003 (configurable)
**Protocol**: UDP

**Purpose**: Core distributed node that handles image encryption/decryption, load balancing, and coordination.

**Capabilities**:
- Image encryption using LSB steganography
- Image decryption and view quota management
- Bully algorithm leader election
- Load-based request forwarding
- Heartbeat monitoring
- Crash recovery
- Request caching

**Configuration**:
```bash
Usage: cloud-node <node_id> [port]
Example: cloud-node 1 8001
```

**State Machine**:
- `ACTIVE` - Normal operation, processing requests
- `ELECTION` - Participating in leader election
- `FAILED` - Detected failure, not processing
- `RECOVERING` - Rejoining cluster after failure

**Metrics Exposed**:
- Total requests processed
- Current queue length
- Load percentage (0-100)
- Success/failure counts
- Average latency
- P95 latency

---

### 2. Encryption Service
**Module**: `src/encryption.rs`
**Type**: Library module

**Purpose**: Provides image steganography encryption/decryption using LSB embedding.

**Key Functions**:

#### `encrypt_image()`
```rust
pub async fn encrypt_image(
    image_data: Vec<u8>,
    usernames: Vec<String>,
    quota: u32,
) -> Result<Vec<u8>, String>
```
- **Input**: Original image bytes, authorized usernames, viewing quota
- **Output**: Encrypted PNG (looks like `encrypction_key.jpg`)
- **Performance**: ~50-60ms, 1.5 MB/s
- **Size**: ~1.1MB output (depends on cover image)

**Encryption Process**:
1. Load original image to get dimensions
2. Load `encrypction_key.jpg` as cover image
3. Serialize metadata (usernames + quota) to JSON
4. Embed in cover image LSBs:
   - 4 bytes: metadata length
   - N bytes: metadata JSON
   - 4 bytes: original image length
   - M bytes: original image data
5. Return PNG with embedded data

#### `decrypt_image()`
```rust
pub async fn decrypt_image(
    encrypted_data: Vec<u8>,
) -> Result<(Vec<u8>, ImageMetadata), String>
```
- **Input**: Encrypted PNG bytes
- **Output**: Original image bytes + metadata
- **Performance**: ~25-30ms, 3.2 MB/s

**Decryption Process**:
1. Load encrypted PNG
2. Extract LSBs to reconstruct:
   - Metadata length (4 bytes)
   - Metadata JSON
   - Image length (4 bytes)
   - Original image data
3. Parse metadata and return both

**Data Structure**:
```rust
pub struct ImageMetadata {
    pub usernames: Vec<String>,  // Authorized users
    pub quota: u32,               // View limit
}
```

---

### 3. Session Management Service
**Module**: `src/node.rs` (SessionManager)
**Type**: In-memory service per node

**Purpose**: Track active client sessions and enforce username uniqueness.

**Data Structure**:
```rust
struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, String>>>
    // Key: username, Value: client_id
}
```

**Operations**:

#### Register Session
```rust
Message::SessionRegister {
    client_id: String,
    username: String,
}
```
- Validates username uniqueness across cluster
- Registers session if available
- Returns `SessionRegisterResponse` with success/error

#### Unregister Session
```rust
Message::SessionUnregister {
    client_id: String,
    username: String,
}
```
- Removes session from active sessions
- Fire-and-forget (no response expected)

#### Check Username Availability
```rust
Message::CheckUsernameAvailable {
    username: String,
}
```
- Queries if username is currently in use
- Returns `CheckUsernameResponse { available: bool }`

---

### 4. Image Storage Service
**Module**: `src/node.rs` (Image storage in Node)
**Type**: In-memory storage per node

**Purpose**: Store encrypted images with metadata and enforce viewing quotas.

**Data Structure**:
```rust
struct ReceivedImage {
    image_id: String,           // Unique identifier
    from_username: String,      // Sender
    to_username: String,        // Recipient
    encrypted_image: Vec<u8>,   // Encrypted PNG
    max_views: u32,             // Total views allowed
    remaining_views: u32,       // Views left
    timestamp: u64,             // Creation time
}
```

**Operations**:

#### Send Image
```rust
Message::SendImage {
    from_username: String,
    to_usernames: Vec<String>,
    encrypted_image: Vec<u8>,
    max_views: u32,
    image_id: String,
}
```
- Stores encrypted image for each recipient
- Initializes view counters
- Returns `SendImageResponse { success: bool }`

#### Query Received Images
```rust
Message::QueryReceivedImages {
    username: String,
}
```
- Returns list of images for user
- Shows remaining views for each
- Returns `QueryReceivedImagesResponse { images: Vec<ReceivedImageInfo> }`

#### View Image
```rust
Message::ViewImage {
    username: String,
    image_id: String,
}
```
- Retrieves encrypted image
- Decrements remaining views
- Deletes if views reach 0
- Returns `ViewImageResponse { image_data: Vec<u8>, remaining_views: u32 }`

---

## Client Services

### 5. Client API Service
**Module**: `src/client.rs`
**Type**: Library

**Purpose**: Client-side API for interacting with cloud nodes.

**Constructor**:
```rust
pub fn new(id: usize, cloud_addresses: Vec<String>) -> Self
```

**Methods**:

#### Session Management
```rust
pub async fn register_session(
    &self,
    client_id: String,
    username: String,
) -> Result<(), String>
```
- Registers username with cluster
- Tries all nodes until success
- Returns error if username taken

```rust
pub async fn unregister_session(
    &self,
    client_id: String,
    username: String,
)
```
- Unregisters from all nodes
- Fire-and-forget async

```rust
pub async fn check_username_available(
    &self,
    username: String,
) -> Result<bool, String>
```
- Checks if username is available
- Returns true if available (not in use)

#### Image Operations
```rust
pub async fn send_encryption_request(
    &self,
    request_id: String,
    client_username: String,
    image_data: Vec<u8>,
    usernames: Vec<String>,
    quota: u32,
) -> Result<Message, String>
```
- Multicasts encryption request to all nodes
- Returns first successful response
- Contains encrypted image in `EncryptionResponse`

```rust
pub async fn send_image(
    &self,
    from_username: String,
    to_usernames: Vec<String>,
    encrypted_image: Vec<u8>,
    max_views: u32,
    image_id: String,
) -> Result<String, String>
```
- Stores encrypted image for recipients
- Enforces view quota

```rust
pub async fn query_received_images(
    &self,
    username: String,
) -> Result<Vec<ReceivedImageInfo>, String>
```
- Lists images sent to user
- Shows sender and remaining views

```rust
pub async fn view_image(
    &self,
    username: String,
    image_id: String,
) -> Result<(Vec<u8>, u32), String>
```
- Retrieves and decrypts image
- Decrements view count
- Returns decrypted image + remaining views

---

### 6. Client GUI Service
**Binary**: `client-gui`
**Location**: `src/bin/client_gui.rs`, `src/gui_client.rs`
**Type**: Desktop application (egui)

**Purpose**: User-friendly graphical interface for image encryption and sharing.

**Features**:

#### Upload Tab
- Image file selection (PNG, JPG, BMP)
- Image preview
- Username management (add authorized users)
- Viewing quota configuration
- Encryption request submission
- Save encrypted image
- Send to users

#### Received Images Tab
- List received images
- View image details (sender, views remaining)
- Decrypt and display images
- Auto-refresh

#### History Tab
- View encryption request history
- Success/failure status
- Request timing
- User count

#### Settings Tab
- Client ID configuration
- Username display
- Cloud node address management

**Launch**:
```bash
client-gui [client_id] [node1_addr] [node2_addr] [node3_addr]
Example: client-gui alice 127.0.0.1:8001 127.0.0.1:8002 127.0.0.1:8003
```

---

### 7. Server Monitor GUI Service
**Binary**: `server-gui`
**Location**: `src/bin/server_gui.rs`, `src/gui_server.rs`
**Type**: Desktop application (egui)

**Purpose**: Real-time monitoring dashboard for cloud node cluster.

**Features**:

#### Overview Tab
- Node state (ACTIVE/FAILED/RECOVERING)
- Load metrics with progress bar
- Queue length
- Processed request count
- Coordinator status
- Peer node list

#### Logs Tab
- Real-time log streaming
- Color-coded severity
- Search/filter
- Auto-scroll
- Clear logs

#### Metrics Tab
- Total requests processed
- Success/failure statistics
- Success rate percentage
- Average latency
- P95 latency
- Election history

#### Network Tab
- Cluster overview
- All nodes status table
- Load distribution
- Current coordinator
- Network health

**Launch**:
```bash
server-gui [node_addr1] [node_addr2] [node_addr3]
Example: server-gui 127.0.0.1:8001 127.0.0.1:8002 127.0.0.1:8003
```

---

## Support Services

### 8. Load Balancing Service
**Module**: `src/node.rs` (forward_to_least_loaded_peer)
**Type**: Distributed algorithm

**Purpose**: Distribute encryption workload across nodes based on current load.

**Algorithm**:
1. Coordinator receives encryption request
2. Queries all peer nodes for current load via `LoadQuery`
3. Receives `LoadResponse` with load percentage
4. Selects node with lowest load
5. Forwards request via `EncryptionRequest { forwarded: true }`
6. Selected node processes and responds

**Load Calculation**:
```rust
load_percentage = (queue_length * 100) / MAX_QUEUE_SIZE
```

**Decision Logic**:
- If current node load < 50%: Process locally
- If current node load >= 50%: Forward to peer with lowest load
- If all peers > 80% load: Process locally anyway

---

### 9. Leader Election Service
**Module**: `src/node.rs` (start_election, handle_election_message)
**Type**: Bully algorithm implementation

**Purpose**: Elect a coordinator node to manage load balancing.

**Algorithm**:
1. **Trigger**: Node detects coordinator failure via heartbeat timeout
2. **Election Phase**:
   - Node sends `Election { candidate_id }` to all higher-ID nodes
   - If any higher node responds: Wait for coordinator announcement
   - If no response within 2s: Declare self as coordinator
3. **Announcement Phase**:
   - Winner sends `Coordinator { coordinator_id }` to all nodes
   - All nodes update their coordinator reference

**States**:
- `ACTIVE` - Normal operation
- `ELECTION` - Election in progress
- `RECOVERING` - Rejoining after failure

**Timeout Values**:
- Heartbeat interval: 5s
- Heartbeat timeout: 15s (3 missed heartbeats)
- Election response timeout: 2s

---

### 10. Heartbeat Service
**Module**: `src/node.rs` (send_heartbeats)
**Type**: Background task per node

**Purpose**: Monitor peer node health and detect failures.

**Operation**:
- Every 5 seconds, send `Heartbeat { node_id, load }` to all peers
- Receive heartbeats from peers and update last_seen timestamp
- If peer not seen for 15 seconds: Mark as FAILED and trigger election

**Heartbeat Message**:
```rust
Message::Heartbeat {
    node_id: usize,
    load: u32,  // Current load percentage
}
```

**Failure Detection**:
```rust
if (now - last_seen) > 15 seconds {
    peer.state = NodeState::FAILED
    if peer.id == coordinator_id {
        trigger_election()
    }
}
```

---

### 11. Chunking Service
**Module**: `src/chunking.rs`
**Type**: Library

**Purpose**: Fragment large messages into UDP-safe chunks for transmission.

**Configuration**:
- Chunk size: 45,000 bytes (safe for UDP 65KB limit)
- Header overhead: ~200 bytes per chunk

**Data Structures**:
```rust
pub struct ChunkedMessage {
    pub message_id: String,     // UUID for reassembly
    pub chunk_index: usize,     // Current chunk number
    pub total_chunks: usize,    // Total chunks in message
    pub data: Vec<u8>,          // Chunk payload
}

pub struct ChunkReassembler {
    chunks: HashMap<String, Vec<Option<Vec<u8>>>>,
}
```

**Functions**:

#### Fragment Message
```rust
pub fn fragment(data: Vec<u8>) -> Vec<ChunkedMessage>
```
- Splits data into 45KB chunks
- Generates UUID for message
- Returns vector of chunks

#### Reassemble Chunks
```rust
pub fn process_chunk(&mut self, chunk: ChunkedMessage) -> Option<Vec<u8>>
```
- Stores chunk by message_id
- Returns complete data when all chunks received
- Returns None if still waiting for chunks

---

### 12. Metrics Collection Service
**Module**: `src/metrics.rs`
**Type**: Per-node metrics tracking

**Purpose**: Collect and report performance metrics for monitoring.

**Metrics Tracked**:
```rust
pub struct MetricsCollector {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    latencies: Arc<RwLock<Vec<u64>>>,  // Milliseconds
}
```

**Operations**:
- `record_success(duration_ms)` - Record successful request
- `record_failure()` - Record failed request
- `get_stats()` - Get aggregated statistics
- `get_average_latency()` - Calculate mean latency
- `get_p95_latency()` - Calculate 95th percentile

**Statistics**:
```rust
pub struct MetricsStats {
    pub total: u64,
    pub successful: u64,
    pub failed: u64,
    pub success_rate: f64,  // Percentage
    pub avg_latency: f64,   // Milliseconds
    pub p95_latency: f64,   // Milliseconds
}
```

---

## Message Protocol

### Protocol Specification
**Format**: JSON over UDP
**Max Size**: 65,507 bytes (UDP limit)
**Chunking**: Messages > 45KB are fragmented

### Message Types

#### Session Messages
```rust
SessionRegister { client_id: String, username: String }
SessionRegisterResponse { success: bool, error: Option<String> }
SessionUnregister { client_id: String, username: String }
CheckUsernameAvailable { username: String }
CheckUsernameResponse { available: bool }
```

#### Image Processing Messages
```rust
EncryptionRequest {
    request_id: String,
    client_username: String,
    image_data: Vec<u8>,
    usernames: Vec<String>,
    quota: u32,
    forwarded: bool,
    client_address: Option<String>,
}

EncryptionResponse {
    request_id: String,
    encrypted_image: Vec<u8>,
    success: bool,
    error: Option<String>,
}

DecryptionRequest {
    request_id: String,
    encrypted_image: Vec<u8>,
}

DecryptionResponse {
    request_id: String,
    decrypted_image: Vec<u8>,
    success: bool,
    error: Option<String>,
}
```

#### Image Storage Messages
```rust
SendImage {
    from_username: String,
    to_usernames: Vec<String>,
    encrypted_image: Vec<u8>,
    max_views: u32,
    image_id: String,
}

SendImageResponse {
    success: bool,
    error: Option<String>,
}

QueryReceivedImages {
    username: String,
}

QueryReceivedImagesResponse {
    images: Vec<ReceivedImageInfo>,
}

ViewImage {
    username: String,
    image_id: String,
}

ViewImageResponse {
    image_data: Vec<u8>,
    remaining_views: u32,
    success: bool,
    error: Option<String>,
}
```

#### Coordination Messages
```rust
Heartbeat {
    node_id: usize,
    load: u32,
}

Election {
    candidate_id: usize,
}

ElectionResponse {
    node_id: usize,
}

Coordinator {
    coordinator_id: usize,
}

LoadQuery {
    request_id: String,
}

LoadResponse {
    node_id: usize,
    load: u32,
}
```

---

## API Reference

### Client API Quick Reference

```rust
// Session Management
client.register_session(client_id, username).await?;
client.unregister_session(client_id, username).await;
let available = client.check_username_available(username).await?;

// Image Encryption
let response = client.send_encryption_request(
    request_id,
    client_username,
    image_data,
    vec!["alice".to_string(), "bob".to_string()],
    5,  // 5 views
).await?;

// Image Sharing
client.send_image(
    from_username,
    vec!["alice".to_string()],
    encrypted_image,
    5,
    image_id,
).await?;

// View Images
let images = client.query_received_images(username).await?;
let (image_data, remaining) = client.view_image(username, image_id).await?;
```

### Node API Quick Reference

```rust
// Start node
let node = Node::new(node_id, port, peer_addresses).await?;
node.run().await;

// Metrics
let stats = node.metrics.get_stats();
println!("Success rate: {:.2}%", stats.success_rate);
println!("Avg latency: {:.2}ms", stats.avg_latency);
```

---

## Service Endpoints

| Service | Type | Address | Port |
|---------|------|---------|------|
| Cloud Node 1 | UDP | 127.0.0.1 | 8001 |
| Cloud Node 2 | UDP | 127.0.0.1 | 8002 |
| Cloud Node 3 | UDP | 127.0.0.1 | 8003 |
| Client GUI | Local | N/A | N/A |
| Server GUI | Local | N/A | N/A |

---

## Dependencies

### Core Dependencies
- `tokio` - Async runtime
- `serde` / `serde_json` - Serialization
- `log` / `env_logger` - Logging
- `image` - Image processing
- `uuid` - Unique IDs

### GUI Dependencies
- `eframe` / `egui` - GUI framework
- `rfd` - File dialogs
- `poll-promise` - Async in GUI

### Network Dependencies
- `tokio::net::UdpSocket` - UDP networking
- Custom chunking protocol

---

## Service Health Checks

```bash
# Check if nodes are running
ps aux | grep cloud-node

# Check node ports
netstat -tulpn | grep -E "800[1-3]"

# Test node connectivity
echo '{"Heartbeat":{"node_id":99,"load":0}}' | nc -u 127.0.0.1 8001

# Monitor logs
RUST_LOG=debug target/release/cloud-node 1 8001
```

---

## Performance Characteristics

| Service | Latency | Throughput |
|---------|---------|------------|
| Encryption | 50-60ms | 1.5 MB/s |
| Decryption | 25-30ms | 3.2 MB/s |
| Session Register | 5-10ms | N/A |
| Image Send | 10-20ms | N/A |
| Image View | 30-40ms | 3 MB/s |
| Heartbeat | 1-2ms | N/A |
| Election | 50-100ms | N/A |

---

**Last Updated**: 2025-11-23
**Version**: 1.0 (Steganography encryption)
