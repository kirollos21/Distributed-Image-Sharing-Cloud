#!/bin/bash

################################################################################
# Production Deployment Setup Script
# For Ubuntu 24.04 LTS - Distributed Image Sharing Cloud
################################################################################

set -e  # Exit on error

echo "=============================================="
echo "  Production Deployment Setup"
echo "  Distributed Image Sharing Cloud"
echo "=============================================="
echo ""

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Detect this machine's IP
echo "Step 1: Detecting network configuration..."
MY_IP=$(hostname -I | awk '{print $1}')
HOSTNAME=$(hostname)

echo -e "${GREEN}✓ IP Address: $MY_IP${NC}"
echo -e "${GREEN}✓ Hostname: $HOSTNAME${NC}"
echo ""

# Ask user what type of machine this is
echo "Step 2: Machine Role Selection"
echo "----------------------------------------------"
echo "What role will this machine serve?"
echo ""
echo "  1) Server Node 1 (Cloud Node)"
echo "  2) Server Node 2 (Cloud Node)"
echo "  3) Server Node 3 (Cloud Node)"
echo "  4) Client (GUI)"
echo "  5) Exit"
echo ""
read -p "Select option [1-5]: " ROLE

case $ROLE in
    1|2|3)
        NODE_ID=$ROLE
        echo ""
        echo -e "${GREEN}Selected: Server Node $NODE_ID${NC}"
        echo ""
        
        # Get peer IPs
        echo "Step 3: Configure Peer Servers"
        echo "----------------------------------------------"
        echo "Enter the IP addresses of the OTHER 2 servers"
        echo "(This machine is $MY_IP)"
        echo ""
        
        if [ "$NODE_ID" -eq 1 ]; then
            read -p "Server 2 IP: " SERVER2_IP
            read -p "Server 3 IP: " SERVER3_IP
            PEER_ADDRESSES="${SERVER2_IP}:8002,${SERVER3_IP}:8003"
            MY_PORT="8001"
            SERVER1_IP=$MY_IP
        elif [ "$NODE_ID" -eq 2 ]; then
            read -p "Server 1 IP: " SERVER1_IP
            read -p "Server 3 IP: " SERVER3_IP
            PEER_ADDRESSES="${SERVER1_IP}:8001,${SERVER3_IP}:8003"
            MY_PORT="8002"
            SERVER2_IP=$MY_IP
        else
            read -p "Server 1 IP: " SERVER1_IP
            read -p "Server 2 IP: " SERVER2_IP
            PEER_ADDRESSES="${SERVER1_IP}:8001,${SERVER2_IP}:8002"
            MY_PORT="8003"
            SERVER3_IP=$MY_IP
        fi
        
        echo ""
        echo "Configuration Summary:"
        echo "----------------------------------------------"
        echo "  Role: Server Node $NODE_ID"
        echo "  Bind Address: 0.0.0.0:$MY_PORT"
        echo "  Peer Addresses: $PEER_ADDRESSES"
        echo ""
        
        read -p "Is this correct? (y/n): " CONFIRM
        if [[ ! "$CONFIRM" =~ ^[Yy]$ ]]; then
            echo "Setup cancelled."
            exit 0
        fi
        
        # Configure firewall
        echo ""
        echo "Step 4: Configuring firewall..."
        sudo ufw --force enable
        sudo ufw allow ssh
        sudo ufw allow 8001/udp
        sudo ufw allow 8002/udp
        sudo ufw allow 8003/udp
        echo -e "${GREEN}✓ Firewall configured${NC}"
        
        # Build project
        echo ""
        echo "Step 5: Building project..."
        cargo build --release
        echo -e "${GREEN}✓ Build complete${NC}"
        
        # Create start script
        cat > start_server.sh << EOF
#!/bin/bash
cd "$PWD"
cargo run --release --bin cloud-node $NODE_ID 0.0.0.0:$MY_PORT $PEER_ADDRESSES
EOF
        chmod +x start_server.sh
        
        # Create systemd service
        echo ""
        echo "Step 6: Creating systemd service..."
        sudo tee /etc/systemd/system/cloud-node${NODE_ID}.service > /dev/null << EOF
[Unit]
Description=Distributed Image Cloud - Node $NODE_ID
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$PWD
ExecStart=$HOME/.cargo/bin/cargo run --release --bin cloud-node $NODE_ID 0.0.0.0:$MY_PORT $PEER_ADDRESSES
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF
        
        sudo systemctl daemon-reload
        sudo systemctl enable cloud-node${NODE_ID}.service
        echo -e "${GREEN}✓ Systemd service created and enabled${NC}"
        
        # Summary
        echo ""
        echo "=============================================="
        echo -e "${GREEN}✓ SERVER NODE $NODE_ID SETUP COMPLETE${NC}"
        echo "=============================================="
        echo ""
        echo "To start the server:"
        echo "  Option 1 (systemd - recommended):"
        echo "    sudo systemctl start cloud-node${NODE_ID}.service"
        echo "    sudo systemctl status cloud-node${NODE_ID}.service"
        echo ""
        echo "  Option 2 (manual):"
        echo "    ./start_server.sh"
        echo ""
        echo "To view logs:"
        echo "    sudo journalctl -u cloud-node${NODE_ID}.service -f"
        echo ""
        echo "Next steps:"
        echo "  1. Run this script on the other 2 server machines"
        echo "  2. Start all 3 servers"
        echo "  3. Configure and start client machines"
        echo ""
        ;;
        
    4)
        echo ""
        echo -e "${GREEN}Selected: Client Machine${NC}"
        echo ""
        
        # Get server IPs
        echo "Step 3: Configure Server Addresses"
        echo "----------------------------------------------"
        echo "Enter the IP addresses of the 3 servers"
        echo ""
        read -p "Server 1 IP: " SERVER1_IP
        read -p "Server 2 IP: " SERVER2_IP
        read -p "Server 3 IP: " SERVER3_IP
        
        echo ""
        echo "Configuration Summary:"
        echo "----------------------------------------------"
        echo "  Role: Client Machine"
        echo "  Server 1: $SERVER1_IP:8001"
        echo "  Server 2: $SERVER2_IP:8002"
        echo "  Server 3: $SERVER3_IP:8003"
        echo ""
        
        read -p "Is this correct? (y/n): " CONFIRM
        if [[ ! "$CONFIRM" =~ ^[Yy]$ ]]; then
            echo "Setup cancelled."
            exit 0
        fi
        
        # Backup original file
        echo ""
        echo "Step 4: Backing up original configuration..."
        cp src/gui_client.rs src/gui_client.rs.backup
        echo -e "${GREEN}✓ Backup created: src/gui_client.rs.backup${NC}"
        
        # Update gui_client.rs
        echo ""
        echo "Step 5: Updating client configuration..."
        
        # Use sed to replace the cloud_addresses section
        sed -i "/cloud_addresses: vec!\[/,/\],/ {
            /cloud_addresses: vec!\[/n
            s|.*\".*:8001\".*|                \"${SERVER1_IP}:8001\".to_string(),|
            s|.*\".*:8002\".*|                \"${SERVER2_IP}:8002\".to_string(),|
            s|.*\".*:8003\".*|                \"${SERVER3_IP}:8003\".to_string(),|
        }" src/gui_client.rs
        
        echo -e "${GREEN}✓ Configuration updated${NC}"
        
        # Build project
        echo ""
        echo "Step 6: Building client..."
        cargo build --release
        echo -e "${GREEN}✓ Build complete${NC}"
        
        # Create start script
        cat > start_client.sh << EOF
#!/bin/bash
cd "$PWD"
cargo run --release --bin client-gui
EOF
        chmod +x start_client.sh
        
        # Summary
        echo ""
        echo "=============================================="
        echo -e "${GREEN}✓ CLIENT SETUP COMPLETE${NC}"
        echo "=============================================="
        echo ""
        echo "To start the client GUI:"
        echo "    ./start_client.sh"
        echo "    or"
        echo "    cargo run --release --bin client-gui"
        echo ""
        echo "Next steps:"
        echo "  1. Ensure all 3 servers are running"
        echo "  2. Start the client GUI"
        echo "  3. Login with a username"
        echo "  4. Start sharing images!"
        echo ""
        ;;
        
    5)
        echo "Exiting..."
        exit 0
        ;;
        
    *)
        echo -e "${RED}Invalid option${NC}"
        exit 1
        ;;
esac

echo "=============================================="
echo ""
