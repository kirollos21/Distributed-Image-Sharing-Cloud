# Multiple Image Stress Testing

Comprehensive multi-process stress testing system for distributed image encryption.

## Features

- **Multi-Process Testing**: Run multiple concurrent processes, each sending multiple images
- **Load Balancing**: Randomly distribute requests across multiple servers
- **Automatic Image Generation**: Generate unique test images on-the-fly
- **Full Encryption/Decryption Cycle**: Test complete workflow with verification
- **Comprehensive Metrics**: Success rate, latency, throughput, per-server stats
- **Configurable**: All parameters easily configurable via JSON

## Quick Start

### 1. Configure the Test

Edit `config.json` to set your parameters:

```json
{
  "test_config": {
    "num_processes": 10,        // Number of concurrent processes
    "images_per_process": 10,   // Images each process sends
    "total_images": 100         // Total images (calculated)
  },
  "server_config": {
    "servers": [                // List of server IPs
      "10.40.59.43:8001",
      "10.40.59.44:8001",
      "10.40.59.45:8001"
    ]
  }
}
```

### 2. Run the Complete Test

```bash
chmod +x run_full_test.sh
./run_full_test.sh
```

This will:
1. Clean previous results
2. Spawn N worker processes
3. Generate and send M images per process
4. Collect encrypted images
5. Decrypt all images for verification
6. Generate comprehensive analysis report

### 3. View Results

```bash
# View analysis report
cat output/metrics/analysis_report.txt

# View detailed metrics
cat output/metrics/aggregated_metrics.json

# View decryption results
cat output/metrics/decryption_results.json
```

## Configuration Options

### Test Configuration
- `num_processes`: Number of concurrent worker processes (default: 10)
- `images_per_process`: Images each process sends (default: 10)
- `image_width`: Image width in pixels (default: 1280)
- `image_height`: Image height in pixels (default: 720)
- `image_quality`: JPEG quality 1-100 (default: 85)

### Server Configuration
- `servers`: Array of server addresses in "IP:PORT" format
- `request_timeout`: Timeout for each request in seconds (default: 30.0)
- `retry_attempts`: Number of retry attempts on failure (default: 3)

### Encryption Configuration
- `usernames`: List of authorized usernames for encrypted images
- `quota`: View quota for encrypted images

### Output Configuration
- `save_test_images`: Save generated test images (default: true)
- `save_encrypted_images`: Save encrypted images (default: true)
- `save_decrypted_images`: Save decrypted images (default: true)

## Metrics Tracked

### Overall Metrics
- Total images processed
- Success count and rate
- Failure count and rate
- Total execution time
- Throughput (images/second)

### Latency Statistics
- Average latency
- Median (P50)
- 95th percentile (P95)
- 99th percentile (P99)
- Min/Max latency

### Per-Server Metrics
- Request distribution
- Success/failure per server
- Average latency per server

### Decryption Verification
- Decryption success rate
- Failed decryptions with error details

## Directory Structure

```
multiple_img_testing/
├── config.json                 # Test configuration
├── generate_test_image.py      # Image generator
├── test_worker.py              # Worker process implementation
├── test_coordinator.py         # Coordinator spawns workers
├── decrypt_all.py              # Decrypt and verify all images
├── analyze_metrics.py          # Generate analysis report
├── run_full_test.sh            # Master script
├── README.md                   # This file
└── output/
    ├── test_images/            # Generated test images
    ├── encrypted/              # Encrypted images from servers
    ├── decrypted/              # Decrypted images (verification)
    └── metrics/                # Metrics and analysis
        ├── process_N_results.json      # Per-process results
        ├── aggregated_metrics.json     # All metrics aggregated
        ├── decryption_results.json     # Decryption verification
        ├── analysis_report.txt         # Human-readable report
        ├── latency_distribution.png    # Latency histogram
        └── success_rate_by_server.png  # Per-server success rate
```

## Running Individual Components

### Run Only the Stress Test
```bash
python3 test_coordinator.py
```

### Run Only Decryption
```bash
python3 decrypt_all.py
```

### Run Only Analysis
```bash
python3 analyze_metrics.py
```

### Run a Single Worker (for testing)
```bash
python3 test_worker.py 1
```

## Image Naming Convention

- **Test images**: `test_image_{process_id}_{image_id}.jpg`
  - Example: `test_image_1_5.jpg` (Process 1, Image 5)

- **Encrypted images**: `encrypted_{process_id}_{image_id}.png`
  - Example: `encrypted_1_5.png`

- **Decrypted images**: `decrypted_{process_id}_{image_id}.png`
  - Example: `decrypted_1_5.png`

## Troubleshooting

### No encrypted images
- Check if servers are running
- Verify server addresses in config.json
- Check firewall settings

### High failure rate
- Increase `request_timeout` in config
- Check server logs for errors
- Verify network connectivity

### Decryption failures
- Ensure encrypted images are saved as PNG (lossless)
- Check if encryption completed successfully
- Verify metadata integrity

## Performance Tips

1. **For high-volume testing**: Increase `num_processes` and `images_per_process`
2. **For latency testing**: Use smaller values and analyze P95/P99
3. **For throughput testing**: Use many processes and measure images/sec
4. **For server balance**: Check per-server metrics to verify load distribution

## Dependencies

- Python 3.7+
- PIL/Pillow: `pip install Pillow`
- matplotlib (optional, for plots): `pip install matplotlib`

## Examples

### Light test (30 images)
```json
"num_processes": 3,
"images_per_process": 10
```

### Standard test (100 images)
```json
"num_processes": 10,
"images_per_process": 10
```

### Heavy stress test (1000 images)
```json
"num_processes": 50,
"images_per_process": 20
```

### Single server test
```json
"servers": ["10.40.59.43:8001"]
```

### Three server test (load balancing)
```json
"servers": [
  "10.40.59.43:8001",
  "10.40.59.44:8001",
  "10.40.59.45:8001"
]
```
