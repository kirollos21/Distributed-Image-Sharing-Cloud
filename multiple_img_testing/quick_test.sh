#!/bin/bash
# Quick test with custom parameters

if [ $# -lt 3 ]; then
    echo "Usage: ./quick_test.sh <num_processes> <images_per_process> <server1> [server2] [server3]"
    echo ""
    echo "Examples:"
    echo "  ./quick_test.sh 5 10 10.40.59.43:8001"
    echo "  ./quick_test.sh 10 10 10.40.59.43:8001 10.40.59.44:8001 10.40.59.45:8001"
    echo ""
    exit 1
fi

NUM_PROCESSES=$1
IMAGES_PER_PROCESS=$2
shift 2
SERVERS=("$@")

echo "Quick Test Configuration:"
echo "  Processes: $NUM_PROCESSES"
echo "  Images per process: $IMAGES_PER_PROCESS"
echo "  Total images: $((NUM_PROCESSES * IMAGES_PER_PROCESS))"
echo "  Servers: ${SERVERS[@]}"
echo ""

# Create temporary config
cat > config_temp.json <<EOF
{
  "test_config": {
    "num_processes": $NUM_PROCESSES,
    "images_per_process": $IMAGES_PER_PROCESS,
    "total_images": $((NUM_PROCESSES * IMAGES_PER_PROCESS)),
    "image_width": 1280,
    "image_height": 720,
    "image_format": "JPEG",
    "image_quality": 85
  },
  "server_config": {
    "servers": [
EOF

# Add servers
for i in "${!SERVERS[@]}"; do
    if [ $i -eq $((${#SERVERS[@]} - 1)) ]; then
        echo "      \"${SERVERS[$i]}\"" >> config_temp.json
    else
        echo "      \"${SERVERS[$i]}\"," >> config_temp.json
    fi
done

cat >> config_temp.json <<EOF
    ],
    "request_timeout": 30.0,
    "retry_attempts": 3
  },
  "encryption_config": {
    "usernames": ["alice", "bob", "charlie"],
    "quota": 5
  },
  "output_config": {
    "output_dir": "output",
    "test_images_dir": "output/test_images",
    "encrypted_dir": "output/encrypted",
    "decrypted_dir": "output/decrypted",
    "metrics_dir": "output/metrics",
    "save_test_images": true,
    "save_encrypted_images": true,
    "save_decrypted_images": true
  },
  "metrics_config": {
    "track_latency": true,
    "track_throughput": true,
    "track_success_rate": true,
    "track_failure_rate": true,
    "track_per_server_metrics": true,
    "generate_plots": true
  }
}
EOF

# Backup original config if exists
if [ -f config.json ]; then
    mv config.json config_backup.json
    echo "✓ Backed up original config to config_backup.json"
fi

# Use temp config
mv config_temp.json config.json

# Run test
./run_full_test.sh

# Restore original config
if [ -f config_backup.json ]; then
    mv config_backup.json config.json
    echo "✓ Restored original config"
fi
