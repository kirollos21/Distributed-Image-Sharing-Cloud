# System Architecture - Distributed Image Cloud

## Overview

This document provides a comprehensive view of the system architecture, including component diagrams, data flow diagrams, and deployment architecture.

---

## Table of Contents

1. [High-Level Architecture](#high-level-architecture)
2. [Component Architecture](#component-architecture)
3. [Network Architecture](#network-architecture)
4. [Data Flow Diagrams](#data-flow-diagrams)
5. [Deployment Architecture](#deployment-architecture)
6. [Security Architecture](#security-architecture)

---

## High-Level Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                   DISTRIBUTED IMAGE CLOUD SYSTEM                    │
│                                                                     │
│  ┌────────────────┐         ┌─────────────────────────────────┐   │
│  │                │         │    CLOUD CLUSTER                │   │
│  │  CLIENT LAYER  │◄───────►│    (3 Nodes)                    │   │
│  │                │   UDP   │                                 │   │
│  └────────────────┘         │  ┌──────┐  ┌──────┐  ┌──────┐  │   │
│         │                   │  │Node 1│  │Node 2│  │Node 3│  │   │
│         │                   │  │:8001 │◄►│:8002 │◄►│:8003 │  │   │
│         ▼                   │  └──────┘  └──────┘  └──────┘  │   │
│  ┌────────────────┐         │      ▲         ▲         ▲     │   │
│  │ Client GUI     │         │      │         │         │     │   │
│  │ - Upload       │         │      └─────────┴─────────┘     │   │
│  │ - View         │         │     Peer-to-Peer Heartbeats    │   │
│  │ - Share        │         │     Leader Election            │   │
│  └────────────────┘         │     Load Balancing             │   │
│                             └─────────────────────────────────┘   │
│  ┌────────────────┐                                               │
│  │ Server GUI     │         ┌─────────────────────────────────┐   │
│  │ (Monitoring)   │◄───────►│   ENCRYPTION SERVICE            │   │
│  │ - Logs         │         │   - LSB Steganography           │   │
│  │ - Metrics      │         │   - Cover Image: encryption_key │   │
│  │ - Network      │         │   - Metadata Embedding          │   │
│  └────────────────┘         └─────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

### Key Components

1. **Client Layer**
   - Client GUI (egui-based desktop app)
   - Client API (Rust library)
   - Session management
   - Image upload/download

2. **Cloud Cluster**
   - 3-node distributed system
   - Peer-to-peer communication
   - Leader election (Bully algorithm)
   - Load balancing
   - Fault tolerance

3. **Encryption Service**
   - Image-on-image steganography
   - LSB (Least Significant Bit) encoding
   - Metadata embedding
   - PNG output format

4. **Monitoring**
   - Server GUI (real-time dashboard)
   - Metrics collection
   - Log aggregation

---

## Component Architecture

### Layered Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                      PRESENTATION LAYER                          │
│  ┌────────────────┐                  ┌────────────────┐          │
│  │  Client GUI    │                  │  Server GUI    │          │
│  │  (egui)        │                  │  (egui)        │          │
│  └───────┬────────┘                  └───────┬────────┘          │
└──────────┼───────────────────────────────────┼───────────────────┘
           │                                   │
┌──────────┼───────────────────────────────────┼───────────────────┐
│          │          APPLICATION LAYER        │                   │
│  ┌───────▼────────┐                  ┌───────▼────────┐          │
│  │  Client API    │                  │  Monitoring    │          │
│  │  - register()  │                  │  - collect()   │          │
│  │  - encrypt()   │                  │  - query()     │          │
│  │  - send()      │                  │  - display()   │          │
│  │  - view()      │                  └────────────────┘          │
│  └───────┬────────┘                                              │
└──────────┼───────────────────────────────────────────────────────┘
           │
┌──────────┼───────────────────────────────────────────────────────┐
│          │           BUSINESS LOGIC LAYER                        │
│  ┌───────▼─────────────────────────────────────────────┐         │
│  │                Cloud Node Service                   │         │
│  ├─────────────────────────────────────────────────────┤         │
│  │  • Request Handler      • Session Manager           │         │
│  │  • Load Balancer        • Image Storage             │         │
│  │  • Election Manager     • Request Cache             │         │
│  │  • Heartbeat Monitor    • Metrics Collector         │         │
│  └───────┬─────────────────────────────────────────────┘         │
│          │                                                        │
│  ┌───────▼────────┐       ┌────────────────┐                    │
│  │ Encryption     │       │ Chunking       │                    │
│  │ - encrypt()    │       │ - fragment()   │                    │
│  │ - decrypt()    │       │ - reassemble() │                    │
│  └────────────────┘       └────────────────┘                    │
└──────────────────────────────────────────────────────────────────┘
           │
┌──────────┼───────────────────────────────────────────────────────┐
│          │           NETWORK / TRANSPORT LAYER                   │
│  ┌───────▼────────┐                                              │
│  │  UDP Socket    │                                              │
│  │  - send_to()   │                                              │
│  │  - recv_from() │                                              │
│  └────────────────┘                                              │
└──────────────────────────────────────────────────────────────────┘
           │
┌──────────▼───────────────────────────────────────────────────────┐
│                      DATA STORAGE LAYER                          │
│  ┌────────────────┐  ┌────────────────┐  ┌──────────────────┐   │
│  │ Session Map    │  │ Image Storage  │  │ Request Cache    │   │
│  │ (In-Memory)    │  │ (In-Memory)    │  │ (In-Memory)      │   │
│  └────────────────┘  └────────────────┘  └──────────────────┘   │
│                                                                  │
│  ┌────────────────┐                                             │
│  │ encrypction_key.jpg (Disk)                                   │
│  │ Cover image for encryption                                   │
│  └────────────────┘                                             │
└──────────────────────────────────────────────────────────────────┘
```

### Node Internal Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                        CLOUD NODE                                │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  Message Receiver (Main Event Loop)                        │ │
│  │  - Listens on UDP socket                                   │ │
│  │  - Deserializes JSON messages                              │ │
│  │  - Routes to appropriate handler                           │ │
│  └────────┬───────────────────────────────────────────────────┘ │
│           │                                                      │
│  ┌────────▼──────────┬─────────────────┬──────────────────┐    │
│  │                   │                 │                  │    │
│  │  ┌─────────────┐  │  ┌───────────┐  │  ┌────────────┐ │    │
│  │  │ Session     │  │  │ Image     │  │  │ Encryption │ │    │
│  │  │ Handler     │  │  │ Handler   │  │  │ Handler    │ │    │
│  │  └──────┬──────┘  │  └─────┬─────┘  │  └──────┬─────┘ │    │
│  │         │         │        │        │         │       │    │
│  │         ▼         │        ▼        │         ▼       │    │
│  │  ┌─────────────┐  │  ┌───────────┐  │  ┌────────────┐ │    │
│  │  │SessionMgr   │  │  │ImageStore │  │  │Encryption  │ │    │
│  │  │             │  │  │           │  │  │Service     │ │    │
│  │  │sessions: {} │  │  │images: {} │  │  │            │ │    │
│  │  └─────────────┘  │  └───────────┘  │  │encrypt()   │ │    │
│  │                   │                 │  │decrypt()   │ │    │
│  └───────────────────┴─────────────────┴──────────────────┘    │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  Background Tasks                                          │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────────┐  │ │
│  │  │ Heartbeat    │  │ Heartbeat    │  │ Election       │  │ │
│  │  │ Sender       │  │ Monitor      │  │ Manager        │  │ │
│  │  │              │  │              │  │                │  │ │
│  │  │ Every 5s:    │  │ Every 5s:    │  │ On demand:     │  │ │
│  │  │ Send HB to   │  │ Check peers  │  │ Start election │  │ │
│  │  │ all peers    │  │ Update state │  │ Respond/Win    │  │ │
│  │  └──────────────┘  └──────────────┘  └────────────────┘  │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  State Management                                          │ │
│  │  ┌─────────────────────────────────────────────────────┐  │ │
│  │  │ Node State: ACTIVE | ELECTION | FAILED | RECOVERING│  │ │
│  │  │ Coordinator ID: usize                                │  │ │
│  │  │ Peers: Vec<PeerNode>                                 │  │ │
│  │  │ Load: u32 (0-100%)                                   │  │ │
│  │  │ Request Queue: Vec<Request>                          │  │ │
│  │  └─────────────────────────────────────────────────────┘  │ │
│  └────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

---

## Network Architecture

### Physical Network Topology

```
┌──────────────────────────────────────────────────────────────────┐
│                      NETWORK TOPOLOGY                            │
│                                                                  │
│                    ┌──────────────┐                             │
│                    │   Internet   │                             │
│                    └───────┬──────┘                             │
│                            │                                     │
│                    ┌───────▼──────┐                             │
│                    │    Router    │                             │
│                    │ 192.168.1.1  │                             │
│                    └───────┬──────┘                             │
│                            │                                     │
│              ┌─────────────┼─────────────┐                      │
│              │             │             │                      │
│      ┌───────▼──────┐ ┌───▼──────┐ ┌───▼──────┐               │
│      │   Switch 1   │ │ Switch 2 │ │ Switch 3 │               │
│      └───────┬──────┘ └────┬─────┘ └────┬─────┘               │
│              │             │             │                      │
│  ┌───────────┼─────────────┼─────────────┼───────────┐         │
│  │           │             │             │           │         │
│  │  ┌────────▼────────┐ ┌─▼──────────┐ ┌▼──────────┐│         │
│  │  │ Cloud Node 1    │ │Cloud Node 2│ │Cloud Node3││         │
│  │  │ 192.168.1.101   │ │192.168.1.102│ │192.168.1.103││      │
│  │  │ Port: 8001      │ │Port: 8002  │ │Port: 8003 ││         │
│  │  └─────────────────┘ └────────────┘ └───────────┘│         │
│  │                                                    │         │
│  │  CLOUD CLUSTER                                    │         │
│  └────────────────────────────────────────────────────┘         │
│                                                                  │
│  ┌────────────────────┐                                         │
│  │  Client Machines   │                                         │
│  │  192.168.1.50-99   │                                         │
│  │  (Dynamic IPs)     │                                         │
│  └────────────────────┘                                         │
│                                                                  │
│  ┌────────────────────┐                                         │
│  │  Admin Machine     │                                         │
│  │  192.168.1.10      │                                         │
│  │  (Server GUI)      │                                         │
│  └────────────────────┘                                         │
└──────────────────────────────────────────────────────────────────┘
```

### Logical Network Topology (Peer-to-Peer Mesh)

```
┌──────────────────────────────────────────────────────────────────┐
│                   P2P MESH TOPOLOGY                              │
│                                                                  │
│                       Node 1                                     │
│                   (192.168.1.101:8001)                           │
│                      /            \                              │
│                     /              \                             │
│                    /                \                            │
│         Heartbeat, Election,     Heartbeat,                     │
│         LoadQuery, Forward       Election                       │
│                  /                    \                          │
│                 /                      \                         │
│                ▼                        ▼                        │
│           Node 2                        Node 3                   │
│       (192.168.1.102:8002)      (192.168.1.103:8003)            │
│                \                        /                        │
│                 \                      /                         │
│                  \                    /                          │
│                   \                  /                           │
│              Heartbeat, Election,   /                            │
│              LoadQuery           /                               │
│                     \          /                                 │
│                      ▼        ▼                                  │
│                    Connection                                    │
│                                                                  │
│  All nodes are directly connected to each other (full mesh)     │
│  Coordinator: Node 3 (highest ID, elected via Bully)            │
│                                                                  │
│  Connection Properties:                                         │
│  - Protocol: UDP                                                │
│  - Bidirectional                                                │
│  - Unreliable (no guaranteed delivery)                          │
│  - Message-based                                                │
└──────────────────────────────────────────────────────────────────┘
```

### Communication Patterns

```
┌──────────────────────────────────────────────────────────────────┐
│                 COMMUNICATION PATTERNS                           │
└──────────────────────────────────────────────────────────────────┘

1. CLIENT → COORDINATOR PATTERN
   ────────────────────────────

   Client                Coordinator (Node 3)
     │                          │
     │  EncryptionRequest       │
     ├─────────────────────────►│
     │                          │
     │                          │ (Process or forward)
     │                          │
     │  EncryptionResponse      │
     │◄─────────────────────────┤
     │                          │


2. COORDINATOR → WORKER FORWARDING
   ──────────────────────────────

   Coordinator (Node 3)    Worker (Node 1)      Worker (Node 2)
          │                      │                      │
          │  LoadQuery           │                      │
          ├─────────────────────►│                      │
          │  LoadQuery           │                      │
          ├──────────────────────┼─────────────────────►│
          │                      │                      │
          │  LoadResponse{30%}   │                      │
          │◄─────────────────────┤                      │
          │  LoadResponse{60%}   │                      │
          │◄─────────────────────┼──────────────────────┤
          │                      │                      │
          │ Forward to Node 1    │                      │
          │ (lowest load)        │                      │
          ├─────────────────────►│                      │
          │                      │                      │


3. PEER-TO-PEER HEARTBEAT PATTERN
   ──────────────────────────────

   Node 1              Node 2              Node 3
     │                   │                   │
     │  Heartbeat        │                   │
     ├──────────────────►│                   │
     │  Heartbeat        │                   │
     ├───────────────────┼──────────────────►│
     │                   │  Heartbeat        │
     │◄──────────────────┤                   │
     │                   │  Heartbeat        │
     │                   ├──────────────────►│
     │                   │                   │
     │                   │  Heartbeat        │
     │◄──────────────────┼───────────────────┤
     │  Heartbeat        │                   │
     │                   │◄──────────────────┤
     │                   │                   │
     (Every 5 seconds, all nodes send to all peers)


4. LEADER ELECTION PATTERN (Bully Algorithm)
   ─────────────────────────────────────────

   Node 1          Node 2              Node 3
     │               │                   │
     │  ELECTION     │                   │
     ├──────────────►│                   │ (Node 1 detects coordinator failure)
     │  ELECTION     │                   │
     ├───────────────┼──────────────────►│
     │               │                   │
     │               │  ELECTION_RESPONSE│
     │◄──────────────┤                   │ (Node 2 responds - I'm alive)
     │               │                   │
     │               │  ELECTION         │
     │               ├──────────────────►│
     │               │                   │
     │               │  COORDINATOR      │ (Node 3 wins - highest ID)
     │◄──────────────┤                   │
     │               │  COORDINATOR      │
     │◄──────────────┼───────────────────┤
     │               │                   │
     (All nodes now know Node 3 is coordinator)
```

---

## Data Flow Diagrams

### Image Encryption Flow

```
┌──────────────────────────────────────────────────────────────────┐
│                 IMAGE ENCRYPTION DATA FLOW                       │
└──────────────────────────────────────────────────────────────────┘

┌─────────┐
│ Client  │
└────┬────┘
     │
     │ 1. Select vacation.jpg (127 KB)
     │
     ▼
┌─────────────────┐
│ Client GUI      │
│ Auto-compress   │ 2. Resize: 1024×768 → 200×150
│                 │    Compress: 127 KB → 9.8 KB
└────┬────────────┘
     │
     │ 3. EncryptionRequest {
     │      image_data: [9,800 bytes],
     │      usernames: ["bob"],
     │      quota: 5
     │    }
     │
     ▼
┌─────────────────────┐
│ Network (UDP)       │ 4. Serialize to JSON (11 KB)
│ Client → Node 3     │    Send via UDP
└────┬────────────────┘
     │
     ▼
┌─────────────────────┐
│ Node 3 (Coordinator)│
│ Load Check          │ 5. Calculate load: 35% < 50%
│                     │    Decision: Process locally
└────┬────────────────┘
     │
     ▼
┌────────────────────────────────────────────────────────┐
│ Encryption Service (Node 3)                            │
│                                                        │
│  1. Load original image bytes (9,800 bytes)           │
│  2. Decode to get dimensions: 200×150                 │
│  3. Load encrypction_key.jpg from disk (126 KB)      │
│  4. Decode cover image: 1248×832 RGB                  │
│  5. Prepare metadata:                                 │
│     JSON: {"usernames":["bob"],"quota":5} (49 bytes)  │
│  6. Calculate embedding:                              │
│     - 4 bytes: metadata length                        │
│     - 49 bytes: metadata JSON                         │
│     - 4 bytes: image length                           │
│     - 9,800 bytes: original image                     │
│     Total: 9,857 bytes                                │
│  7. Embed in LSBs of cover image:                     │
│     For each byte to embed:                           │
│       For each of 8 bits:                             │
│         Modify LSB of next pixel byte                 │
│     Total pixels modified: 9,857 × 8 = 78,856        │
│  8. Encode as PNG (lossless)                          │
│     Result: 1,098,063 bytes (1.1 MB)                  │
│                                                        │
│  Duration: 54 ms                                      │
└────┬───────────────────────────────────────────────────┘
     │
     │ 9. Encrypted PNG (1.1 MB)
     │
     ▼
┌─────────────────────┐
│ Network (UDP)       │ 10. Too large for single packet!
│ Chunking Required   │     Fragment into 25 chunks (45 KB each)
└────┬────────────────┘
     │
     │ 11. Send chunks with 2ms delay
     │     Chunk 1/25 (45 KB)
     │     Chunk 2/25 (45 KB)
     │     ...
     │     Chunk 25/25 (10 KB)
     │
     ▼
┌─────────────────────┐
│ Client              │ 12. Receive chunks
│ Reassembler         │     Reassemble to 1.1 MB
└────┬────────────────┘
     │
     ▼
┌─────────────────────┐
│ Client GUI          │ 13. Display success
│                     │     ✅ Encrypted: 1,100,000 bytes
│                     │     Duration: 54ms
│                     │     [Save] [Send to Users]
└─────────────────────┘
```

### Image Viewing Flow

```
┌──────────────────────────────────────────────────────────────────┐
│                   IMAGE VIEWING DATA FLOW                        │
└──────────────────────────────────────────────────────────────────┘

┌─────────┐
│ Bob     │
│ (Client)│
└────┬────┘
     │
     │ 1. Click "Received Images" → "Refresh"
     │
     ▼
┌─────────────────┐
│ Client GUI      │ 2. QueryReceivedImages { username: "bob" }
└────┬────────────┘
     │
     ▼
┌─────────────────────┐
│ Network (Multicast  │ 3. Send query to all nodes
│ to all nodes)       │    (try to find image)
└────┬────────────────┘
     │
     ├──────► Node 1: Check local storage → No images for "bob"
     ├──────► Node 2: Check local storage → Found 1 image! ✓
     └──────► Node 3: Check local storage → No images for "bob"
     │
     ▼
┌─────────────────────┐
│ Node 2 Response     │ 4. QueryReceivedImagesResponse {
│                     │      images: [
│                     │        { image_id: "...", from: "alice",
│                     │          remaining_views: 3 }
│                     │      ]
│                     │    }
└────┬────────────────┘
     │
     ▼
┌─────────────────────┐
│ Client GUI          │ 5. Display:
│                     │    From: alice
│                     │    Remaining: 3 views
│                     │    [View] button
└────┬────────────────┘
     │
     │ 6. Bob clicks [View]
     │
     ▼
┌─────────────────────┐
│ Client              │ 7. ViewImage {
│                     │      username: "bob",
│                     │      image_id: "..."
│                     │    }
└────┬────────────────┘
     │
     ▼
┌──────────────────────────────────────────────────────────┐
│ Node 2 - Image Retrieval & Decryption                   │
│                                                          │
│  1. Lookup image for bob: FOUND                         │
│  2. Check remaining_views: 3 > 0 ✓                      │
│  3. Retrieve encrypted PNG (1.1 MB)                     │
│  4. Call decrypt_image():                               │
│     a) Load PNG into memory                             │
│     b) Extract LSBs from pixel bytes:                   │
│        - Read 4 bytes (metadata length): 49             │
│        - Read 49 bytes (metadata JSON)                  │
│          Parse: {"usernames":["bob"],"quota":5}         │
│        - Read 4 bytes (image length): 9,800             │
│        - Read 9,800 bytes (original JPEG)               │
│     c) Duration: 28 ms                                  │
│  5. Decrement remaining_views: 3 → 2                    │
│  6. Update image record (keep, still has views)         │
│  7. Return decrypted data                               │
└────┬─────────────────────────────────────────────────────┘
     │
     │ 8. ViewImageResponse {
     │      image_data: [9,800 bytes] ← ORIGINAL JPEG
     │      remaining_views: 2
     │    }
     │
     ▼
┌─────────────────────┐
│ Network (Chunking)  │ 9. Fragment if needed
│                     │    (9.8 KB fits in 1 packet ✓)
└────┬────────────────┘
     │
     ▼
┌─────────────────────┐
│ Client GUI          │ 10. Decode image bytes
│                     │     Load JPEG (200×150)
│                     │     Display in preview
│                     │
│  ┌───────────────┐  │ 11. Bob sees ORIGINAL vacation photo!
│  │  [Image]      │  │     (Not the encryption key)
│  │  vacation.jpg │  │
│  │  200×150      │  │     Remaining views: 2
│  └───────────────┘  │
└─────────────────────┘
```

### Session Registration Flow

```
┌──────────────────────────────────────────────────────────────────┐
│               SESSION REGISTRATION DATA FLOW                     │
└──────────────────────────────────────────────────────────────────┘

┌─────────┐
│ Alice   │
└────┬────┘
     │
     │ 1. Start client GUI
     │
     ▼
┌─────────────────┐
│ Login Screen    │ 2. Enter username: "alice"
│                 │    Click [Login]
└────┬────────────┘
     │
     │ 3. SessionRegister {
     │      client_id: "alice_12345",
     │      username: "alice"
     │    }
     │
     ▼
┌─────────────────────┐
│ Network             │ 4. Try each node until success
│ (Multicast to nodes)│
└────┬────────────────┘
     │
     ├──────► Try Node 1
     │
     ▼
┌──────────────────────────────────────────────────────────┐
│ Node 1 - Session Check                                   │
│                                                          │
│  1. Receive SessionRegister                             │
│  2. Lock session manager (exclusive write)              │
│  3. Check if "alice" exists in sessions map             │
│     sessions.get("alice") → None (available!)           │
│  4. Insert into sessions:                               │
│     sessions.insert("alice", "alice_12345")             │
│  5. Unlock session manager                              │
│  6. Create response                                     │
└────┬─────────────────────────────────────────────────────┘
     │
     │ 7. SessionRegisterResponse {
     │      success: true,
     │      error: None
     │    }
     │
     ▼
┌─────────────────────┐
│ Network             │
└────┬────────────────┘
     │
     ▼
┌─────────────────────┐
│ Client GUI          │ 8. Login successful!
│                     │    Show main interface
│  ┌───────────────┐  │    Username: alice ✓
│  │ Upload Tab    │  │
│  │ Received Tab  │  │
│  │ History Tab   │  │
│  └───────────────┘  │
└─────────────────────┘

CONCURRENT REGISTRATION (Conflict Detection):
──────────────────────────────────────────────

If Bob tries to register as "alice" at the same time:

Bob → Node 2: SessionRegister { username: "alice" }
                 │
                 ▼
             Node 2: Check sessions.get("alice")
                     → None (available)
                     Insert "alice" → "bob_67890"
                     Response: success: true

Alice → Node 1: SessionRegister { username: "alice" }
                    │
                    ▼
                Node 1: Check sessions.get("alice")
                        → None (available)
                        Insert "alice" → "alice_12345"
                        Response: success: true

PROBLEM: Username registered on both nodes!
└─ No cross-node validation (known limitation)
```

---

## Deployment Architecture

### Single-Machine Deployment (Development)

```
┌──────────────────────────────────────────────────────────────────┐
│                    LOCALHOST DEPLOYMENT                          │
│                   (Single Machine - Dev/Test)                    │
│                                                                  │
│  Machine: Ubuntu 22.04, 8 GB RAM, 4 CPU cores                   │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  Process 1: Cloud Node 1                                   │ │
│  │  Command: ./cloud-node 1 8001                              │ │
│  │  PID: 12345                                                │ │
│  │  Port: 8001 (UDP)                                          │ │
│  │  Memory: ~50 MB                                            │ │
│  │  Peers: 127.0.0.1:8002, 127.0.0.1:8003                     │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  Process 2: Cloud Node 2                                   │ │
│  │  Command: ./cloud-node 2 8002                              │ │
│  │  PID: 12346                                                │ │
│  │  Port: 8002 (UDP)                                          │ │
│  │  Memory: ~50 MB                                            │ │
│  │  Peers: 127.0.0.1:8001, 127.0.0.1:8003                     │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  Process 3: Cloud Node 3                                   │ │
│  │  Command: ./cloud-node 3 8003                              │ │
│  │  PID: 12347                                                │ │
│  │  Port: 8003 (UDP)                                          │ │
│  │  Memory: ~50 MB                                            │ │
│  │  Peers: 127.0.0.1:8001, 127.0.0.1:8002                     │ │
│  │  Status: COORDINATOR ★                                     │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  Process 4: Server GUI                                     │ │
│  │  Command: ./server-gui 127.0.0.1:8001 ...                 │ │
│  │  PID: 12348                                                │ │
│  │  Display: :0 (X11)                                         │ │
│  │  Memory: ~100 MB                                           │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  Process 5+: Client GUIs                                   │ │
│  │  Command: ./client-gui alice                               │ │
│  │  PID: 12349, 12350, ...                                    │ │
│  │  Memory: ~80 MB each                                       │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  Total Resource Usage:                                         │
│  - CPU: 10-30% (idle), up to 80% (under load)                 │
│  - RAM: ~400 MB total                                          │
│  - Disk: encrypction_key.jpg (126 KB)                         │
│  - Network: Loopback (no external network)                    │
└──────────────────────────────────────────────────────────────────┘
```

### Multi-Machine Deployment (Production)

```
┌──────────────────────────────────────────────────────────────────┐
│                  PRODUCTION DEPLOYMENT                           │
│                  (3 Physical Machines)                           │
└──────────────────────────────────────────────────────────────────┘

┌────────────────────────┐  ┌────────────────────────┐  ┌────────────────────────┐
│ Machine 1              │  │ Machine 2              │  │ Machine 3              │
│ server1.example.com    │  │ server2.example.com    │  │ server3.example.com    │
│ 192.168.1.101          │  │ 192.168.1.102          │  │ 192.168.1.103          │
├────────────────────────┤  ├────────────────────────┤  ├────────────────────────┤
│ OS: Ubuntu 22.04 LTS   │  │ OS: Ubuntu 22.04 LTS   │  │ OS: Ubuntu 22.04 LTS   │
│ RAM: 8 GB              │  │ RAM: 8 GB              │  │ RAM: 8 GB              │
│ CPU: 4 cores           │  │ CPU: 4 cores           │  │ CPU: 4 cores           │
│ Disk: 100 GB SSD       │  │ Disk: 100 GB SSD       │  │ Disk: 100 GB SSD       │
├────────────────────────┤  ├────────────────────────┤  ├────────────────────────┤
│ Process:               │  │ Process:               │  │ Process:               │
│ ./cloud-node 1 8001    │  │ ./cloud-node 2 8002    │  │ ./cloud-node 3 8003    │
│                        │  │                        │  │                        │
│ Peers:                 │  │ Peers:                 │  │ Peers:                 │
│ - 192.168.1.102:8002   │  │ - 192.168.1.101:8001   │  │ - 192.168.1.101:8001   │
│ - 192.168.1.103:8003   │  │ - 192.168.1.103:8003   │  │ - 192.168.1.102:8002   │
│                        │  │                        │  │                        │
│ Firewall:              │  │ Firewall:              │  │ Firewall:              │
│ Allow UDP 8001         │  │ Allow UDP 8002         │  │ Allow UDP 8003         │
│ From: 192.168.1.0/24   │  │ From: 192.168.1.0/24   │  │ From: 192.168.1.0/24   │
│                        │  │                        │  │                        │
│ Systemd Service:       │  │ Systemd Service:       │  │ Systemd Service:       │
│ cloud-node.service     │  │ cloud-node.service     │  │ cloud-node.service     │
│ - Auto-restart on fail │  │ - Auto-restart on fail │  │ - Auto-restart on fail │
│ - Start on boot        │  │ - Start on boot        │  │ - Start on boot        │
└────────────────────────┘  └────────────────────────┘  └────────────────────────┘

┌────────────────────────────────────────────────────────────────┐
│ Admin/Monitoring Machine                                       │
│ admin.example.com (192.168.1.10)                               │
├────────────────────────────────────────────────────────────────┤
│ ./server-gui 192.168.1.101:8001 \                             │
│              192.168.1.102:8002 \                             │
│              192.168.1.103:8003                               │
└────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────┐
│ Client Machines                                                │
│ (Workstations, Laptops)                                        │
│ 192.168.1.50-99                                                │
├────────────────────────────────────────────────────────────────┤
│ ./client-gui <username> 192.168.1.101:8001 \                  │
│                          192.168.1.102:8002 \                  │
│                          192.168.1.103:8003                    │
└────────────────────────────────────────────────────────────────┘
```

---

## Security Architecture

### Encryption Security Model

```
┌──────────────────────────────────────────────────────────────────┐
│                  ENCRYPTION SECURITY MODEL                       │
└──────────────────────────────────────────────────────────────────┘

SECRET KEY (Shared):
┌────────────────────────┐
│ encrypction_key.jpg    │ ← Distributed to all nodes
│ (1248×832 PNG)         │   Must be kept secret!
│ 126 KB                 │   If leaked, all images can be decrypted
└────────────────────────┘

ENCRYPTION (Steganography):
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  Original Image              Encryption Key                    │
│  ┌──────────┐               ┌─────────────────┐               │
│  │          │               │                 │               │
│  │ Vacation │               │ Cover Pattern   │               │
│  │ Photo    │   +   LSB     │                 │               │
│  │          │   Embedding   │ (Key Image)     │               │
│  └──────────┘               └─────────────────┘               │
│                                      │                          │
│                                      ▼                          │
│                             ┌─────────────────┐                │
│                             │ Encrypted PNG   │                │
│                             │ Looks like key! │                │
│                             │                 │                │
│                             │ (Imperceptible  │                │
│                             │  LSB changes)   │                │
│                             └─────────────────┘                │
└─────────────────────────────────────────────────────────────────┘

SECURITY PROPERTIES:
✓ Confidentiality: Image content hidden in LSBs
✓ Stealth: Encrypted image looks like innocent cover
✗ Authentication: No signature verification
✗ Integrity: No tampering detection (no HMAC/signature)
✗ Key Distribution: Manual (must copy encrypction_key.jpg to all nodes)
✗ Forward Secrecy: Single key for all images
✗ Access Control Enforcement: Relies on correct node implementation

THREAT MODEL:

Threats Mitigated:
✓ Casual observation (image looks like cover)
✓ Unauthorized viewing (quota enforcement)
✓ Network sniffing visual content (encrypted in transit)

Threats NOT Mitigated:
✗ Attacker with encryption key (can decrypt all images)
✗ Compromised node (has key, can decrypt)
✗ Man-in-the-middle (no authentication)
✗ Replay attacks (no nonce/timestamp validation)
✗ Image tampering (no integrity check)
```

### Access Control Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                    ACCESS CONTROL MODEL                          │
└──────────────────────────────────────────────────────────────────┘

AUTHENTICATION: Username-based (No password!)
┌────────────────────────────────────────────┐
│ User                                       │
│  │                                         │
│  └─► Username: "alice" ──────► Register   │
│                                   │        │
│                                   ▼        │
│                          ┌────────────────┐│
│                          │ Session Map    ││
│                          │ {              ││
│                          │  "alice": "c1" ││
│                          │  "bob": "c2"   ││
│                          │ }              ││
│                          └────────────────┘│
└────────────────────────────────────────────┘

Security Issue: No password validation!
Anyone can register as any username (if not taken)

AUTHORIZATION: Sender-specified whitelist
┌────────────────────────────────────────────┐
│ Alice sends image to:                      │
│  ☑ bob                                     │
│  ☑ charlie                                 │
│                                            │
│ Metadata embedded:                         │
│ { "usernames": ["bob", "charlie"] }        │
│                                            │
│ Enforcement:                               │
│  - Node stores image for bob & charlie     │
│  - Only bob and charlie can view           │
│  - Dave's ViewImage request → DENIED       │
└────────────────────────────────────────────┘

QUOTA ENFORCEMENT:
┌────────────────────────────────────────────┐
│ Image Record:                              │
│ {                                          │
│   to_username: "bob",                      │
│   max_views: 5,                            │
│   remaining_views: 3  ← Decremented        │
│ }                                          │
│                                            │
│ View Operations:                           │
│  View 1: remaining_views = 4               │
│  View 2: remaining_views = 3               │
│  View 3: remaining_views = 2               │
│  View 4: remaining_views = 1               │
│  View 5: remaining_views = 0 → DELETE      │
│  View 6: Image not found → DENIED          │
└────────────────────────────────────────────┘
```

---

**Last Updated**: 2025-11-23
**Version**: 1.0
