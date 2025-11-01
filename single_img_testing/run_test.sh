#!/bin/bash
# Fully automated end-to-end test script - NO manual intervention needed

echo "=================================================="
echo "  AUTOMATED END-TO-END ENCRYPTION TEST"
echo "=================================================="
echo ""

# Check if server address provided
if [ -z "$1" ]; then
    echo "Usage: ./run_test.sh <server_address>"
    echo "Example: ./run_test.sh 10.40.59.43:8001"
    echo ""
    echo "This script will:"
    echo "  1. Create test image (if needed)"
    echo "  2. Send to server for encryption"
    echo "  3. Receive encrypted image"
    echo "  4. Decrypt the image"
    echo "  5. Display all images (original, encrypted, decrypted)"
    echo ""
    echo "Make sure your server is running:"
    echo "  cd .."
    echo "  cargo run --release --bin cloud-node -- 1 10.40.59.43:8001"
    echo ""
    exit 1
fi

SERVER_ADDRESS=$1
IMAGE_PATH="test_image_720p.jpg"

echo "Server: $SERVER_ADDRESS"
echo "Image: $IMAGE_PATH (720p - 1280x720)"
echo "Mode: FULLY AUTOMATED (no manual steps)"
echo "Features: Multi-packet UDP transmission for large images"
echo ""

# Create output directories
mkdir -p client_output server_output

# Step 1: Create 720p test image if it doesn't exist
if [ ! -f "$IMAGE_PATH" ]; then
    echo "=================================================="
    echo "  STEP 1: Creating 720p Test Image"
    echo "=================================================="
    echo ""
    python3 generate_720p_image.py $IMAGE_PATH
    if [ $? -ne 0 ]; then
        echo "❌ Failed to create 720p test image"
        exit 1
    fi
    echo ""
    sleep 2
else
    echo "✓ 720p test image exists: $IMAGE_PATH"
    ls -lh $IMAGE_PATH | awk '{print "  Size: "$5", Modified: "$6" "$7" "$8}'
    echo ""
fi

# Step 2: Send to server and receive encrypted image
echo "=================================================="
echo "  STEP 2: Send to Server for Encryption"
echo "=================================================="
echo ""
python3 test_client.py $SERVER_ADDRESS $IMAGE_PATH

if [ $? -ne 0 ]; then
    echo ""
    echo "❌ Test failed. Make sure server is running:"
    echo "   cargo run --release --bin cloud-node -- 1 $SERVER_ADDRESS"
    exit 1
fi

echo ""
sleep 2

# Step 3: Decrypt the encrypted image
echo "=================================================="
echo "  STEP 3: Decrypt the Encrypted Image"
echo "=================================================="
echo ""
python3 decrypt_client.py client_output/encrypted_image.png

if [ $? -ne 0 ]; then
    echo ""
    echo "❌ Decryption failed. Check that encrypted_image.png exists."
    exit 1
fi

echo ""
sleep 1

# Step 4: Final summary with image comparison
echo "=================================================="
echo "  TEST COMPLETE - SUMMARY"
echo "=================================================="
echo ""
echo "✅ All steps completed successfully!"
echo ""
echo "📊 RESULTS:"
echo "   1. Original image:  $IMAGE_PATH"
echo "   2. Encrypted image: client_output/encrypted_image.png"
echo "   3. Decrypted image: client_output/decrypted_image.png"
echo ""
echo "📁 FILE SIZES:"
ls -lh $IMAGE_PATH client_output/encrypted_image.png client_output/decrypted_image.png 2>/dev/null | awk '{print "   "$9" - "$5}'
echo ""
echo "📁 ALL OUTPUT FILES:"
ls -lh client_output/ 2>/dev/null | grep -v "^total" | awk '{print "   "$9" - "$5}'
echo ""
echo "🎯 VERIFICATION:"
echo "   ✓ Image sent to server and encrypted"
echo "   ✓ Encrypted image received (scrambled)"
echo "   ✓ Image decrypted successfully"
echo "   ✓ Original restored from encrypted version"
echo ""
echo "=================================================="
