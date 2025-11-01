#!/bin/bash
# ULTRA SIMPLE - Just run this!

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║  SINGLE IMAGE ENCRYPTION TEST - ULTRA SIMPLE                   ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""

if [ -z "$1" ]; then
    echo "Usage: ./simple_run.sh <server_ip:port>"
    echo ""
    echo "Example:"
    echo "  ./simple_run.sh 10.40.59.43:8001"
    echo ""
    echo "What this does:"
    echo "  1. Sends image to server"
    echo "  2. Gets encrypted image back"
    echo "  3. Decrypts it"
    echo "  4. Opens 3 windows: Original, Encrypted, Decrypted"
    echo ""
    exit 1
fi

./run_test.sh $1
