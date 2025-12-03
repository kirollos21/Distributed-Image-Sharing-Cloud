#!/bin/bash

# Script to test the new image steganography encryption

PROJECT_DIR="/media/kirollos/Data/Distributed Systems/Cloud Project"
cd "$PROJECT_DIR"

echo "=========================================="
echo "  Encryption Test Script"
echo "=========================================="
echo ""

# Check if test binary exists, build if not
if [ ! -f "target/release/test_encryption" ]; then
    echo "🔨 Building test binary..."
    cargo build --release --bin test_encryption
    if [ $? -ne 0 ]; then
        echo "❌ Build failed!"
        exit 1
    fi
    echo "✅ Build complete!"
    echo ""
fi

# Check for test image
if [ ! -f "test_input.jpg" ] && [ ! -f "test_input.png" ]; then
    echo "⚠️  No test image found!"
    echo ""
    echo "Please provide a test image in one of these ways:"
    echo ""
    echo "Option 1: Copy an image to test_input.jpg"
    echo "  cp /path/to/your/image.jpg test_input.jpg"
    echo ""
    echo "Option 2: Create a simple test image"
    echo "  convert -size 400x300 gradient:red-blue test_input.jpg"
    echo "  (requires ImageMagick)"
    echo ""
    echo "Option 3: Download a test image"
    echo "  wget https://via.placeholder.com/400x300.jpg -O test_input.jpg"
    echo ""
    echo "Option 4: Use any image from your system"
    echo "  cp ~/Pictures/some_photo.jpg test_input.jpg"
    echo ""

    # Try to find an image automatically
    echo "🔍 Searching for images in common locations..."

    FOUND_IMAGE=""

    # Check Pictures directory
    if [ -d "$HOME/Pictures" ]; then
        FOUND_IMAGE=$(find "$HOME/Pictures" -maxdepth 2 -type f \( -iname "*.jpg" -o -iname "*.jpeg" -o -iname "*.png" \) 2>/dev/null | head -n 1)
    fi

    # Check Downloads directory
    if [ -z "$FOUND_IMAGE" ] && [ -d "$HOME/Downloads" ]; then
        FOUND_IMAGE=$(find "$HOME/Downloads" -maxdepth 1 -type f \( -iname "*.jpg" -o -iname "*.jpeg" -o -iname "*.png" \) 2>/dev/null | head -n 1)
    fi

    if [ -n "$FOUND_IMAGE" ]; then
        echo "   Found: $FOUND_IMAGE"
        echo ""
        read -p "   Use this image for testing? (y/n): " -n 1 -r
        echo ""
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            cp "$FOUND_IMAGE" test_input.jpg
            echo "   ✓ Copied to test_input.jpg"
            echo ""
        else
            echo "   Skipped. Please provide test_input.jpg manually."
            exit 1
        fi
    else
        echo "   No images found automatically."
        echo ""
        echo "Please create test_input.jpg manually and run again."
        exit 1
    fi
fi

# Determine which test image to use
TEST_IMAGE=""
if [ -f "test_input.jpg" ]; then
    TEST_IMAGE="test_input.jpg"
elif [ -f "test_input.png" ]; then
    TEST_IMAGE="test_input.png"
fi

echo "🧪 Running encryption test..."
echo ""

# Run the test with logging
export RUST_LOG=info
./target/release/test_encryption "$TEST_IMAGE"

EXIT_CODE=$?

echo ""

if [ $EXIT_CODE -eq 0 ]; then
    echo "=========================================="
    echo "  ✅ Test completed successfully!"
    echo "=========================================="
    echo ""

    # Show file sizes
    if [ -f "test_encrypted.png" ] && [ -f "test_decrypted.jpg" ]; then
        echo "📊 File size comparison:"
        ls -lh "$TEST_IMAGE" test_encrypted.png test_decrypted.jpg | awk '{print "   "$9": "$5}'
        echo ""

        echo "🖼️  View the results:"
        echo "   1. Original:  $TEST_IMAGE"
        echo "   2. Encrypted: test_encrypted.png (should look like gradient)"
        echo "   3. Decrypted: test_decrypted.jpg (should match original)"
        echo ""

        # Try to open images if display tools available
        if command -v eog &> /dev/null; then
            read -p "Open images in viewer? (y/n): " -n 1 -r
            echo ""
            if [[ $REPLY =~ ^[Yy]$ ]]; then
                eog "$TEST_IMAGE" test_encrypted.png test_decrypted.jpg &
            fi
        elif command -v xdg-open &> /dev/null; then
            read -p "Open images? (y/n): " -n 1 -r
            echo ""
            if [[ $REPLY =~ ^[Yy]$ ]]; then
                xdg-open "$TEST_IMAGE" &
                xdg-open test_encrypted.png &
                xdg-open test_decrypted.jpg &
            fi
        fi
    fi
else
    echo "=========================================="
    echo "  ❌ Test failed!"
    echo "=========================================="
    echo ""
    echo "Check the error messages above for details."
fi

echo ""
echo "🧹 Cleanup:"
echo "   To remove test files: rm test_encrypted.png test_decrypted.jpg"
echo ""
