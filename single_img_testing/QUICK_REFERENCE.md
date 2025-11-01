# Quick Reference Guide - Single Image Testing

## 🚀 Fastest Way to Test

```bash
cd single_img_testing

# 1. Start server (separate terminal)
cd ..
cargo run --release --bin cloud-node -- 1 10.40.59.43:8001

# 2. Run complete test (this terminal)
./run_test.sh 10.40.59.43:8001
```

## 📋 What Each Script Does

### `test_client.py` - Main Test Client
**Purpose**: Send image to server, receive encrypted version

**Usage**:
```bash
python3 test_client.py <server:port> <image_path>
python3 test_client.py 10.40.59.43:8001 ../test_image.jpg
```

**What happens**:
1. 📷 Displays original image
2. 📤 Sends to server
3. 🔒 Receives encrypted image (looks scrambled)
4. 💾 Saves to `client_output/encrypted_image.jpg`

**Output**:
- Shows: Original image, Encrypted image
- Saves: `client_output/encrypted_image.jpg`

---

### `decrypt_client.py` - Decryption Tool
**Purpose**: Decrypt encrypted images

**Usage**:
```bash
python3 decrypt_client.py <encrypted_image_path>
python3 decrypt_client.py client_output/encrypted_image.jpg
```

**What happens**:
1. 🔒 Displays encrypted image (scrambled)
2. 🔍 Extracts metadata from LSBs
3. 🔑 Calculates seed from metadata
4. 🔄 Unscrambles pixels
5. 🔓 Displays decrypted image (original restored!)
6. 💾 Saves to `client_output/decrypted_image.png`

**Output**:
- Shows: Encrypted image, Decrypted image
- Saves: `client_output/decrypted_image.png`

---

### `server_viewer.py` - Server Simulator
**Purpose**: View images as server would see them

**Usage**:
```bash
python3 server_viewer.py <image_path>
python3 server_viewer.py ../test_image.jpg
```

**What happens**:
1. 📷 Displays image as server sees it
2. 💾 Saves to `server_output/server_received_original.jpg`
3. Shows what encryption process would do

**Note**: This is a simulation. Real encryption happens in Rust server.

---

### `create_test_image.py` - Test Image Generator
**Purpose**: Create a colorful test image

**Usage**:
```bash
python3 create_test_image.py [filename]
python3 create_test_image.py test_image.jpg
```

**What happens**:
- Creates 640x480 image with shapes and gradients
- Saves as JPEG
- Displays the created image

---

### `run_test.sh` - Automated Test Script
**Purpose**: Run complete test flow automatically

**Usage**:
```bash
./run_test.sh <server_address>
./run_test.sh 10.40.59.43:8001
```

**What happens**:
1. Checks for test image (creates if missing)
2. Optionally shows server view
3. Runs test_client.py
4. Runs decrypt_client.py
5. Shows summary

---

## 🎯 Common Use Cases

### Case 1: Quick Test
```bash
# Assuming server is running
./run_test.sh 10.40.59.43:8001
```

### Case 2: Manual Step-by-Step
```bash
# Step 1: Send and encrypt
python3 test_client.py 10.40.59.43:8001 ../test_image.jpg

# Step 2: Decrypt
python3 decrypt_client.py client_output/encrypted_image.jpg
```

### Case 3: Server View Only
```bash
python3 server_viewer.py ../test_image.jpg
```

### Case 4: Create Custom Test Image
```bash
python3 create_test_image.py my_test.jpg
python3 test_client.py 10.40.59.43:8001 my_test.jpg
```

### Case 5: Test Multiple Servers
```bash
# Node 1
python3 test_client.py 10.40.59.43:8001 ../test_image.jpg

# Node 2
python3 test_client.py 10.40.44.230:8002 ../test_image.jpg

# Node 3
python3 test_client.py 10.40.33.156:8003 ../test_image.jpg
```

---

## 📁 Output Files

### Client Output (`client_output/`)
```
client_output/
├── encrypted_image.jpg       # Encrypted (scrambled) image from server
├── decrypted_image.png       # Decrypted (restored) image
└── decryption_note.txt       # Information about decryption
```

### Server Output (`server_output/`)
```
server_output/
└── server_received_original.jpg  # Copy of original (server view)
```

---

## 🔍 What You'll See

### Original Image
- Clear, readable
- Normal colors
- **Shown by**: test_client.py (step 1), server_viewer.py

### Encrypted Image
- Scrambled pixels
- Looks like TV static/noise
- **Shown by**: test_client.py (step 3), decrypt_client.py (step 1)

### Decrypted Image
- Original restored
- Identical to original (pixel-perfect)
- **Shown by**: decrypt_client.py (step 5)

---

## 🛠 Troubleshooting

### Server not responding
```bash
# Check server is running
ps aux | grep cloud-node

# Start server
cd ..
cargo run --release --bin cloud-node -- 1 10.40.59.43:8001
```

### Missing test image
```bash
# Create one
python3 create_test_image.py ../test_image.jpg

# Or use your own
python3 test_client.py 10.40.59.43:8001 /path/to/your/image.jpg
```

### Images not displaying
```bash
# If on headless server, check saved files
ls -lh client_output/
eog client_output/*.jpg  # Or use your image viewer
```

### Decryption fails
```bash
# Re-run test_client.py to get fresh encrypted image
python3 test_client.py 10.40.59.43:8001 ../test_image.jpg

# Then try decrypting again
python3 decrypt_client.py client_output/encrypted_image.jpg
```

---

## 📊 Expected Output

### test_client.py
```
STEP 1: Loading Original Image
✓ Loaded image: ../test_image.jpg
✓ Size: 23456 bytes (22.91 KB)
📷 Opening original image for viewing...

STEP 2: Sending to Server for Encryption
✓ Request ID: test_img_1730505600
📤 Sending to server 10.40.59.43:8001...
✓ Response received in 0.78s

STEP 3: Processing Encrypted Image
✓ Encrypted image received: 28934 bytes (28.26 KB)
🔒 Opening ENCRYPTED image for viewing...

TEST COMPLETE - SUMMARY
✓ Original image loaded and displayed
✓ Encrypted image received (28.26 KB)
✓ Processing time: 0.78s
```

### decrypt_client.py
```
Extracting Hidden Metadata from LSBs
✓ Metadata length: 56 bytes
✓ Extracted metadata:
   - Usernames: ['alice', 'bob', 'charlie']
   - Quota: 5

Calculating Seed for Unscrambling
✓ Seed calculated: 12345678901234567890

Unscrambling Pixels (Reverse Fisher-Yates)
✓ Generated 307199 swap operations
✓ Pixels unscrambled successfully

DECRYPTION COMPLETE - SUMMARY
✓ Original image restored!
```

---

## 💡 Tips

1. **Close image windows** to continue script execution
2. **Use ./run_test.sh** for automated testing
3. **Check client_output/** for saved files
4. **Compare images** side-by-side to verify
5. **Test different servers** to verify load balancing

---

## 🔗 Related Files

- **Full documentation**: `README.md`
- **Technical details**: `../REQUEST_FLOW_DETAILED.md`
- **Encryption code**: `../src/encryption.rs`
- **Server code**: `../src/node.rs`

---

## ⚡ One-Line Commands

```bash
# Complete test
./run_test.sh 10.40.59.43:8001

# Just encryption
python3 test_client.py 10.40.59.43:8001 ../test_image.jpg

# Just decryption
python3 decrypt_client.py client_output/encrypted_image.jpg

# Create test image
python3 create_test_image.py ../test_image.jpg

# Server view
python3 server_viewer.py ../test_image.jpg
```
