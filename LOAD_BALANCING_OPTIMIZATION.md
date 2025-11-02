# Load Balancing Overhead Optimization

## Problem

The original load balancing system was sending `LoadQuery` messages to **all peer nodes** for **every single encryption request**. This created massive network overhead:

- With 100 requests/sec and 2 peer nodes: **200 extra LoadQuery messages/sec**
- Plus 200 LoadResponse messages/sec
- **Total: 400 messages/sec just for load balancing**

Additionally, the load monitoring task was also sending LoadQuery messages every 10 seconds to all peers.

## Solution: Load Caching via Heartbeat Piggybacking

Instead of querying nodes on-demand, we now **piggyback load information on existing heartbeat messages**.

### Key Changes:

#### 1. Enhanced Heartbeat Messages (src/messages.rs)
```rust
// BEFORE:
Heartbeat { from_node: NodeId }

// AFTER:
Heartbeat {
    from_node: NodeId,
    load: f64,              // Current load (active requests)
    processed_count: usize, // Total processed requests
}
```

Heartbeat acknowledgments also include load data, so **every heartbeat exchange** updates load information in both directions.

#### 2. Added Load Cache (src/node.rs)
```rust
pub struct CachedLoadInfo {
    pub load: f64,
    pub processed_count: usize,
    pub timestamp: Instant,  // When this was received
}

// In CloudNode:
pub peer_load_cache: Arc<RwLock<HashMap<NodeId, CachedLoadInfo>>>
```

#### 3. Cache Population (src/node.rs:679-733)
When receiving `Heartbeat` or `HeartbeatAck`:
```rust
let mut load_cache = self.peer_load_cache.write().await;
load_cache.insert(from_node, CachedLoadInfo {
    load,
    processed_count,
    timestamp: Instant::now(),
});
```

#### 4. Optimized Load Balancing (src/node.rs:944-1033)
`find_lowest_load_node()` now uses cached data with a 5-second TTL:

```rust
// Get cached load data from heartbeats
let load_cache = self.peer_load_cache.read().await;

for (peer_id, _) in &self.peer_addresses {
    if let Some(cached) = load_cache.get(peer_id) {
        let age = now.duration_since(cached.timestamp);

        if age < CACHE_TTL {  // 5 seconds
            // Use cached data - NO NETWORK CALL!
            node_data.insert(*peer_id, (cached.load, cached.processed_count));
        }
    }
}
```

#### 5. Optimized Load Monitoring (src/node.rs:1416-1508)
The monitoring task (runs every 10 seconds) now also uses cached data instead of sending LoadQuery messages.

## Performance Impact

### Before Optimization:
| Operation | Messages/Request | Messages/10sec (monitoring) |
|-----------|------------------|------------------------------|
| Load balancing | 2N (query + response per peer) | 2N |
| **With 100 req/sec, 2 peers** | **400 msg/sec** | **4 msg** |
| **Total** | **400 msg/sec** | **+ 4 msg/10sec** |

### After Optimization:
| Operation | Messages/Request | Messages/5sec (heartbeat) |
|-----------|------------------|---------------------------|
| Load balancing | **0** (uses cache) | 2N (heartbeat + ack per peer) |
| Monitoring | **0** (uses cache) | Already counted in heartbeat |
| **With 100 req/sec, 2 peers** | **0 msg/sec** | **4 msg/5sec = 0.8 msg/sec** |
| **Total** | **0 msg/sec** | **+ 0.8 msg/sec (unavoidable)** |

### Overhead Reduction:
- **Load balancing overhead: 400 msg/sec → 0 msg/sec (100% reduction)**
- **Monitoring overhead: 0.4 msg/sec → 0 msg/sec (100% reduction)**
- **Only heartbeat overhead remains: 0.8 msg/sec (required for failure detection)**

**Net reduction: ~99.8% fewer messages**

## Trade-offs

### Benefits:
✅ **Massive reduction in network traffic** (400+ msg/sec → 2 msg/sec)
✅ **No additional latency** - load balancing decisions are instant (no waiting for queries)
✅ **More scalable** - overhead stays constant regardless of request rate
✅ **Simpler code** - no complex query/response handling in load balancing

### Potential Drawbacks:
⚠️ **Load data can be up to 5-10 seconds stale** (heartbeat interval + cache TTL)
  - **Mitigation**: This is acceptable for load balancing, as load changes gradually
  - **Note**: Increased from 2-5 seconds to reduce overhead even further (0.8 msg/sec vs 2 msg/sec)
⚠️ **Slightly higher heartbeat message size** (~20 extra bytes per heartbeat)
  - **Impact**: Negligible (4 bytes for f64 + ~8 bytes for usize)

## Cache Freshness

- **Heartbeat interval**: 5 seconds
- **Cache TTL**: 10 seconds (2x heartbeat interval)
- **Failure timeout**: 20 seconds (4x heartbeat interval)
- **Maximum staleness**: 10 seconds (worst case: heartbeat just missed)
- **Typical staleness**: 2.5-5 seconds (average of heartbeat interval)

For load balancing, this level of staleness is acceptable because:
1. Load changes gradually (nodes don't go from 0% to 100% instantly)
2. The hybrid scoring (70% load + 30% historical work) smooths out fluctuations
3. The coordinator hysteresis (20% threshold) prevents rapid coordinator changes
4. Even 10-second-old data is sufficient for making good load balancing decisions

## Status Messages

The monitoring table now shows different statuses:

| Status | Meaning |
|--------|---------|
| `COORDINATOR` | This node is the current coordinator |
| `Worker` | Normal worker node with fresh cache (< 15 sec) |
| `STALE` | Cache is old (> 15 sec) but node not yet marked failed |
| `NO_HEARTBEAT` | Never received a heartbeat from this node |
| `FAILED` | Node marked as failed by failure detector (> 20 sec timeout) |

## Testing

To verify the optimization is working, look for these log patterns:

**Good (cache hit):**
```
[Node 1] Finding lowest load node (my load: 2.00, processed: 45)
[Node 1] Using cached load for Node 2 (age: 1.2s, load: 1.50)
[Node 1] Selected node: 2 (score: 1.05) [CACHED]
```

**Monitoring (no queries sent):**
```
[Node 1] ════════════════ CLUSTER LOAD DISTRIBUTION ════════════════
[Node 1] │ Node 2 │ Worker     │     1.50% │         45 │            1 │
[Node 1] │ Node 3 │ Worker     │     2.00% │         52 │            2 │
```

Notice there are **no LoadQuery/LoadResponse log messages** during normal operation!

## Configuration

No configuration changes needed. The optimization is automatic and transparent.

**Key timing parameters** can be adjusted in `src/node.rs`:

```rust
// Heartbeat interval (heartbeat_sender_task)
let mut interval = interval(Duration::from_secs(5));

// Cache TTL (find_lowest_load_node)
const CACHE_TTL: Duration = Duration::from_secs(10);

// Failure detection timeout (failure_detector_task)
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(20);
```

**Current settings:**
- Heartbeat interval: 5 seconds
- Cache TTL: 10 seconds (2x heartbeat)
- Failure timeout: 20 seconds (4x heartbeat)

**Tuning guidelines:**
- Longer heartbeat interval = less network overhead, but slower failure detection
- Shorter heartbeat interval = faster updates, but more network traffic
- Cache TTL should be 2x heartbeat interval for best results
- Failure timeout should be 4x heartbeat interval to avoid false positives
