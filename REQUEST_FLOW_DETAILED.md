# Complete Request Flow: From Client to Server and Back

This document provides a detailed explanation of what happens when a server receives an encryption request, including the encryption mechanism, storage behavior, and complete message flow.

## Table of Contents
1. [Overview](#overview)
2. [Complete Request Flow](#complete-request-flow)
3. [Encryption Details](#encryption-details)
4. [Storage Behavior](#storage-behavior)
5. [Load Balancing Logic](#load-balancing-logic)
6. [Response Flow](#response-flow)

---

## Overview

### What Happens When a Server Receives a Request?

**Short Answer**: The server **encrypts the image and sends it back** to the client. It does **NOT** store the encrypted image on the server.

**Storage Clarification**: 
- ✅ Servers DO store images when explicitly sent via `SendImage` message (for image sharing between users)
- ❌ Servers DO NOT store images during encryption requests - they just encrypt and return

---

## Complete Request Flow

### Phase 1: Client Sends Request

**Location**: `src/client.rs` - `send_encryption_request()` (lines 84-127)

```rust
pub async fn send_encryption_request(
    &self,
    request_id: String,
    client_username: String,
    image_data: Vec<u8>,      // Raw image bytes (JPEG/PNG)
    usernames: Vec<String>,   // Who can view this image
    quota: u32,               // How many times it can be viewed
) -> Result<Message, String>
```

**What happens**:
1. Client creates an `EncryptionRequest` message with:
   - `request_id`: Unique identifier for this request
   - `client_username`: Who is sending the image
   - `image_data`: Raw image bytes (could be 10KB - 50KB)
   - `usernames`: List of authorized viewers (e.g., ["alice", "bob"])
   - `quota`: Maximum number of views (e.g., 5)
   - `forwarded: false`: Not yet forwarded

2. Client **multicasts** this request to **all known cloud nodes** simultaneously
3. Client waits for **first successful response** (ignores slower nodes)

**Example Message**:
```json
{
  "EncryptionRequest": {
    "request_id": "client_1_stress_50_42_1730505600",
    "client_username": "user1",
    "image_data": [255, 216, 255, ...],  // JPEG bytes
    "usernames": ["user2", "user3"],
    "quota": 5,
    "forwarded": false
  }
}
```

---

### Phase 2: Server Receives Request

**Location**: `src/node.rs` - `handle_message()` (lines 170-364)

When any server receives the request, it goes through **load balancing logic**:

#### Case A: Non-Coordinator Node Receives Request

**Flow**:
1. Node checks: "Am I the coordinator?"
2. If NO → **Forward to coordinator** for load balancing
3. Wait for coordinator's response
4. Return response to client

**Code Path** (lines 205-277):
```rust
if coordinator_id != self.id {
    // Not coordinator - forward to current coordinator for load balancing
    info!("[Node {}] Forwarding request {} to coordinator Node {}", 
          self.id, request_id, coordinator_id);
    
    // Forward to coordinator
    match self.send_message_to_node(coordinator_id, forward_message).await {
        Ok(Some(response)) => Some(response),
        // ... fallback logic if coordinator fails
    }
}
```

#### Case B: Coordinator Node Receives Request

**Flow**:
1. Node checks: "Am I the coordinator?"
2. If YES → **Perform load balancing**:
   - Query all peer nodes for their current load
   - Find the node with **lowest load**
   - Decision time:
     - If coordinator has lowest load → **Process locally**
     - If another node has lower load → **Forward to that node**

**Code Path** (lines 278-364):
```rust
// This node IS the coordinator - perform load balancing
info!("[Node {}] Coordinator performing load balancing for request {}", 
      self.id, request_id);

// Query all nodes for their current load
let lowest_load_node = self.find_lowest_load_node().await;

if lowest_load_node == self.id {
    // Process locally (lowest load)
    // ... increment queue, process, decrement queue
} else {
    // Forward to lowest-load node
    let forward_message = Message::EncryptionRequest {
        // ... same data ...
        forwarded: true,  // Mark as forwarded to prevent loops
    };
}
```

#### Case C: Node Receives Forwarded Request

**Flow**:
1. Node checks: `forwarded == true`?
2. If YES → **MUST process locally** (no more forwarding!)
3. This prevents infinite forwarding loops

**Code Path** (lines 183-204):
```rust
if forwarded {
    // Request forwarded by coordinator - MUST process locally
    info!("[Node {}] Processing forwarded request {} locally", 
          self.id, request_id);
    
    // Increment queue length
    {
        let mut queue = self.queue_length.write().await;
        *queue += 1;
    }

    // Process encryption
    let result = self_clone
        .process_encryption_request(request_id, image_data, usernames, quota)
        .await;

    // Decrement queue length
    {
        let mut queue = self.queue_length.write().await;
        *queue = queue.saturating_sub(1);
    }

    Some(result)
}
```

---

### Phase 3: Server Processes Encryption

**Location**: `src/node.rs` - `process_encryption_request()` (lines 520-562)

```rust
async fn process_encryption_request(
    &self,
    request_id: String,
    image_data: Vec<u8>,      // Raw image from client
    usernames: Vec<String>,   // Authorized viewers
    quota: u32,               // View limit
) -> Message
```

**What happens**:
1. **Update load metrics**:
   ```rust
   let queue = *self.queue_length.read().await;
   let mut load = self.current_load.write().await;
   *load = queue as f64;  // Load = queue length directly
   ```

2. **Call encryption function**:
   ```rust
   match encryption::encrypt_image(image_data, usernames, quota).await {
       Ok(encrypted_image) => {
           // Success! Increment processed counter
           let mut processed = self.processed_requests.write().await;
           *processed += 1;
           
           // Return success response
           Message::EncryptionResponse {
               request_id,
               encrypted_image,  // ← Encrypted image bytes
               success: true,
               error: None,
           }
       }
       Err(e) => {
           // Return error response
           Message::EncryptionResponse {
               request_id,
               encrypted_image: vec![],
               success: false,
               error: Some(e),
           }
       }
   }
   ```

3. **No storage happens here!** The encrypted image is returned directly to the client.

---

## Encryption Details

**Location**: `src/encryption.rs` - `encrypt_image()` (lines 16-142)

### Step-by-Step Encryption Process

#### Step 1: Simulate Processing Load
```rust
// Simulate heavy computational load (critical for load testing)
let processing_delay = Duration::from_millis(500 + (image_data.len() / 100) as u64);
sleep(processing_delay).await;
```
- **Base delay**: 500ms
- **Size penalty**: +10ms per 1KB of image data
- **Purpose**: Simulate realistic CPU-intensive encryption

#### Step 2: Decode Image Format
```rust
let format = image::guess_format(&image_data)
    .map_err(|e| format!("Cannot detect image format: {}", e))?;

let img = image::load_from_memory(&image_data)
    .map_err(|e| format!("Failed to decode image: {}", e))?;

let mut rgba_img = img.to_rgba8();
let (width, height) = rgba_img.dimensions();
```
- **Input**: Raw JPEG/PNG bytes
- **Output**: RGBA pixel array (4 bytes per pixel: Red, Green, Blue, Alpha)
- **Example**: 640x480 image = 1,228,800 bytes of pixel data

#### Step 3: Embed Metadata Using LSB Steganography

**What is LSB Steganography?**
- LSB = "Least Significant Bit"
- Each pixel byte has 8 bits
- We modify only the **last bit** (LSB) of each byte
- This is **invisible** to the human eye (changes value by max ±1)

**Metadata Structure**:
```rust
pub struct ImageMetadata {
    pub usernames: Vec<String>,  // ["alice", "bob"]
    pub quota: u32,              // 5
}
```

**Embedding Process**:
```rust
// 1. Convert metadata to JSON
let metadata = ImageMetadata { usernames, quota };
let metadata_json = serde_json::to_string(&metadata)?;
// Example: {"usernames":["alice","bob"],"quota":5}

// 2. Embed metadata LENGTH (4 bytes = 32 bits)
let metadata_len = metadata_bytes.len() as u32;
let len_bytes = metadata_len.to_be_bytes();  // [0, 0, 0, 45] for 45 bytes

// Embed each bit of length into LSB of first 32 pixels
for (i, &byte) in len_bytes.iter().enumerate() {
    for bit in 0..8 {
        let bit_value = (byte >> (7 - bit)) & 1;  // Extract bit
        let pixel_index = i * 8 + bit;
        // Replace LSB of pixel with our bit
        pixels[pixel_index] = (pixels[pixel_index] & 0xFE) | bit_value;
    }
}

// 3. Embed metadata CONTENT (starting at bit 32)
for (i, &byte) in metadata_bytes.iter().enumerate() {
    for bit in 0..8 {
        let bit_value = (byte >> (7 - bit)) & 1;
        let pixel_index = 32 + i * 8 + bit;  // After length
        pixels[pixel_index] = (pixels[pixel_index] & 0xFE) | bit_value;
    }
}
```

**Visual Example**:
```
Original pixel: 11010110 (214)
Our bit:        1
Result:         11010111 (215)  ← Only LSB changed!

Original pixel: 11010110 (214)
Our bit:        0
Result:         11010110 (214)  ← No change if bit matches
```

#### Step 4: Scramble Pixels (Visual Encryption)

**Purpose**: Make image visually unrecognizable (looks like noise/static)

```rust
// Calculate deterministic seed from metadata
fn calculate_seed(metadata: &ImageMetadata) -> u64 {
    let mut hasher = DefaultHasher::new();
    for username in &metadata.usernames {
        username.hash(&mut hasher);
    }
    metadata.quota.hash(&mut hasher);
    hasher.finish()  // Returns u64 seed
}

let seed = calculate_seed(&metadata);
scramble_pixels(pixels, seed);
```

**Scrambling Algorithm: Fisher-Yates Shuffle**
```rust
fn scramble_pixels(pixels: &mut [u8], seed: u64) {
    let len = pixels.len() / 4;  // Number of RGBA pixels
    
    // Use Linear Congruential Generator for deterministic randomness
    let mut rng_state = seed;
    
    for i in (1..len).rev() {
        // Generate pseudo-random index
        rng_state = rng_state.wrapping_mul(6364136223846793005)
                             .wrapping_add(1442695040888963407);
        let j = (rng_state % (i as u64 + 1)) as usize;
        
        // Swap pixels (4 bytes each: RGBA)
        for k in 0..4 {
            pixels.swap(i * 4 + k, j * 4 + k);
        }
    }
}
```

**Visual Effect**:
- **Before**: Clear image of a cat
- **After**: Looks like TV static/noise (pixels randomly rearranged)
- **Key Point**: Same seed will produce **same scrambling** (deterministic)

#### Step 5: Compress for UDP Transmission

```rust
// Check if image is too large
let estimated_size = (width * height * 3) / 2;  // ~1.5 bytes per pixel for JPEG
let max_safe_size = 30000;  // 30KB target

// Resize if needed
if estimated_size > max_safe_size {
    let scale = ((max_safe_size as f32) / (estimated_size as f32)).sqrt();
    let new_width = ((width as f32) * scale) as u32;
    let new_height = ((height as f32) * scale) as u32;
    
    resized_img = dynamic_img.resize(new_width, new_height, FilterType::Lanczos3);
}

// Encode as JPEG with quality 60
let mut output_bytes = Vec::new();
let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 60);
resized_img.write_with_encoder(encoder)?;

// Final safety check
if output_bytes.len() > 50000 {
    return Err("Encrypted image too large for UDP".to_string());
}
```

**UDP Packet Limit**: 65,535 bytes maximum
- JSON overhead: ~200 bytes
- Safe limit: 50KB for encrypted image
- Result: Most images fit in a **single UDP packet**

---

### Encryption Summary

**Input**: 
- Raw image (JPEG/PNG, 20KB)
- Usernames: ["alice", "bob"]
- Quota: 5

**Processing**:
1. ✅ Decode to RGBA pixels (1.2MB pixel data)
2. ✅ Embed metadata into LSBs (invisible)
3. ✅ Scramble pixels using seed (visual encryption)
4. ✅ Resize if needed (fit in UDP)
5. ✅ Compress as JPEG quality 60 (~25KB)

**Output**: 
- Encrypted image (25KB)
- Looks like static/noise
- Contains hidden metadata
- Can be unscrambled with correct seed

---

## Storage Behavior

### What Gets Stored? What Doesn't?

#### ❌ NOT Stored During Encryption Requests

```rust
// In process_encryption_request() - NO storage happens!
match encryption::encrypt_image(image_data, usernames, quota).await {
    Ok(encrypted_image) => {
        // Just return it! No storage!
        Message::EncryptionResponse {
            encrypted_image,  // ← Goes back to client
            success: true,
            error: None,
        }
    }
}
```

**Reason**: Encryption is a **stateless service**
- Client sends image
- Server encrypts it
- Server sends encrypted image back
- Server **forgets** about it

#### ✅ STORED During Image Sharing

**Location**: `src/node.rs` - `handle_message()` (lines 451-479)

```rust
Message::SendImage {
    from_username,
    to_usernames,
    encrypted_image,
    max_views,
    image_id,
} => {
    let mut stored = self.stored_images.write().await;
    let timestamp = chrono::Utc::now().timestamp();

    // Store image for each recipient
    for username in to_usernames {
        let image = StoredImage {
            image_id: image_id.clone(),
            from_username: from_username.clone(),
            encrypted_data: encrypted_image.clone(),
            remaining_views: max_views,
            max_views,
            timestamp,
        };

        stored.entry(username.clone())
              .or_insert_with(Vec::new)
              .push(image);
    }

    info!("[Node {}] Stored image {} from {}", 
          self.id, image_id, from_username);

    Some(Message::SendImageResponse {
        success: true,
        image_id,
        error: None,
    })
}
```

**Storage Structure**:
```rust
// In CloudNode struct
pub stored_images: Arc<RwLock<HashMap<String, Vec<StoredImage>>>>,
//                                      ^^^^^^  ^^^^^^^^^^^^^^^^
//                                      username -> list of images

pub struct StoredImage {
    pub image_id: String,           // "img_12345"
    pub from_username: String,      // "alice"
    pub encrypted_data: Vec<u8>,    // Encrypted JPEG bytes
    pub remaining_views: u32,       // 3 (decrements on view)
    pub max_views: u32,             // 5 (original quota)
    pub timestamp: i64,             // Unix timestamp
}
```

**Example Storage**:
```
Node 1's stored_images:
{
  "bob": [
    { image_id: "img_001", from: "alice", remaining_views: 3, ... },
    { image_id: "img_005", from: "charlie", remaining_views: 5, ... }
  ],
  "charlie": [
    { image_id: "img_002", from: "alice", remaining_views: 1, ... }
  ]
}
```

**When does storage happen?**
- User encrypts image → Gets encrypted image back → **NOT stored**
- User sends encrypted image to friends → **STORED on server** for each friend
- Friends can then **query** and **view** their received images

---

## Load Balancing Logic

### How Nodes Decide Who Processes Request

#### Step 1: Find Lowest Load Node

**Location**: `src/node.rs` - `find_lowest_load_node()` (lines 564-608)

```rust
async fn find_lowest_load_node(&self) -> NodeId {
    let mut lowest_load = *self.current_load.read().await;
    let mut lowest_node = self.id;
    
    info!("[Node {}] Current load: {:.2}", self.id, lowest_load);
    
    // Query all peer nodes SEQUENTIALLY
    for (peer_id, _) in &self.peer_addresses {
        let load_query = Message::LoadQuery { from_node: self.id };
        
        match self.send_message_to_node(*peer_id, load_query).await {
            Ok(Some(Message::LoadResponse { 
                node_id, load, queue_length, processed_count 
            })) => {
                info!("[Node {}] Node {} load: {:.2} (queue: {}, processed: {})", 
                      self.id, node_id, load, queue_length, processed_count);
                
                if load < lowest_load {
                    lowest_load = load;
                    lowest_node = node_id;
                }
            }
            _ => {
                warn!("[Node {}] No response from Node {}", self.id, peer_id);
            }
        }
    }
    
    info!("[Node {}] Selected Node {} (load: {:.2})", 
          self.id, lowest_node, lowest_load);
    
    lowest_node
}
```

#### Step 2: Load Calculation

**What is "load"?**

```rust
// In process_encryption_request()
let queue = *self.queue_length.read().await;
let mut load = self.current_load.write().await;
*load = queue as f64;  // Load = queue length directly
```

**Load Metric**:
- `load = 0.0` → Idle (no requests in queue)
- `load = 1.0` → 1 request being processed
- `load = 5.0` → 5 requests in queue
- `load = 10.0` → 10 requests in queue (heavily loaded)

**Example Scenario**:
```
Node 1: load = 2.0 (2 requests in queue)
Node 2: load = 0.0 (idle)
Node 3: load = 5.0 (heavily loaded)

Coordinator selects: Node 2 (lowest load = 0.0)
```

#### Step 3: Queue Management

```rust
// BEFORE processing
{
    let mut queue = self.queue_length.write().await;
    *queue += 1;  // Increment queue
}

// Process encryption (takes 500-1000ms)
let result = self.process_encryption_request(...).await;

// AFTER processing
{
    let mut queue = self.queue_length.write().await;
    *queue = queue.saturating_sub(1);  // Decrement queue
}
```

**Timeline**:
```
Time 0ms:   Request arrives, queue = 0 → 1, load = 1.0
Time 500ms: Encryption completes, queue = 1 → 0, load = 0.0
Time 0ms:   (Next request) queue = 0 → 1, load = 1.0
```

---

## Response Flow

### Phase 4: Server Sends Response Back

**Location**: `src/node.rs` - `handle_datagram()` (lines 128-163)

```rust
async fn handle_datagram(
    self: Arc<Self>,
    socket: Arc<UdpSocket>,
    data: Vec<u8>,
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    // ... parse message ...
    
    if let Some(response) = self.handle_message(message, addr).await {
        let response_bytes = serde_json::to_vec(&response)?;
        
        // Check size limit
        if response_bytes.len() > 65507 {
            error!("[Node {}] Response too large: {} bytes", 
                   self.id, response_bytes.len());
            // ... send error response ...
        } else {
            // Send response back to client
            socket.send_to(&response_bytes, addr).await?;
            debug!("[Node {}] Sent response: {} bytes", 
                   self.id, response_bytes.len());
        }
    }
    
    Ok(())
}
```

**Response Message**:
```json
{
  "EncryptionResponse": {
    "request_id": "client_1_stress_50_42_1730505600",
    "encrypted_image": [255, 216, 255, ...],  // Encrypted JPEG bytes
    "success": true,
    "error": null
  }
}
```

**Response Size**:
- Request ID: ~50 bytes
- Encrypted image: 20-40KB (compressed JPEG)
- JSON overhead: ~100 bytes
- **Total**: 20-40KB (fits in one UDP packet)

### Phase 5: Client Receives Response

**Location**: `src/client.rs` - `send_to_node()` (lines 129-200)

```rust
// Client is waiting for response...
let mut buffer = vec![0u8; 65535];

// Receive with 10-second timeout
let n = tokio::time::timeout(
    Duration::from_secs(10), 
    socket.recv_from(&mut buffer)
).await??;

// Parse response
let response: Message = serde_json::from_slice(&buffer[..n])?;

match response {
    Message::EncryptionResponse { 
        encrypted_image, success, error, .. 
    } => {
        if success {
            // Success! Client now has encrypted image
            println!("✅ Image encrypted successfully!");
            println!("   Size: {} bytes", encrypted_image.len());
            // Client can now:
            // 1. Save to file
            // 2. Send to other users
            // 3. Decrypt for viewing
        } else {
            println!("❌ Encryption failed: {:?}", error);
        }
    }
    _ => {
        println!("❌ Unexpected response");
    }
}
```

---

## Complete Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                        CLIENT (user1)                                │
│                                                                       │
│  1. User wants to encrypt image.jpg                                  │
│  2. Reads file: [255, 216, 255, ...] (20KB)                         │
│  3. Creates EncryptionRequest:                                       │
│     - usernames: ["user2", "user3"]                                  │
│     - quota: 5                                                       │
│  4. Multicasts to ALL nodes                                          │
└───────────────────────────┬───────────────────────────────────────────┘
                            │
              ┌─────────────┼─────────────┐
              │             │             │
              ▼             ▼             ▼
         ┌────────┐    ┌────────┐    ┌────────┐
         │ Node 1 │    │ Node 2 │    │ Node 3 │
         │load:2.0│    │load:0.0│    │load:5.0│
         └────────┘    └───┬────┘    └────────┘
              │            │              │
              │            │              │
              └────────────┼──────────────┘
                           │
                  ▼        ▼        ▼
            All nodes receive request
                           │
                  ┌────────┴────────┐
                  │ Is Coordinator? │
                  └────────┬────────┘
                           │
              YES ◄────────┴────────► NO
               │                      │
               │                      │ Forward to
               │                      │ Coordinator
               │                      └──────┐
               │                             │
               ▼                             │
    ┌──────────────────────┐                │
    │ Load Balancing       │◄───────────────┘
    │ Query all nodes      │
    │ Node 1: load = 2.0   │
    │ Node 2: load = 0.0   │ ← LOWEST!
    │ Node 3: load = 5.0   │
    └──────────┬───────────┘
               │
               ▼
      Forward to Node 2
      (forwarded=true)
               │
               ▼
         ┌────────────────────────────┐
         │ Node 2 Processes Request   │
         ├────────────────────────────┤
         │ 1. queue += 1 (now 1)      │
         │    load = 1.0              │
         │                             │
         │ 2. encrypt_image():         │
         │    - Sleep 500ms            │
         │    - Decode to RGBA         │
         │    - Embed metadata (LSB)   │
         │    - Scramble pixels        │
         │    - Compress JPEG Q60      │
         │    - Result: 25KB           │
         │                             │
         │ 3. processed_requests += 1  │
         │                             │
         │ 4. queue -= 1 (now 0)       │
         │    load = 0.0              │
         └────────────┬───────────────┘
                      │
                      ▼
            ┌─────────────────────┐
            │ EncryptionResponse  │
            │ encrypted_image:    │
            │ [255,216,255,...]   │
            │ (25KB JPEG)         │
            │ success: true       │
            └──────────┬──────────┘
                       │
                       │ UDP Response
                       │
                       ▼
            ┌──────────────────────┐
            │ CLIENT receives:     │
            │ - Encrypted image    │
            │ - Looks like noise   │
            │ - Has metadata       │
            │ - NOT stored         │
            └──────────────────────┘
                       │
                       ▼
            What client can do:
            1. Save to file ✓
            2. Send to friends ✓
            3. Decrypt + view ✓
```

---

## Key Takeaways

### 1. **Encryption Request Flow**
- ✅ Client → Server: Send raw image
- ✅ Server: Encrypt image (LSB + scramble)
- ✅ Server → Client: Return encrypted image
- ❌ Server does NOT store the encrypted image

### 2. **Encryption Mechanism**
- **LSB Steganography**: Hide metadata in pixel LSBs (invisible)
- **Fisher-Yates Shuffle**: Scramble pixels (visual encryption)
- **JPEG Compression**: Reduce size for UDP transmission
- **Deterministic**: Same seed → same scrambling (reversible)

### 3. **Storage Policy**
- **Encryption**: No storage (stateless service)
- **Image Sharing**: Stored on server (for recipients to view later)
- **Storage Location**: `HashMap<username, Vec<StoredImage>>`

### 4. **Load Balancing**
- **Coordinator**: Queries all nodes for load
- **Load Metric**: Queue length (# of requests being processed)
- **Decision**: Forward to lowest-load node
- **Prevents Loops**: `forwarded=true` flag

### 5. **Response Size**
- **Target**: < 50KB (safe for UDP)
- **Compression**: JPEG quality 60
- **Resize**: Automatic if needed
- **Failure**: Error if still > 50KB after compression

### 6. **Performance**
- **Base delay**: 500ms per request
- **Size penalty**: +10ms per 1KB
- **Concurrent**: Multiple requests processed in parallel
- **Load awareness**: Distributes work to idle nodes

---

## Files Reference

| File | Purpose | Key Functions |
|------|---------|---------------|
| `src/client.rs` | Client logic | `send_encryption_request()` |
| `src/node.rs` | Server logic | `handle_message()`, `process_encryption_request()`, `find_lowest_load_node()` |
| `src/encryption.rs` | Encryption/decryption | `encrypt_image()`, `decrypt_image()`, `scramble_pixels()` |
| `src/messages.rs` | Message types | `EncryptionRequest`, `EncryptionResponse`, `LoadQuery`, `LoadResponse` |

---

## FAQ

**Q: Does the server store images during encryption?**  
A: No. The server encrypts and returns the image immediately. Storage only happens during explicit image sharing.

**Q: What makes the encrypted image secure?**  
A: Two layers: (1) Metadata hidden in LSBs (steganography), (2) Pixels scrambled (visual encryption). Both are invisible and reversible.

**Q: Can encrypted images be viewed without decryption?**  
A: No. The scrambled image looks like static/noise. Decryption unscrambles the pixels using the correct seed.

**Q: What happens if UDP packet is too large?**  
A: Server automatically resizes and compresses. If still > 50KB, returns error.

**Q: How does load balancing prevent overload?**  
A: Coordinator queries all nodes, selects lowest-load node. Queue length reflects actual workload.

**Q: What if coordinator fails?**  
A: Elections run every 15 seconds. New coordinator elected automatically (Bully algorithm).

---

**Generated**: 2025-11-01  
**Version**: 1.0  
**Corresponds to**: `Stress` branch commit
