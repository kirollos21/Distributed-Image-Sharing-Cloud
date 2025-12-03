# Distributed Image Cloud - Complete Documentation

## 📚 Documentation Overview

Welcome to the comprehensive documentation for the Distributed Image Cloud system - a peer-to-peer distributed system for secure image sharing using LSB steganography encryption.

---

## 🎯 Quick Start

**New to the project?** Start here:

1. **[README.md](README.md)** - Project overview and quick start guide
2. **[GUI_README.md](GUI_README.md)** - GUI applications user guide
3. **[USE_CASES.md](USE_CASES.md)** - Practical usage scenarios

**Ready to deploy?**

```bash
# Build the system
cargo build --release --bin cloud-node --bin server-gui --bin client-gui

# Start the system
./start_gui_system.sh

# Start a client
RUST_LOG=info target/release/client-gui alice
```

---

## 📖 Documentation Structure

### 1. **Service Directory** 📋
**File**: [SERVICE_DIRECTORY.md](SERVICE_DIRECTORY.md)

Complete catalog of all services, APIs, and interfaces in the system.

**Contents**:
- Core Services (Cloud Node, Encryption, Session Management)
- Client Services (Client API, GUI applications)
- Support Services (Load Balancing, Leader Election, Heartbeat)
- Message Protocol specification
- API Reference
- Performance characteristics

**Use this when**: You need to understand what services are available, how to call them, or what messages to send.

---

### 2. **Peer-to-Peer Operations** 🔗
**File**: [PEER_TO_PEER_OPERATIONS.md](PEER_TO_PEER_OPERATIONS.md)

Detailed documentation of distributed algorithms and peer-to-peer protocols.

**Contents**:
- Node Discovery & Initialization
- Leader Election (Bully Algorithm)
- Heartbeat & Failure Detection
- Load Balancing algorithm
- Request Forwarding protocol
- Data Replication
- Network Communication
- Fault Tolerance mechanisms

**Use this when**: You need to understand how nodes communicate, coordinate, handle failures, or distribute load.

---

### 3. **Use Cases** 📝
**File**: [USE_CASES.md](USE_CASES.md)

Detailed scenarios showing how the system works in practice.

**Contents**:
- **User Scenarios**: Alice shares photo with Bob, multiple recipients, medical imaging
- **System Operations**: Startup, load balancing, dynamic coordination
- **Failure Scenarios**: Coordinator crashes, network partition, cascading failures
- **Security Scenarios**: Unauthorized access, quota attacks
- **Performance Scenarios**: 100 concurrent clients
- **Edge Cases**: Large images, single-node operation

**Use this when**: You want to see real-world examples of how the system behaves in different situations.

---

### 4. **Architecture** 🏗️
**File**: [ARCHITECTURE.md](ARCHITECTURE.md)

System architecture with comprehensive diagrams and design documentation.

**Contents**:
- High-Level Architecture (system overview)
- Component Architecture (layered design)
- Network Architecture (topology, communication patterns)
- Data Flow Diagrams (encryption flow, viewing flow, session flow)
- Deployment Architecture (single/multi-machine)
- Security Architecture (encryption model, access control)

**Use this when**: You need to understand system structure, component interactions, or deployment options.

---

### 5. **Encryption Documentation** 🔐

#### **NEW_ENCRYPTION_METHOD.md**
Description of the LSB steganography encryption method.

**Key Points**:
- **Method**: Image-on-image steganography using LSB (Least Significant Bit) encoding
- **Cover Image**: `encrypction_key.jpg` serves as the encryption key
- **Result**: Encrypted PNG that visually looks identical to the cover image
- **Performance**: ~50ms encryption, ~25ms decryption
- **Security**: Visual stealth, quota enforcement

#### **TEST_ENCRYPTION_README.md**
Guide for testing the encryption system.

**Quick Test**:
```bash
./test_encryption.sh
```

**Expected Output**:
```
✅ PERFECT MATCH - All 87,241 bytes identical!
✅ Metadata matches!
```

---

### 6. **GUI Documentation** 🖥️
**File**: [GUI_README.md](GUI_README.md)

User guide for the graphical applications.

**Applications**:

1. **Client GUI**
   - Upload & encrypt images
   - Configure authorized users and quotas
   - Send to cloud and share with users
   - View received images
   - Track request history

2. **Server GUI**
   - Monitor node status
   - View real-time logs
   - Track metrics
   - Visualize network health

**Quick Launch**:
```bash
# Server monitoring
./server-gui 127.0.0.1:8001 127.0.0.1:8002 127.0.0.1:8003

# Client
./client-gui alice
```

---

## 🗂️ Additional Documentation Files

### Request Flow Documentation
- **[REQUEST_FLOW.md](REQUEST_FLOW.md)** - Overview of request processing
- **[REQUEST_FLOW_DETAILED.md](REQUEST_FLOW_DETAILED.md)** - Detailed request flow with examples

### Historical Documentation
- **[CACHE_IMPLEMENTATION.md](CACHE_IMPLEMENTATION.md)** - Request caching system
- **[ELECTION_ALGORITHM.md](ELECTION_ALGORITHM.md)** - Bully algorithm details
- **[FAILOVER_PROCEDURE.md](FAILOVER_PROCEDURE.md)** - Node failover process
- **[GUI_UPDATES.md](GUI_UPDATES.md)** - GUI changelog
- **[HEARTBEAT_OPTIMIZATION.md](HEARTBEAT_OPTIMIZATION.md)** - Heartbeat tuning
- **[INTEGRATION_GUIDE.md](INTEGRATION_GUIDE.md)** - Integration instructions
- **[MONITORING_GUIDE.md](MONITORING_GUIDE.md)** - Monitoring setup
- **[P2P_PROTOCOL.md](P2P_PROTOCOL.md)** - P2P protocol specification

---

## 🚀 Quick Reference

### Common Commands

```bash
# Build everything
cargo build --release

# Start cloud nodes
target/release/cloud-node 1 8001 &
target/release/cloud-node 2 8002 &
target/release/cloud-node 3 8003 &

# Start server monitoring
target/release/server-gui 127.0.0.1:8001 127.0.0.1:8002 127.0.0.1:8003 &

# Start client
target/release/client-gui alice

# Test encryption
./test_encryption.sh

# Start full system
./start_gui_system.sh

# Stop all
./stop_gui_system.sh
```

### File Locations

| Component | Location |
|-----------|----------|
| Node binary | `target/release/cloud-node` |
| Client GUI | `target/release/client-gui` |
| Server GUI | `target/release/server-gui` |
| Encryption key | `encrypction_key.jpg` (project root) |
| Source code | `src/*.rs` |
| Tests | `src/bin/test_*.rs` |

---

## 🎨 System Components Map

```
Distributed Image Cloud
├── Core Components
│   ├── Cloud Nodes (src/node.rs, src/bin/cloud_node.rs)
│   ├── Encryption Service (src/encryption.rs)
│   ├── Client API (src/client.rs)
│   └── Message Protocol (src/messages.rs)
│
├── GUI Applications
│   ├── Client GUI (src/gui_client.rs, src/bin/client_gui.rs)
│   └── Server GUI (src/gui_server.rs, src/bin/server_gui.rs)
│
├── Support Modules
│   ├── Chunking (src/chunking.rs)
│   ├── Metrics (src/metrics.rs)
│   └── Utilities (src/main.rs, src/lib.rs)
│
├── Scripts
│   ├── start_gui_system.sh
│   ├── stop_gui_system.sh
│   ├── test_encryption.sh
│   └── create_test_image.py
│
└── Documentation
    ├── SERVICE_DIRECTORY.md ← Service catalog
    ├── PEER_TO_PEER_OPERATIONS.md ← P2P algorithms
    ├── USE_CASES.md ← Practical scenarios
    ├── ARCHITECTURE.md ← System design
    ├── GUI_README.md ← GUI guide
    ├── NEW_ENCRYPTION_METHOD.md ← Encryption details
    └── DOCUMENTATION_INDEX.md ← You are here!
```

---

## 📊 Key Metrics

### Performance
- **Encryption**: 50-60ms (1.5 MB/s)
- **Decryption**: 25-30ms (3.2 MB/s)
- **Heartbeat interval**: 5 seconds
- **Failure detection**: 15 seconds
- **Election duration**: 2-5 seconds

### Capacity
- **Nodes**: 3 (scalable to more)
- **Max concurrent requests**: ~100 per node
- **Image size limit**: 10 KB (auto-compressed)
- **Encrypted size**: ~1.1 MB (depends on cover image)
- **Max UDP packet**: 65 KB (chunking used for larger)

### Network
- **Protocol**: UDP
- **Topology**: Full mesh P2P
- **Coordinator**: Elected via Bully algorithm
- **Heartbeat bandwidth**: ~0.1 KB/s per peer

---

## 🔍 Finding Information

### "How do I..."

| Question | Document | Section |
|----------|----------|---------|
| ...start the system? | GUI_README.md | Running the GUIs |
| ...upload an image? | USE_CASES.md | Use Case 1: Alice shares photo |
| ...understand encryption? | NEW_ENCRYPTION_METHOD.md | How It Works |
| ...monitor nodes? | GUI_README.md | Server Monitor GUI |
| ...call the API? | SERVICE_DIRECTORY.md | Client API Service |
| ...handle node failures? | PEER_TO_PEER_OPERATIONS.md | Fault Tolerance |
| ...deploy to production? | ARCHITECTURE.md | Deployment Architecture |

### "What is..."

| Question | Document | Section |
|----------|----------|---------|
| ...the Bully algorithm? | PEER_TO_PEER_OPERATIONS.md | Leader Election Protocol |
| ...LSB steganography? | NEW_ENCRYPTION_METHOD.md | Method Overview |
| ...the load balancing algorithm? | PEER_TO_PEER_OPERATIONS.md | Load Balancing |
| ...the message format? | SERVICE_DIRECTORY.md | Message Protocol |
| ...the system architecture? | ARCHITECTURE.md | High-Level Architecture |

### "Why does..."

| Question | Document | Section |
|----------|----------|---------|
| ...the encrypted image look like the key? | NEW_ENCRYPTION_METHOD.md | Cover Image |
| ...the coordinator change? | PEER_TO_PEER_OPERATIONS.md | Leader Election |
| ...my request get forwarded? | PEER_TO_PEER_OPERATIONS.md | Load Balancing |
| ...the node fail? | USE_CASES.md | Failure Scenarios |
| ...quota enforcement matter? | ARCHITECTURE.md | Access Control |

---

## 🛠️ Troubleshooting Guide

### Common Issues

**Issue**: "Encryption key image not found"
- **Cause**: Missing `encrypction_key.jpg`
- **Solution**: Place an image file named `encrypction_key.jpg` in project root
- **Reference**: NEW_ENCRYPTION_METHOD.md, Setup section

**Issue**: "Failed to connect to any cloud node"
- **Cause**: Nodes not running or wrong addresses
- **Solution**: Check nodes with `ps aux | grep cloud-node`, verify ports
- **Reference**: GUI_README.md, Troubleshooting

**Issue**: "Image too large for UDP"
- **Cause**: Image > 10 KB after compression
- **Solution**: Manually resize image before upload
- **Reference**: USE_CASES.md, Edge Cases - Use Case 11

**Issue**: "Username already in use"
- **Cause**: Username registered by another client
- **Solution**: Choose different username or restart client that owns it
- **Reference**: SERVICE_DIRECTORY.md, Session Management

**Issue**: "Coordinator not responding"
- **Cause**: Coordinator node crashed
- **Solution**: Wait 15s for automatic re-election
- **Reference**: PEER_TO_PEER_OPERATIONS.md, Leader Election

---

## 📈 Development Roadmap

### Current Version (1.0)
- ✅ LSB steganography encryption
- ✅ Peer-to-peer mesh topology
- ✅ Bully algorithm leader election
- ✅ Load balancing
- ✅ Client & Server GUIs
- ✅ Session management
- ✅ Quota enforcement

### Planned Enhancements
- [ ] Use RPC library for peer communication (gRPC/Tarpc)
- [ ] Service discovery/directory
- [ ] State replication (sessions, images)
- [ ] Raft consensus (replace Bully)
- [ ] Password authentication
- [ ] TLS encryption for network
- [ ] Image integrity verification (HMAC)
- [ ] Persistent storage (database)
- [ ] Horizontal scaling (> 3 nodes)

---

## 📞 Support & Contribution

### Getting Help
1. Check this documentation index
2. Read the specific documentation file
3. Review use cases for examples
4. Check the GUI README for user issues

### Reporting Issues
- Use GitHub Issues (if applicable)
- Include: version, OS, error messages, steps to reproduce

### Contributing
1. Read ARCHITECTURE.md to understand design
2. Follow existing code patterns
3. Add tests for new features
4. Update relevant documentation

---

## 📅 Version History

### Version 1.0 (2025-11-23)
- Complete rewrite of encryption method to LSB steganography
- New image-on-image encryption using `encrypction_key.jpg`
- Comprehensive documentation suite
- GUI improvements
- Performance optimizations

### Version 0.9 (Previous)
- Pixel scrambling encryption (deprecated)
- Basic P2P coordination
- Initial GUI

---

## 📜 License

Academic use for CSCE 4411 - Distributed Systems
See main README.md for license details

---

## 🎓 Learning Resources

### For Understanding Distributed Systems
- **Leader Election**: PEER_TO_PEER_OPERATIONS.md → Leader Election Protocol
- **Fault Tolerance**: USE_CASES.md → Failure Scenarios
- **Load Balancing**: PEER_TO_PEER_OPERATIONS.md → Load Balancing
- **Consensus**: PEER_TO_PEER_OPERATIONS.md → State Synchronization

### For Understanding Steganography
- **LSB Encoding**: NEW_ENCRYPTION_METHOD.md → How LSB Steganography Works
- **Data Flow**: ARCHITECTURE.md → Image Encryption Flow
- **Testing**: test_encryption.sh + TEST_ENCRYPTION_README.md

### For Understanding UDP Networking
- **Message Protocol**: SERVICE_DIRECTORY.md → Message Protocol
- **Chunking**: SERVICE_DIRECTORY.md → Chunking Service
- **Communication Patterns**: ARCHITECTURE.md → Communication Patterns

---

## 🔗 Quick Links

| Document | Description | Size |
|----------|-------------|------|
| [SERVICE_DIRECTORY.md](SERVICE_DIRECTORY.md) | Service catalog & API reference | ~35 KB |
| [PEER_TO_PEER_OPERATIONS.md](PEER_TO_PEER_OPERATIONS.md) | P2P algorithms & protocols | ~45 KB |
| [USE_CASES.md](USE_CASES.md) | 12 detailed scenarios | ~40 KB |
| [ARCHITECTURE.md](ARCHITECTURE.md) | System architecture & diagrams | ~50 KB |
| [GUI_README.md](GUI_README.md) | GUI user guide | ~20 KB |
| [NEW_ENCRYPTION_METHOD.md](NEW_ENCRYPTION_METHOD.md) | Encryption documentation | ~15 KB |

**Total Documentation**: ~205 KB of comprehensive technical documentation

---

**📚 Last Updated**: 2025-11-23
**✍️ Maintained By**: Distributed Image Cloud Development Team
**🏫 Project**: CSCE 4411 - Distributed Systems

---

**Happy Coding!** 🚀

For questions about specific features, consult the appropriate documentation file listed above.
