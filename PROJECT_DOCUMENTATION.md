# Distributed Image Sharing Cloud - Complete Documentation

**Last Updated:** December 3, 2025  
**System Type:** Peer-to-Peer Distributed Cloud Storage with LSB Steganography Encryption

---

## Table of Contents
1. [System Overview](#system-overview)
2. [Architecture](#architecture)
3. [Encryption Method](#encryption-method)
4. [Coordinator Election](#coordinator-election)
5. [Communication Protocol](#communication-protocol)
6. [Quick Start](#quick-start)
7. [Testing](#testing)
8. [File Structure](#file-structure)

---

## System Overview

A distributed cloud storage system where multiple nodes collaborate to provide encrypted image storage and sharing. Key features:

- **Peer-to-Peer Architecture:** 3+ nodes communicate directly (no central server)
- **LSB Steganography Encryption:** Images hidden inside cover images
- **Periodic Coordinator Election:** Leader elected every 60 seconds based on lowest load
- **Access Control:** Username-based permissions with viewing quotas
- **UDP Communication:** Chunked packet transmission (45KB chunks) for large images
- **GUI Interface:** egui-based client and server monitoring interfaces

---

## Architecture

### Node Types
- **Cloud Nodes:** Storage nodes that hold encrypted images (IDs: 1-255)
- **Coordinator:** Current leader that processes all encryption/decryption requests
- **Clients:** Users who upload/download images

### Port Structure
Each node with ID `N` uses:
- **Main Port:** User-configured (e.g., 8001, 8002, 8003)
- **Message Port:** `9000 + N` (for peer-to-peer messages)
- **Heartbeat Port:** `10000 + N` (for health checks)

Example for Node 2:
- Main: 8002
- Messages: 9002
- Heartbeat: 10002

### Request Flow

```
1. Client → Any Node (port 8001-8003)
2. Node checks: Am I coordinator?
   - YES → Process request locally
   - NO  → Forward to coordinator (port 9000+id)
3. Coordinator → Processes request → Client (direct response)
```

**Key Point:** Coordinator responds **directly** to client, not through forwarding node.

---

## Encryption Method

### LSB Steganography (No Scrambling)

**Encryption Process:**
1. Load cover image (`encrypction_key.jpg`) as "encryption key"
2. Prepare payload: metadata (usernames, quota) + original image bytes
3. Embed into LSBs (Least Significant Bits) of cover pixels:
   ```
   Structure: [metadata_len][metadata_json][image_len][image_data]
   ```
4. For each bit of payload, modify last bit of a cover pixel:
   - `pixel = (pixel & 0xFE) | bit_value`
   - Changes pixel by ±1 (imperceptible)
5. Output as PNG (lossless format preserves hidden bits)

**Decryption Process:**
1. Read encrypted image (looks like cover)
2. Extract LSBs from each pixel in sequence
3. Reassemble bits → bytes → metadata + original image
4. Return original image + access control metadata

**Security:**
- Encrypted image looks identical to cover image
- Without proper cover structure, data cannot be extracted
- Cover image acts as the decryption key
- No visual indication of hidden data

**Example:**
```
Original:  vacation.jpg (1MB)
Cover:     encrypction_key.jpg (landscape photo)
Result:    Looks like landscape, contains vacation photo in LSBs
```

---

## Coordinator Election

### Periodic Election System (Every 60 Seconds)

**Election Algorithm:** Bully Algorithm (Modified for Load-Based Selection)

**Process:**
1. Every 60 seconds, `periodic_election_task()` triggers election
2. All nodes broadcast their current load (active tasks)
3. Node with **lowest load** becomes coordinator
4. In case of tie, **lowest node ID** wins
5. All nodes update their coordinator reference
6. Coordinator processes **all requests** for next 60 seconds

**Load Calculation:**
```rust
fn current_load(&self) -> f64 {
    self.active_tasks.load(Ordering::Relaxed) as f64
}
```

**Visual Indicators:**
- 👑 = "I am coordinator"
- 📤 = "Forwarding request to coordinator"

**Implementation:**
```rust
// In node.rs
async fn periodic_election_task() {
    loop {
        sleep(Duration::from_secs(60)).await;
        trigger_election().await;
    }
}
```

**Benefits:**
- Automatic load balancing
- Fault tolerance (re-election if coordinator fails)
- No per-request overhead
- Clear authority model

---

## Communication Protocol

### Message Types (src/messages.rs)

1. **EncryptionRequest**
   - Purpose: Upload and encrypt image
   - Fields: image_data, usernames, quota, sender_addr
   - Response: EncryptionResponse (encrypted_data, image_id)

2. **ViewRequest**
   - Purpose: Decrypt and view image
   - Fields: image_id, username, sender_addr
   - Response: DecryptionResponse (decrypted_image, remaining_views)

3. **LoadQuery**
   - Purpose: Check node workload
   - Response: LoadResponse (current_load)

4. **Coordinator**
   - Purpose: Announce new coordinator
   - Fields: node_id, load

5. **Heartbeat**
   - Purpose: Node health monitoring
   - Sent every 2 seconds

### UDP Chunking (src/chunking.rs)

Large messages split into 45KB chunks:
```rust
Chunk Structure:
- message_id: u32 (4 bytes)
- chunk_index: u16 (2 bytes)
- total_chunks: u16 (2 bytes)
- data: Vec<u8> (up to 45KB)
```

**Process:**
1. Serialize message (serde)
2. Split into 45KB chunks
3. Send each chunk with header
4. Receiver reassembles when all chunks arrive

---

## Quick Start

### 1. Build the Project
```bash
cargo build --release
```

### 2. Start Nodes (3-node cluster)

**Terminal 1:**
```bash
cargo run --release --bin cloud-node -- 1 127.0.0.1:8001 127.0.0.1:8002,127.0.0.1:8003
```

**Terminal 2:**
```bash
cargo run --release --bin cloud-node -- 2 127.0.0.1:8002 127.0.0.1:8001,127.0.0.1:8003
```

**Terminal 3:**
```bash
cargo run --release --bin cloud-node -- 3 127.0.0.1:8003 127.0.0.1:8001,127.0.0.1:8002
```

### 3. Run Client GUI
```bash
cargo run --release --bin client-gui
```

### 4. Upload Image
1. Connect to any node (127.0.0.1:8001)
2. Select image file
3. Enter usernames (comma-separated)
4. Set viewing quota
5. Click "Encrypt & Upload"

### 5. View Image
1. Enter image ID
2. Enter username
3. Click "Request Image"

---

## Testing

### Single Image Test
```bash
cd single_img_testing
./run_test.sh
```

### Multiple Image Test
```bash
cd multiple_img_testing
./run_full_test.sh
```

### Stress Testing

**10 Concurrent Requests:**
```bash
python3 stress_test_10.py
```

**1000 Sequential Requests:**
```bash
python3 stress_test_1000.py
```

**Concurrent with Failures:**
```bash
python3 stress_test_with_failures.py
```

### Manual Testing

**Start 3x3 local grid:**
```bash
./start_local_3x3.sh
```

**Run encryption test:**
```bash
./test_encryption.sh
```

---

## File Structure

### Core Source Files (src/)

- **node.rs** (1695 lines) - Main cloud node server, handles requests, elections, storage
- **election.rs** (204 lines) - Bully algorithm implementation
- **encryption.rs** (284 lines) - LSB steganography encryption/decryption
- **messages.rs** (236 lines) - Protocol message definitions
- **chunking.rs** (276 lines) - UDP packet chunking/reassembly
- **client.rs** (599 lines) - Client API for cloud communication
- **gui_client.rs** (1000 lines) - egui-based client GUI
- **gui_server.rs** (800 lines) - egui-based server monitoring GUI
- **metrics.rs** - Performance metrics collection

### Binary Entry Points (src/bin/)

- **cloud_node.rs** - Cloud node executable
- **client.rs** - CLI client
- **client_gui.rs** - GUI client launcher
- **server_gui.rs** - GUI server monitor launcher
- **demo.rs** - Demo mode
- **test_encryption.rs** - Encryption testing tool

### Configuration Files

- **Cargo.toml** - Rust dependencies (tokio, serde, image, egui, etc.)
- **users.txt** - User authentication database
- **encrypction_key.jpg** - Cover image for steganography

### Scripts

- **run_nodes.sh** - Start node cluster
- **stop_nodes.sh** - Stop all nodes
- **start_local_3x3.sh** - Start 9-node local grid
- **run_gui_demo.sh** - Start GUI demo
- **test_encryption.sh** - Test encryption manually
- **cleanup.sh** - Clean temporary files

### Test Suites

- **single_img_testing/** - Single image upload/download tests
- **multiple_img_testing/** - Batch testing with metrics
- **stress_test_*.py** - Performance and load testing

---

## Key Design Decisions

1. **Why LSB Steganography?**
   - Encrypted images look benign (just photos)
   - No suspicious file formats
   - Cover image acts as encryption key
   - Simple yet effective

2. **Why 60-Second Elections?**
   - Balance between responsiveness and stability
   - Avoids election overhead on every request
   - Allows coordinator to optimize batch operations
   - Long enough for meaningful work periods

3. **Why UDP over TCP?**
   - Lower latency for peer-to-peer communication
   - Custom chunking allows large transfers
   - Better for LAN environments
   - No connection state overhead

4. **Why Bully Algorithm?**
   - Simple implementation
   - Deterministic outcomes (lowest load wins)
   - Well-understood failure handling
   - Works well with load-based priorities

---

## Troubleshooting

**Node won't start:**
- Check if port is already in use: `netstat -tulpn | grep 8001`
- Ensure encrypction_key.jpg exists in project root
- Verify peer addresses are reachable

**Encryption fails:**
- Check cover image file: `encrypction_key.jpg`
- Ensure image is large enough (capacity = pixels × 3 bits)
- Verify PNG encoding libraries installed

**Client can't connect:**
- Confirm nodes are running: `ps aux | grep cloud-node`
- Check firewall rules
- Verify node addresses match client configuration

**Coordinator not elected:**
- Check node logs for election messages
- Verify heartbeat messages (every 2 seconds)
- Ensure at least 2 nodes are running

---

## Performance Characteristics

- **Encryption Time:** ~100-500ms per image (depends on size)
- **Network Transfer:** ~1-2 seconds for 1MB image (UDP chunked)
- **Election Time:** ~500ms for 3-node cluster
- **Concurrent Capacity:** 50+ requests/second (3-node cluster)
- **Storage Limit:** ~10MB per encrypted image (cover size dependent)

---

## Future Enhancements

- [ ] Replication across multiple nodes
- [ ] Byzantine fault tolerance
- [ ] Dynamic cluster scaling
- [ ] Encrypted metadata storage
- [ ] Image compression optimization
- [ ] TLS for client connections

---

## License & Credits

Distributed Image Sharing Cloud  
Built with Rust, Tokio, and egui  
Uses LSB Steganography for encryption
