# START HERE - Quick Guide

## 🎯 Run the Complete Test in 3 Steps

### 1. Edit Configuration (Optional)
```bash
nano config.json
```

Change these values as needed:
- `num_processes`: How many concurrent processes (default: 10)
- `images_per_process`: Images per process (default: 10)
- `servers`: List of your server IPs

### 2. Make Sure Servers Are Running

On each server machine:
```bash
cd "/media/kirollos/Data/Distributed Systems/Cloud Project"
cargo run --release --bin cloud-node -- <node_id> <ip:port>
```

Example:
```bash
# Server 1
cargo run --release --bin cloud-node -- 1 10.40.59.43:8001

# Server 2
cargo run --release --bin cloud-node -- 2 10.40.59.44:8001

# Server 3
cargo run --release --bin cloud-node -- 3 10.40.59.45:8001
```

### 3. Run the Test
```bash
./run_full_test.sh
```

That's it! ✨

## 📊 What Happens

1. **Spawns N processes** (default: 10)
2. **Each process generates and sends M images** (default: 10)
3. **Total: N × M images** (default: 100 images)
4. **Sends to random servers** (load balancing)
5. **Collects encrypted images**
6. **Decrypts all for verification**
7. **Generates comprehensive report**

## 📁 Results

All results saved to `output/`:

```bash
# View quick summary
cat output/metrics/analysis_report.txt

# View detailed metrics
cat output/metrics/aggregated_metrics.json

# Check specific folders
ls output/test_images/     # Generated test images
ls output/encrypted/       # Encrypted images from servers
ls output/decrypted/       # Decrypted verification images
ls output/metrics/         # All metrics and plots
```

## 🎛️ Common Configurations

### Light Test (30 images)
```json
"num_processes": 3,
"images_per_process": 10
```

### Standard Test (100 images - DEFAULT)
```json
"num_processes": 10,
"images_per_process": 10
```

### Heavy Stress Test (1000 images)
```json
"num_processes": 50,
"images_per_process": 20
```

### Single Server Test
```json
"servers": ["10.40.59.43:8001"]
```

## 📈 Key Metrics Tracked

- ✅ **Success Rate**: % of images successfully encrypted
- ⏱️ **Latency**: Average, P50, P95, P99, Min, Max
- 🚀 **Throughput**: Images processed per second
- 🖥️ **Per-Server Stats**: Distribution and performance
- 🔐 **Decryption Verification**: % successfully decrypted

## 🔧 Troubleshooting

**No images sent?**
- Check servers are running
- Verify IPs in config.json

**High failure rate?**
- Increase `request_timeout` in config.json
- Check network connectivity
- Review server logs

**Decryption failing?**
- Verify images saved as PNG (lossless)
- Check encryption succeeded first

## 📚 Need More Info?

See `README.md` for comprehensive documentation.

---

**Happy Testing!** 🚀
