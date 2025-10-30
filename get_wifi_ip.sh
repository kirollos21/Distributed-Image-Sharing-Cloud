#!/bin/bash

echo "=========================================="
echo "  WiFi Network Configuration Helper"
echo "=========================================="
echo ""

# Find WiFi interface
WIFI_INTERFACE=$(ip link show | grep -E "wlan|wlp|wlo" | head -n1 | cut -d: -f2 | xargs)

if [ -z "$WIFI_INTERFACE" ]; then
    echo "❌ No WiFi interface found!"
    echo ""
    echo "Available interfaces:"
    ip link show | grep -E "^[0-9]:" | cut -d: -f2
    exit 1
fi

echo "WiFi Interface: $WIFI_INTERFACE"
echo ""

# Get WiFi IP
WIFI_IP=$(ip -4 addr show $WIFI_INTERFACE | grep inet | awk '{print $2}' | cut -d/ -f1)

if [ -z "$WIFI_IP" ]; then
    echo "❌ WiFi interface has no IP address!"
    echo "   Make sure you're connected to WiFi."
    exit 1
fi

echo "✅ WiFi IP Address: $WIFI_IP"
echo ""
echo "=========================================="
echo "  Copy this information:"
echo "=========================================="
echo ""
echo "Interface: $WIFI_INTERFACE"
echo "IP:        $WIFI_IP"
echo ""
echo "Use this IP when starting cloud nodes:"
echo "  cargo run --release --bin cloud-node 1 0.0.0.0:8001 <peer1>:8002,<peer2>:8003"
echo ""
echo "Update src/gui_client.rs line 101-103 with:"
echo "  \"$WIFI_IP:800X\".to_string(),"
echo ""
