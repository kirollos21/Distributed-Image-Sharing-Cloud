#!/bin/bash

echo "=========================================="
echo "  Distributed Cloud Setup Helper"
echo "=========================================="
echo ""

# Detect IP address
echo "Step 1: Detecting your IP address..."
if command -v hostname &> /dev/null; then
    MY_IP=$(hostname -I | awk '{print $1}')
elif command -v ip &> /dev/null; then
    MY_IP=$(ip addr show | grep "inet " | grep -v 127.0.0.1 | awk '{print $2}' | cut -d/ -f1 | head -n1)
else
    MY_IP="Unable to detect"
fi

echo "Your IP address appears to be: $MY_IP"
echo ""

# Ask for node ID
echo "Step 2: Which node will this machine run?"
echo "Enter 1, 2, or 3:"
read -p "Node ID: " NODE_ID

if [[ ! "$NODE_ID" =~ ^[1-3]$ ]]; then
    echo "Invalid node ID. Must be 1, 2, or 3."
    exit 1
fi

echo ""
echo "Step 3: Enter IP addresses for all 3 nodes"
echo "(Press Enter to use detected IP for this node)"

# Get Node 1 IP
if [ "$NODE_ID" -eq 1 ]; then
    read -p "Node 1 IP [$MY_IP]: " NODE1_IP
    NODE1_IP=${NODE1_IP:-$MY_IP}
else
    read -p "Node 1 IP: " NODE1_IP
fi

# Get Node 2 IP
if [ "$NODE_ID" -eq 2 ]; then
    read -p "Node 2 IP [$MY_IP]: " NODE2_IP
    NODE2_IP=${NODE2_IP:-$MY_IP}
else
    read -p "Node 2 IP: " NODE2_IP
fi

# Get Node 3 IP
if [ "$NODE_ID" -eq 3 ]; then
    read -p "Node 3 IP [$MY_IP]: " NODE3_IP
    NODE3_IP=${NODE3_IP:-$MY_IP}
else
    read -p "Node 3 IP: " NODE3_IP
fi

echo ""
echo "Configuration Summary:"
echo "  Node 1: $NODE1_IP:8001"
echo "  Node 2: $NODE2_IP:8002"
echo "  Node 3: $NODE3_IP:8003"
echo "  This machine: Node $NODE_ID"
echo ""

read -p "Is this correct? (y/n): " CONFIRM
if [[ ! "$CONFIRM" =~ ^[Yy]$ ]]; then
    echo "Setup cancelled."
    exit 0
fi

# Create backup
echo ""
echo "Step 4: Creating backup of original files..."
cp src/bin/cloud_node.rs src/bin/cloud_node.rs.backup
cp src/gui_client.rs src/gui_client.rs.backup
echo "Backups created: *.backup"

# Update cloud_node.rs
echo ""
echo "Step 5: Updating cloud_node.rs..."
cat > /tmp/node_addresses.txt << EOF
    // Define node addresses (3 cloud nodes)
    let node_addresses = vec![
        (1, "$NODE1_IP:8001".to_string()),
        (2, "$NODE2_IP:8002".to_string()),
        (3, "$NODE3_IP:8003".to_string()),
    ];
EOF

# Replace the configuration
sed -i '/Define node addresses/,/];/{
    /Define node addresses/r /tmp/node_addresses.txt
    d
}' src/bin/cloud_node.rs

# Update gui_client.rs
echo "Step 6: Updating gui_client.rs..."
cat > /tmp/cloud_addresses.txt << EOF
            cloud_addresses: vec![
                "$NODE1_IP:8001".to_string(),
                "$NODE2_IP:8002".to_string(),
                "$NODE3_IP:8003".to_string(),
            ],
EOF

sed -i '/cloud_addresses: vec!/,/],/{
    /cloud_addresses: vec!/r /tmp/cloud_addresses.txt
    d
}' src/gui_client.rs

# Configure firewall
echo ""
echo "Step 7: Configuring firewall..."
PORT=$((8000 + NODE_ID))

if command -v ufw &> /dev/null; then
    echo "Detected UFW firewall"
    read -p "Open port $PORT? (requires sudo) (y/n): " OPEN_PORT
    if [[ "$OPEN_PORT" =~ ^[Yy]$ ]]; then
        sudo ufw allow $PORT/udp
        echo "Port $PORT opened (UDP)"
    fi
elif command -v firewall-cmd &> /dev/null; then
    echo "Detected firewalld"
    read -p "Open port $PORT? (requires sudo) (y/n): " OPEN_PORT
    if [[ "$OPEN_PORT" =~ ^[Yy]$ ]]; then
        sudo firewall-cmd --add-port=$PORT/udp --permanent
        sudo firewall-cmd --reload
        echo "Port $PORT opened (UDP)"
    fi
else
    echo "No firewall detected or manual configuration needed"
    echo "Please manually open port $PORT (UDP)"
fi

# Build
echo ""
echo "Step 8: Building the project..."
echo "This may take a few minutes..."
cargo build --release --bin cloud-node --bin client-gui --bin server-gui

if [ $? -eq 0 ]; then
    echo ""
    echo "=========================================="
    echo "  ✅ Setup Complete!"
    echo "=========================================="
    echo ""
    echo "To start Node $NODE_ID:"
    echo "  cargo run --release --bin cloud-node $NODE_ID"
    echo ""
    echo "To start Client GUI:"
    echo "  cargo run --release --bin client-gui"
    echo ""
    echo "To start Server Monitor:"
    echo "  cargo run --release --bin server-gui $NODE_ID"
    echo ""
    echo "Network Configuration:"
    echo "  Node 1: $NODE1_IP:8001"
    echo "  Node 2: $NODE2_IP:8002"
    echo "  Node 3: $NODE3_IP:8003"
    echo ""
    echo "To restore original files:"
    echo "  mv src/bin/cloud_node.rs.backup src/bin/cloud_node.rs"
    echo "  mv src/gui_client.rs.backup src/gui_client.rs"
    echo ""
else
    echo "Build failed! Check errors above."
    echo "Restoring backup files..."
    mv src/bin/cloud_node.rs.backup src/bin/cloud_node.rs
    mv src/gui_client.rs.backup src/gui_client.rs
    exit 1
fi
