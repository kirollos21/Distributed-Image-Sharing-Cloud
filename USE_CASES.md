# Use Cases - Distributed Image Cloud

## Overview

This document provides detailed use-case scenarios for the Distributed Image Cloud system, covering typical user interactions, system behaviors, and edge cases.

---

## Table of Contents

1. [User Scenarios](#user-scenarios)
2. [System Operations](#system-operations)
3. [Failure Scenarios](#failure-scenarios)
4. [Security Scenarios](#security-scenarios)
5. [Performance Scenarios](#performance-scenarios)
6. [Edge Cases](#edge-cases)

---

## User Scenarios

### Use Case 1: Alice Shares Private Photo with Bob

**Actors**: Alice (sender), Bob (recipient)
**Preconditions**: Both Alice and Bob have the client GUI installed
**Goal**: Alice wants to share a private photo with Bob, allowing him to view it 3 times

#### Detailed Flow

```
┌──────────────────────────────────────────────────────────────┐
│ Step-by-Step Scenario                                       │
└──────────────────────────────────────────────────────────────┘

1. SYSTEM SETUP
   ──────────────
   Administrator starts the distributed cloud:

   Terminal 1: $ ./start_gui_system.sh

   Output:
   ✓ Node 1 started (PID: 12345, Port: 8001)
   ✓ Node 2 started (PID: 12346, Port: 8002)
   ✓ Node 3 started (PID: 12347, Port: 8003)
   ✓ Server GUI started (PID: 12348)

   Logs show:
   [Node 3] Elected as coordinator
   [All Nodes] Cluster ready

2. ALICE REGISTERS
   ────────────────
   Terminal 2: $ RUST_LOG=info target/release/client-gui alice

   Alice's GUI opens:
   ┌─────────────────────────────────────────┐
   │ Distributed Image Cloud - Client       │
   ├─────────────────────────────────────────┤
   │ Welcome! Please enter your username.    │
   │                                         │
   │ Username: [alice          ]             │
   │           [Login]                       │
   └─────────────────────────────────────────┘

   Alice types "alice" and clicks Login

   Network Activity:
   alice → Node 1: SessionRegister { client_id: "alice", username: "alice" }
   Node 1 → alice: SessionRegisterResponse { success: true }

   GUI Updates:
   ✓ Login successful
   ✓ Main interface displayed

3. BOB REGISTERS
   ─────────────
   Terminal 3: $ RUST_LOG=info target/release/client-gui bob

   Bob logs in with username "bob"

   Network Activity:
   bob → Node 2: SessionRegister { client_id: "bob", username: "bob" }
   Node 2 → bob: SessionRegisterResponse { success: true }

4. ALICE SELECTS IMAGE
   ───────────────────
   Alice clicks "Upload" tab
   Alice clicks "Choose Image File"

   File dialog opens:
   Alice navigates to ~/Pictures/vacation.jpg
   Alice selects file

   GUI Updates:
   ┌─────────────────────────────────────────┐
   │ 1. Select Image                         │
   │ Selected: vacation.jpg (127 KB)         │
   │ ⚠️ Large image - will be compressed    │
   │                                         │
   │ Preview:                                │
   │ ┌───────────┐                          │
   │ │  [Image]  │                          │
   │ └───────────┘                          │
   └─────────────────────────────────────────┘

5. ALICE ADDS BOB AS AUTHORIZED USER
   ──────────────────────────────────
   Alice scrolls to "Configure Encryption"
   Alice types "bob" in username field
   Alice clicks "➕ Add User"

   Network Activity:
   alice → Node 1: CheckUsernameAvailable { username: "bob" }
   Node 1 → alice: CheckUsernameResponse { available: false }
   (false = user exists = good!)

   GUI Updates:
   ✓ Bob added to authorized users list
   ☑ bob  (checkbox appears, Alice checks it)

6. ALICE SETS VIEWING QUOTA
   ────────────────────────
   Alice adjusts quota counter:
   Viewing Quota: [◀] [3] [▶] views

7. ALICE ENCRYPTS IMAGE
   ────────────────────
   Alice clicks "🚀 Encrypt Image"

   GUI shows:
   ┌─────────────────────────────────────────┐
   │ Request Status                          │
   │ ⏳ Processing encryption request...     │
   └─────────────────────────────────────────┘

   Network Activity:
   a) Image compression:
      - Original: 127 KB → Resized/compressed: 9.8 KB

   b) Encryption request:
      alice → Node 3 (coordinator): EncryptionRequest {
          request_id: "req_alice_1234567890",
          client_username: "alice",
          image_data: [9,800 bytes],
          usernames: ["bob"],
          quota: 3,
          forwarded: false,
      }

   c) Load balancing:
      Node 3: "My load is 20%, I'll process locally"

   d) Encryption (on Node 3):
      - Load encrypction_key.jpg (1248×832, 126 KB)
      - Serialize metadata: { "usernames": ["bob"], "quota": 3 }
      - Embed in LSBs: 49 bytes metadata + 9,800 bytes image
      - Generate encrypted PNG: 1.1 MB
      - Duration: 54 ms

   e) Response:
      Node 3 → alice: EncryptionResponse {
          request_id: "req_alice_1234567890",
          encrypted_image: [1,100,000 bytes],
          success: true,
      }

   GUI Updates:
   ┌─────────────────────────────────────────┐
   │ Request Status                          │
   │ ✅ Success! Request ID: req_alice_...   │
   │ Duration: 54ms                          │
   │ Encrypted data size: 1,100,000 bytes    │
   │                                         │
   │ [Save Encrypted Image]                  │
   │ [📤 Send to Selected Users]            │
   └─────────────────────────────────────────┘

8. ALICE SENDS TO BOB
   ──────────────────
   Alice clicks "📤 Send to Selected Users"

   Network Activity:
   alice → Node 2: SendImage {
       from_username: "alice",
       to_usernames: ["bob"],
       encrypted_image: [1,100,000 bytes via chunks],
       max_views: 3,
       image_id: "req_alice_1234567890",
   }

   Node 2 Processing:
   - Stores image for bob in local storage
   - Initializes view counter: remaining_views = 3

   Node 2 → alice: SendImageResponse { success: true }

   GUI Updates:
   ✅ Image sent successfully to bob!

9. BOB RECEIVES NOTIFICATION
   ─────────────────────────
   Bob clicks "Received Images" tab
   Bob clicks "🔄 Refresh"

   Network Activity:
   bob → Node 1: QueryReceivedImages { username: "bob" }
   Node 1 checks local storage: No images for bob

   bob → Node 2: QueryReceivedImages { username: "bob" }
   Node 2 checks local storage: Found 1 image!

   Node 2 → bob: QueryReceivedImagesResponse {
       images: [
           ReceivedImageInfo {
               image_id: "req_alice_1234567890",
               from_username: "alice",
               remaining_views: 3,
           }
       ]
   }

   GUI Updates:
   ┌─────────────────────────────────────────┐
   │ Received Images                         │
   │                                         │
   │ ┌─────────────────────────────────────┐ │
   │ │ From: alice                         │ │
   │ │ ID: req_alice_1234567890            │ │
   │ │ Remaining views: 3                  │ │
   │ │                          [👁 View] │ │
   │ └─────────────────────────────────────┘ │
   └─────────────────────────────────────────┘

10. BOB VIEWS IMAGE (1st time)
    ─────────────────────────
    Bob clicks "👁 View"

    Network Activity:
    bob → Node 2: ViewImage {
        username: "bob",
        image_id: "req_alice_1234567890",
    }

    Node 2 Processing:
    a) Retrieve encrypted image (1.1 MB PNG)
    b) Call decrypt_image():
       - Extract LSBs from PNG
       - Reconstruct: 49 bytes metadata + 9,800 bytes image
       - Parse metadata: { "usernames": ["bob"], "quota": 3 }
       - Duration: 28 ms
    c) Decrement view counter: remaining_views = 2
    d) Keep image (still has views left)

    Node 2 → bob: ViewImageResponse {
        image_data: [9,800 bytes - ORIGINAL JPEG],
        remaining_views: 2,
        success: true,
    }

    GUI Updates:
    ┌─────────────────────────────────────────┐
    │ Viewing Image                           │
    │ Remaining views: 2                      │
    │                                         │
    │ ┌───────────────────────────┐          │
    │ │                           │          │
    │ │   [vacation.jpg shown]    │          │
    │ │   (ORIGINAL, not cover)   │          │
    │ │                           │          │
    │ └───────────────────────────┘          │
    │                                         │
    │ [Close]                                 │
    └─────────────────────────────────────────┘

    Bob sees the original vacation photo!

11. BOB VIEWS IMAGE (2nd time)
    ─────────────────────────
    Bob closes and clicks "👁 View" again
    remaining_views: 2 → 1

12. BOB VIEWS IMAGE (3rd time)
    ─────────────────────────
    Bob clicks "👁 View" one more time

    Node 2 Processing:
    - Decrement: remaining_views = 0
    - **DELETE IMAGE** (quota exhausted)

    Node 2 → bob: ViewImageResponse {
        image_data: [9,800 bytes],
        remaining_views: 0,
        success: true,
    }

    Bob views the image for the last time

13. BOB TRIES TO VIEW AGAIN (4th time - DENIED)
    ──────────────────────────────────────────
    Bob refreshes, sees no images

    Network Activity:
    bob → Node 2: QueryReceivedImages { username: "bob" }
    Node 2 → bob: QueryReceivedImagesResponse { images: [] }

    GUI shows: "No images received"

    Image is permanently deleted!
```

**Postconditions**:
- Alice successfully shared image with Bob
- Bob viewed it exactly 3 times
- Image is now deleted from all nodes
- Both users remain logged in

---

### Use Case 2: Charlie Sends Image to Multiple Users

**Actors**: Charlie (sender), Alice, Bob, Dave (recipients)
**Preconditions**: All users registered
**Goal**: Charlie shares a document with 3 colleagues, allowing 10 views each

#### Flow

```
1. Charlie selects document.png (15 KB)
   - GUI auto-compresses to 9.5 KB

2. Charlie adds authorized users:
   ☑ alice
   ☑ bob
   ☑ dave

3. Charlie sets quota: 10 views

4. Charlie encrypts:
   - Sends EncryptionRequest to coordinator
   - Receives encrypted PNG (1.1 MB)

5. Charlie sends to users:
   Network Activity:
   charlie → Node 1: SendImage {
       to_usernames: ["alice", "bob", "dave"],
       max_views: 10,
   }

   Node 1 creates 3 separate image records:
   - ReceivedImage for alice (remaining_views: 10)
   - ReceivedImage for bob (remaining_views: 10)
   - ReceivedImage for dave (remaining_views: 10)

6. Each recipient views independently:
   - Alice views 5 times → remaining: 5
   - Bob views 10 times → deleted
   - Dave never views → remaining: 10

7. Result:
   - Alice can still view 5 more times
   - Bob's copy is deleted
   - Dave's copy intact (can still view 10 times)
```

**Key Points**:
- Each recipient has independent view counter
- One user exhausting quota doesn't affect others
- Unused quotas persist indefinitely

---

### Use Case 3: Emergency Room - Medical Image Sharing

**Scenario**: Hospital emergency room needs to share X-ray images with specialists

**Actors**:
- Dr. Smith (emergency room doctor)
- Dr. Jones (radiologist)
- Dr. Brown (surgeon)

**Requirements**:
- Urgent image sharing
- Limited viewing for privacy
- Secure encryption

#### Flow

```
1. EMERGENCY ADMISSION
   Patient arrives with chest trauma
   X-ray taken: chest_xray_patient_1234.jpg (85 KB)

2. DR. SMITH UPLOADS X-RAY
   - Logs in as "dr_smith"
   - Selects chest_xray_patient_1234.jpg
   - Adds authorized users:
     ☑ dr_jones (radiologist)
     ☑ dr_brown (surgeon)
   - Sets quota: 5 views (for review & consultation)

3. ENCRYPTION
   - System compresses X-ray: 85 KB → 9.8 KB
   - Encrypts using hospital's encryption key image
   - Encrypted image looks like hospital logo (1.1 MB)

4. DISTRIBUTION
   - Dr. Jones receives notification on tablet
   - Dr. Brown receives notification in OR prep room

5. DR. JONES REVIEWS (RADIOLOGIST)
   - Views X-ray 2 times (initial review + detailed analysis)
   - Remaining views: 3
   - Provides diagnosis via separate system

6. DR. BROWN REVIEWS (SURGEON)
   - Views X-ray 3 times (surgical planning)
   - Remaining views: 2
   - Plans surgical approach

7. POST-SURGERY FOLLOW-UP
   - Dr. Smith views again (1 time)
   - Remaining: 1 view
   - Decides no further viewing needed

8. AUTOMATIC DELETION
   - After final view, image auto-deletes
   - HIPAA compliance: image not stored indefinitely
   - Audit log shows: 6 total views (under quota of 5 per user)
```

**Benefits**:
- Secure image transmission (encrypted)
- Privacy protection (auto-deletion)
- Access control (only authorized doctors)
- Audit trail (view counts logged)

---

## System Operations

### Use Case 4: System Startup and Initialization

**Actor**: System Administrator
**Goal**: Start distributed cloud cluster

#### Flow

```
1. START NODES IN SEQUENCE
   ───────────────────────

   Terminal 1:
   $ RUST_LOG=info target/release/cloud-node 1 8001

   [Node 1] Binding to 0.0.0.0:8001
   [Node 1] Initializing with 2 peers
   [Node 1] Peer 1: 127.0.0.1:8002
   [Node 1] Peer 2: 127.0.0.1:8003
   [Node 1] Starting heartbeat sender
   [Node 1] Starting heartbeat monitor
   [Node 1] Waiting for peers...
   [Node 1] Starting election
   [Node 1] Sending ELECTION to higher nodes: [2, 3]
   [Node 1] Waiting for election responses (2s timeout)...

   Terminal 2 (5 seconds later):
   $ RUST_LOG=info target/release/cloud-node 2 8002

   [Node 2] Binding to 0.0.0.0:8002
   [Node 2] Initializing with 2 peers
   [Node 2] Starting election
   [Node 2] Received ELECTION from Node 1
   [Node 2] Responding to Node 1 (I'm higher)
   [Node 2] Sending ELECTION to Node 3
   [Node 2] Waiting for responses...

   Terminal 3 (5 seconds later):
   $ RUST_LOG=info target/release/cloud-node 3 8003

   [Node 3] Binding to 0.0.0.0:8003
   [Node 3] Initializing with 2 peers
   [Node 3] Starting election
   [Node 3] Received ELECTION from Nodes 1, 2
   [Node 3] I'm the highest ID node!
   [Node 3] Declaring self as COORDINATOR
   [Node 3] Broadcasting COORDINATOR message

   All Nodes:
   [Node 1] Coordinator elected: Node 3
   [Node 2] Coordinator elected: Node 3
   [Node 3] I am the coordinator

   [All Nodes] State: ACTIVE
   [All Nodes] Cluster ready!

2. START MONITORING GUI
   ────────────────────

   Terminal 4:
   $ target/release/server-gui 127.0.0.1:8001 127.0.0.1:8002 127.0.0.1:8003

   [Server GUI] Connected to 3 nodes
   [Server GUI] Refreshing metrics...

   GUI shows:
   ┌────────────────────────────────────┐
   │ Cluster Status                     │
   ├────────────────────────────────────┤
   │ Node 1: ACTIVE     Load: 0%       │
   │ Node 2: ACTIVE     Load: 0%       │
   │ Node 3: ACTIVE ★   Load: 0%       │
   │                                    │
   │ ★ Coordinator: Node 3              │
   │ Cluster health: HEALTHY            │
   └────────────────────────────────────┘

3. VERIFY CLUSTER HEALTH
   ──────────────────────

   $ ps aux | grep cloud-node
   user  12345  cloud-node 1 8001
   user  12346  cloud-node 2 8002
   user  12347  cloud-node 3 8003

   $ netstat -tulpn | grep -E "800[1-3]"
   udp  0.0.0.0:8001  LISTEN  12345/cloud-node
   udp  0.0.0.0:8002  LISTEN  12346/cloud-node
   udp  0.0.0.0:8003  LISTEN  12347/cloud-node

   All nodes operational!
```

---

### Use Case 5: Dynamic Load Balancing

**Scenario**: High traffic causes load imbalance
**Goal**: System automatically distributes load

#### Flow

```
Initial State:
  Node 1: Load =  5%
  Node 2: Load = 10%
  Node 3: Load = 15% (Coordinator)

10 Encryption Requests Arrive Simultaneously
─────────────────────────────────────────────

Request 1 → Node 3 (Coordinator)
  Node 3: My load is 15% < 50%, I'll process
  Node 3: Load → 25%
  ✓ Processed locally

Request 2 → Node 3
  Node 3: My load is 25% < 50%, I'll process
  Node 3: Load → 35%
  ✓ Processed locally

Request 3 → Node 3
  Node 3: My load is 35% < 50%, I'll process
  Node 3: Load → 45%
  ✓ Processed locally

Request 4 → Node 3
  Node 3: My load is 45% < 50%, I'll process
  Node 3: Load → 55%
  ✓ Processed locally

Request 5 → Node 3
  Node 3: My load is 55% >= 50%, FORWARD!
  Node 3: Querying peers...
    Node 1: LoadResponse { load: 5% }
    Node 2: LoadResponse { load: 10% }
  Node 3: Forwarding to Node 1 (lowest at 5%)
  Node 1: Load → 15%
  ✓ Processed by Node 1

Request 6 → Node 3
  Node 3: Load 55% >= 50%, FORWARD!
  Node 3: Querying peers...
    Node 1: LoadResponse { load: 15% }
    Node 2: LoadResponse { load: 10% }
  Node 3: Forwarding to Node 2 (lowest at 10%)
  Node 2: Load → 20%
  ✓ Processed by Node 2

Request 7-10 → Node 3
  Similar forwarding logic...

Final State:
  Node 1: Load = 35% (processed 4 requests)
  Node 2: Load = 40% (processed 3 requests)
  Node 3: Load = 65% (processed 4 requests, coordinated all)

Load Distribution:
  Node 1: 36%
  Node 2: 27%
  Node 3: 36%
  ✓ Balanced!
```

**System Behavior**:
- Automatic load detection
- Dynamic request forwarding
- No manual intervention required
- Even distribution achieved

---

## Failure Scenarios

### Use Case 6: Coordinator Node Crashes During Operation

**Scenario**: Node 3 (coordinator) crashes unexpectedly
**Impact**: System must elect new coordinator
**Goal**: Maintain service availability

#### Detailed Timeline

```
Time: 00:00 - Normal Operation
────────────────────────────────
  Node 1: ACTIVE, Load = 20%
  Node 2: ACTIVE, Load = 30%
  Node 3: ACTIVE, Load = 40%, COORDINATOR ★

  Heartbeats flowing normally:
  00:00: Node 3 → All peers: Heartbeat
  00:05: Node 3 → All peers: Heartbeat
  00:10: Node 3 → All peers: Heartbeat

Time: 00:12 - COORDINATOR CRASH
────────────────────────────────
  ** Node 3 power failure **

  [Node 3] OFFLINE
  [Node 1] Last heartbeat from Node 3: 00:10
  [Node 2] Last heartbeat from Node 3: 00:10

Time: 00:15 - Heartbeat Expected
─────────────────────────────────
  [Node 1] Expecting heartbeat from Node 3... none received
  [Node 2] Expecting heartbeat from Node 3... none received

  (Still within timeout, wait...)

Time: 00:20 - Second Missed Heartbeat
──────────────────────────────────────
  [Node 1] Two heartbeats missed from Node 3
  [Node 2] Two heartbeats missed from Node 3

  (Still within 15s timeout...)

Time: 00:25 - FAILURE DETECTED
───────────────────────────────
  [Node 1] 15 seconds since last heartbeat from Node 3!
  [Node 1] Marking Node 3 as FAILED
  [Node 1] Node 3 was coordinator → TRIGGER ELECTION

  [Node 2] 15 seconds since last heartbeat from Node 3!
  [Node 2] Marking Node 3 as FAILED
  [Node 2] Node 3 was coordinator → TRIGGER ELECTION

Time: 00:25.100 - ELECTION PHASE 1
───────────────────────────────────
  [Node 1] Starting election
  [Node 1] State: ACTIVE → ELECTION
  [Node 1] Sending ELECTION to higher nodes: [2]

  [Node 2] Starting election (parallel)
  [Node 2] State: ACTIVE → ELECTION
  [Node 2] Sending ELECTION to higher nodes: []
  [Node 2] No higher nodes! I'm the highest!

Time: 00:25.200 - ELECTION PHASE 2
───────────────────────────────────
  [Node 2] Declaring self as COORDINATOR
  [Node 2] Broadcasting COORDINATOR message to all

  [Node 1] Received ELECTION from Node 2... wait, that's wrong!
  [Node 1] Actually, received COORDINATOR from Node 2
  [Node 1] Accepting Node 2 as coordinator
  [Node 1] State: ELECTION → ACTIVE
  [Node 1] coordinator_id = 2

  [Node 2] State: ELECTION → ACTIVE
  [Node 2] I am coordinator
  [Node 2] coordinator_id = 2 (self)

Time: 00:26 - NEW COORDINATOR ESTABLISHED
──────────────────────────────────────────
  Node 1: ACTIVE, coordinator_id = 2
  Node 2: ACTIVE, COORDINATOR ★, coordinator_id = 2
  Node 3: OFFLINE

  Cluster operational with 2 nodes!
  Elapsed recovery time: 14 seconds

Time: 00:30 - CLIENT REQUEST DURING RECOVERY
─────────────────────────────────────────────
  Client Alice: EncryptionRequest → Node 2

  [Node 2] I'm the coordinator, checking load
  [Node 2] My load: 30% < 50%, process locally
  [Node 2] Encrypting image...
  [Node 2] Success!
  [Node 2] → Alice: EncryptionResponse

  ✓ Service maintained despite node failure!

Time: 05:00 - NODE 3 RECOVERS
──────────────────────────────
  ** Admin restarts Node 3 **

  [Node 3] Restarting...
  [Node 3] State: RECOVERING
  [Node 3] Binding to 0.0.0.0:8003
  [Node 3] Sending heartbeats to peers

  [Node 1] Received heartbeat from Node 3!
  [Node 1] Node 3 state: FAILED → RECOVERING → ACTIVE

  [Node 2] Received heartbeat from Node 3!
  [Node 2] Node 3 state: FAILED → RECOVERING → ACTIVE
  [Node 2] Still coordinator (I'm Node 2, Node 3 not higher)

  [Node 3] Received COORDINATOR message or heartbeat from Node 2
  [Node 3] Accepting Node 2 as coordinator
  [Node 3] coordinator_id = 2

Time: 05:05 - CLUSTER FULLY RECOVERED
──────────────────────────────────────
  Node 1: ACTIVE, coordinator_id = 2
  Node 2: ACTIVE, COORDINATOR ★, coordinator_id = 2
  Node 3: ACTIVE, coordinator_id = 2

  Full 3-node cluster operational!
```

**Observations**:
- Failure detection: 15 seconds (3 missed heartbeats)
- Election duration: 1 second
- Total recovery: 16 seconds
- Zero service downtime (requests still processed)
- Automatic recovery when node restarts

---

### Use Case 7: Network Partition

**Scenario**: Network cable disconnected, splitting cluster
**Problem**: Split-brain possible
**Result**: Two independent coordinators

#### Flow

```
Before Partition:
  Node 1 (192.168.1.101) ─┐
  Node 2 (192.168.1.102) ─┼─ Switch ─ Router
  Node 3 (192.168.1.103) ─┘   ★ Coordinator

** Network Cable to Node 3 Unplugged **

After Partition:
  Partition A: {Node 1, Node 2}
  Partition B: {Node 3}

Partition A Perspective:
  [Node 1] No heartbeat from Node 3 for 15s
  [Node 2] No heartbeat from Node 3 for 15s
  [Node 1] Trigger election
  [Node 2] Trigger election
  [Node 2] Wins (higher ID than Node 1)
  [Node 2] Becomes coordinator

  Result: Node 2 is coordinator in Partition A

Partition B Perspective:
  [Node 3] No heartbeat from Nodes 1, 2 for 15s
  [Node 3] Marking Nodes 1, 2 as FAILED
  [Node 3] I was already coordinator, remain coordinator
  [Node 3] Trigger election just to be sure
  [Node 3] No other nodes respond
  [Node 3] Confirm self as coordinator

  Result: Node 3 is coordinator in Partition B

SPLIT-BRAIN SCENARIO:
  - Partition A thinks Node 2 is coordinator
  - Partition B thinks Node 3 is coordinator
  - Both process requests independently
  - Data divergence possible

Client Behavior:
  - Clients connected to Partition A → Node 2 handles
  - Clients connected to Partition B → Node 3 handles
  - No cross-partition communication

** Network Cable Reconnected **

Partition Healed:
  [All Nodes] Heartbeats resume
  [Node 2] Received heartbeat from Node 3
  [Node 3] Received heartbeat from Nodes 1, 2
  [Node 2] Wait... Node 3 is higher than me!
  [Node 2] Trigger new election
  [Node 3] Wins election (highest ID)
  [Node 3] Becomes coordinator again

Final State:
  Node 1: ACTIVE
  Node 2: ACTIVE
  Node 3: ACTIVE, COORDINATOR ★

  Normal operation resumed
```

**Data Inconsistency**:
- Sessions registered in Partition A not visible in Partition B
- Images stored in one partition not in the other
- No automatic reconciliation

**Mitigation** (Future):
- Implement quorum (require majority)
- Use Raft or Paxos consensus
- Network partition detection

---

## Security Scenarios

### Use Case 8: Unauthorized View Attempt

**Scenario**: Eve tries to view image she's not authorized for
**Goal**: System denies access

#### Flow

```
Setup:
  - Alice sent image to Bob (quota: 3)
  - Image stored on Node 2
  - Eve is a registered user

Eve Attempts Access:
  1. Eve logs in successfully
  2. Eve sees "Received Images" is empty
  3. Eve somehow discovers image ID: "req_alice_1234567890"
  4. Eve manually crafts ViewImage request

  Network Activity:
  Eve → Node 2: ViewImage {
      username: "eve",
      image_id: "req_alice_1234567890",
  }

  Node 2 Processing:
  1. Lookup images for username "eve"
  2. Search for image_id "req_alice_1234567890"
  3. NOT FOUND (image is stored for "bob", not "eve")

  Node 2 → Eve: ViewImageResponse {
      success: false,
      error: Some("Image not found"),
  }

  Eve's GUI shows: ❌ Error fetching image: Image not found

Result: ✓ Access denied, security maintained
```

---

### Use Case 9: Quota Exhaustion Attack

**Scenario**: Malicious user tries to exhaust another user's quota
**Goal**: System enforces quota correctly

#### Flow

```
Setup:
  - Alice sent image to Bob (quota: 5)
  - Image ID: "img_123"

Bob's View Count: 0/5

Eve's Attack Attempt:
  Eve tries to view Bob's image 10 times

  ViewImage requests:
  Eve → Node: ViewImage { username: "eve", image_id: "img_123" }
  ✗ Image not found (not authorized for Eve)

  No effect on Bob's quota!

Bob Views Legitimately:
  View 1: remaining = 4
  View 2: remaining = 3
  View 3: remaining = 2
  View 4: remaining = 1
  View 5: remaining = 0, IMAGE DELETED

  After deletion, even Bob can't view anymore

Result: ✓ Quota enforced per authorized user only
```

---

## Performance Scenarios

### Use Case 10: High Concurrency - 100 Clients Simultaneously

**Scenario**: 100 clients send encryption requests at same time
**Goal**: System handles load gracefully

#### Flow

```
Test Setup:
  - 3 cloud nodes running
  - 100 client instances
  - Each sends 1 encryption request simultaneously

Load Distribution Algorithm:

Time 00:00.000 - Requests Arrive
─────────────────────────────────
  100 EncryptionRequests → Coordinator (Node 3)

Time 00:00.001 - Coordinator Queues Requests
─────────────────────────────────────────────
  [Node 3] Received 100 requests
  [Node 3] Queue length: 100
  [Node 3] Load: 100% (queue full!)
  [Node 3] Start forwarding...

Time 00:00.010 - Load Balancing Begins
───────────────────────────────────────
  [Node 3] Query peers for load

  Load Responses:
    Node 1: 0%
    Node 2: 0%

  [Node 3] Forward to Node 1: 50 requests
  [Node 3] Forward to Node 2: 50 requests

Time 00:00 - 00:10 - Processing
────────────────────────────────
  Node 1: Processing 50 requests
    - Parallel processing (up to 10 concurrent)
    - Average: 55ms per request
    - Total: ~5 seconds

  Node 2: Processing 50 requests
    - Parallel processing
    - Average: 55ms per request
    - Total: ~5 seconds

Time 00:05.000 - First Responses
─────────────────────────────────
  Clients start receiving EncryptionResponse messages

Time 00:10.000 - All Completed
───────────────────────────────
  All 100 requests processed

  Results:
    - Success: 100
    - Failures: 0
    - Average latency: 5.5 seconds
    - Throughput: 20 requests/second

Performance Metrics:
  - CPU usage: 85% (Node 1), 85% (Node 2), 40% (Node 3)
  - Memory: 512 MB per node
  - Network: 110 MB transferred (1.1 MB × 100 images)
```

---

## Edge Cases

### Use Case 11: Image Too Large for UDP

**Scenario**: User uploads 5 MB image
**Problem**: UDP packet limit is 65 KB
**Solution**: Auto-compression + chunking

#### Flow

```
1. USER UPLOADS LARGE IMAGE
   ───────────────────────
   File: high_res_photo.jpg (5 MB, 4000×3000 pixels)

2. CLIENT-SIDE COMPRESSION
   ──────────────────────
   GUI detects: 5 MB > 10 KB limit

   Compression steps:
   a) Load image
   b) Calculate scale: scale = sqrt(10KB / 5MB) = 0.045
   c) Resize: 4000×3000 → 180×135 pixels
   d) Re-encode as JPEG (quality: 75%)
   e) Result: 9.8 KB

   GUI shows: ✓ Image compressed to 9.8 KB

3. ENCRYPTION REQUEST
   ──────────────────
   Client → Node: EncryptionRequest { image_data: [9,800 bytes] }

   Request size: 9,800 + overhead = ~11 KB ✓ (fits in UDP)

4. ENCRYPTION RESPONSE
   ───────────────────
   Encrypted image: 1.1 MB (cover image size)

   Response size: 1.1 MB > 65 KB ✗ (needs chunking!)

5. CHUNKING
   ────────
   Node 3: Message too large, fragmenting...

   Chunks:
   - Chunk 1: 45 KB
   - Chunk 2: 45 KB
   - ...
   - Chunk 25: 10 KB

   Total: 25 chunks

6. CHUNK TRANSMISSION
   ──────────────────
   Node 3 sends chunks with 2ms delay between each:

   Chunk 1 → Client (45 KB)
   Sleep 2ms
   Chunk 2 → Client (45 KB)
   Sleep 2ms
   ...
   Chunk 25 → Client (10 KB)

   Total time: 25 × 2ms = 50ms

7. CLIENT REASSEMBLY
   ─────────────────
   Client receives all 25 chunks
   Client reassembles: 1.1 MB encrypted image

   GUI shows: ✅ Success! Encrypted data size: 1,100,000 bytes

Result: ✓ Large image handled automatically
```

---

### Use Case 12: All Nodes Fail Except One

**Scenario**: Nodes 1 and 2 crash, only Node 3 remains
**Impact**: Reduced capacity, no redundancy
**Behavior**: System continues with single node

#### Flow

```
Initial State:
  Node 1: ACTIVE
  Node 2: ACTIVE
  Node 3: ACTIVE, COORDINATOR

Failure Events:
  Time 00:00: Node 1 crashes (power failure)
  Time 00:05: Node 2 crashes (software bug)

Time 00:15 - Node 3 Detects Failures
─────────────────────────────────────
  [Node 3] No heartbeat from Node 1 for 15s → FAILED
  [Node 3] No heartbeat from Node 2 for 15s → FAILED
  [Node 3] I'm the coordinator, I remain coordinator
  [Node 3] Active peers: 0
  [Node 3] Cluster degraded: 1/3 nodes operational

Time 00:16 - Client Request Arrives
────────────────────────────────────
  Client → Node 3: EncryptionRequest

  [Node 3] I'm coordinator, check my load
  [Node 3] My load: 45%
  [Node 3] No active peers to forward to
  [Node 3] Processing locally

  [Node 3] Encrypting...
  [Node 3] Success!
  [Node 3] → Client: EncryptionResponse

  ✓ Request processed despite degraded cluster

Limitations:
  - No load balancing (only 1 node)
  - No redundancy (single point of failure)
  - Reduced throughput
  - If Node 3 fails → total outage

Administrator Actions:
  1. Restart Nodes 1 and 2
  2. Cluster recovers to 3/3 nodes
  3. Full capacity restored
```

---

**Last Updated**: 2025-11-23
**Total Use Cases**: 12 scenarios
**Coverage**: User operations, system management, failures, security, performance, edge cases
