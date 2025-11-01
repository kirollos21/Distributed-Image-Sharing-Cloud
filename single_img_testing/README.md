# Single Image Testing Suite

This folder contains tools for testing the complete image encryption/decryption flow with visual feedback at both client and server sides.

## Overview

This testing suite demonstrates:
1. ✅ **Original image viewing** (before encryption)
2. ✅ **Server-side encryption** (with before/after viewing)
3. ✅ **Encrypted image transfer** (client ↔ server)
4. ✅ **Client-side decryption** (with viewing)
5. ✅ **Visual comparison** (original vs encrypted vs decrypted)

## Files

### Client Side
- **`test_client.py`** - Main test client that sends image and receives encrypted version
- **`decrypt_client.py`** - Decrypts encrypted images and restores original
- **`client_output/`** - Directory for client-side output files

### Server Side
- **`server_viewer.py`** - Simulates server-side image viewing (before encryption)
- **`server_output/`** - Directory for server-side output files

### Supporting Files
- **`README.md`** - This file
- **`create_test_image.py`** - Creates a test image (optional)

## Prerequisites

```bash
# Install Python dependencies
pip3 install Pillow

# Ensure your Rust server is compiled
cd ..
cargo build --release --bin cloud-node
```

## Quick Start

### Option 1: Complete End-to-End Test

```bash
# 1. Start your Rust server (in separate terminal)
cd ..
cargo run --release --bin cloud-node -- 1 10.40.59.43:8001

# 2. Run client test (this terminal)
cd single_img_testing
python3 test_client.py 10.40.59.43:8001 ../test_image.jpg

# 3. Decrypt the received image
python3 decrypt_client.py client_output/encrypted_image.jpg
```

### Option 2: Server-Side Viewing Only

```bash
# View how server sees the image before encryption
python3 server_viewer.py ../test_image.jpg
```

## Detailed Usage

### 1. Server Viewer (Optional - Simulation Only)

Shows how the server would view images before encryption:

```bash
python3 server_viewer.py <image_path>

# Example
python3 server_viewer.py ../test_image.jpg
```

**What it does:**
- Loads the original image
- Displays it (as server would see it)
- Saves copy to `server_output/`
- Shows what encryption process would do

**Note**: This is a simulation. Real encryption happens in Rust server.

### 2. Test Client (Main Test)

Sends image to server, receives encrypted version:

```bash
python3 test_client.py <server_address> <image_path>

# Example
python3 test_client.py 10.40.59.43:8001 ../test_image.jpg
```

**What it does:**
1. 📷 **Loads and displays original image**
2. 📤 **Sends image to Rust server for encryption**
   - Creates EncryptionRequest with usernames and quota
   - Sends via UDP to server
3. ⏳ **Waits for server response** (encryption happens server-side)
4. 📥 **Receives encrypted image**
5. 🔒 **Displays encrypted image** (should look scrambled/noise)
6. 💾 **Saves encrypted image** to `client_output/encrypted_image.jpg`

**Output files:**
```
client_output/
├── encrypted_image.jpg      # Encrypted (scrambled) image
└── decryption_note.txt      # Information about decryption
```

### 3. Decryption Client

Decrypts the encrypted image received from server:

```bash
python3 decrypt_client.py <encrypted_image_path>

# Example (after running test_client.py)
python3 decrypt_client.py client_output/encrypted_image.jpg
```

**What it does:**
1. 🔒 **Loads encrypted image** and displays it
2. 🔍 **Extracts hidden metadata** from pixel LSBs
   - Reads metadata length (first 32 bits)
   - Extracts metadata JSON (usernames + quota)
3. 🔑 **Calculates seed** from metadata hash
4. 🔄 **Unscrambles pixels** using reverse Fisher-Yates shuffle
5. 🔓 **Displays decrypted image** (original restored!)
6. 💾 **Saves decrypted image** to `client_output/decrypted_image.png`

**Output files:**
```
client_output/
├── encrypted_image.jpg      # From test_client.py
├── decrypted_image.png      # Restored original image
└── decryption_note.txt      # Decryption information
```

## Complete Testing Flow

### Full Workflow Example

```bash
# Terminal 1: Start Rust server
cd /home/khoulykid@auc.egy/Desktop/Distributed-Image-Sharing-Cloud
cargo run --release --bin cloud-node -- 1 10.40.59.43:8001

# Terminal 2: Run tests
cd single_img_testing

# Step 1: View server-side (optional)
python3 server_viewer.py ../test_image.jpg
# → Opens original image (as server sees it)

# Step 2: Send to server for encryption
python3 test_client.py 10.40.59.43:8001 ../test_image.jpg
# → Opens original image (client side)
# → Opens encrypted image (received from server - looks scrambled)

# Step 3: Decrypt the encrypted image
python3 decrypt_client.py client_output/encrypted_image.jpg
# → Opens encrypted image (before decryption)
# → Opens decrypted image (original restored!)
```

## What You'll See

### 1. Original Image (Before Encryption)
- Clear, readable image
- Normal colors and structure
- **Shown by**: `test_client.py` (step 1), `server_viewer.py`

### 2. Encrypted Image (After Server Processing)
- Looks like TV static/noise
- Pixels completely scrambled
- Contains hidden metadata (invisible)
- **Shown by**: `test_client.py` (step 3), `decrypt_client.py` (step 1)

### 3. Decrypted Image (After Client Decryption)
- Original image restored perfectly
- All colors and structure recovered
- Pixel-perfect match with original
- **Shown by**: `decrypt_client.py` (step 5)

## Technical Details

### Encryption Process (Server Side - Rust)

```
Original Image (20KB JPEG)
        ↓
1. Decode to RGBA pixels (1.2MB)
        ↓
2. Embed metadata in LSBs
   - Usernames: ["alice", "bob"]
   - Quota: 5
   (Invisible - changes pixels by ±1)
        ↓
3. Scramble pixels (Fisher-Yates)
   - Seed = hash(usernames + quota)
   - Deterministic shuffle
        ↓
4. Re-encode as JPEG (quality 60)
        ↓
Encrypted Image (25KB JPEG)
- Looks like noise
- Has hidden metadata
```

### Decryption Process (Client Side - Python)

```
Encrypted Image (25KB JPEG)
        ↓
1. Decode to RGBA pixels
        ↓
2. Extract metadata from LSBs
   - Read length (32 bits)
   - Read JSON (N bits)
   - Parse usernames + quota
        ↓
3. Calculate seed = hash(metadata)
        ↓
4. Unscramble pixels (reverse Fisher-Yates)
   - Same seed → reverse shuffle
        ↓
5. Re-encode as PNG
        ↓
Decrypted Image (original restored)
```

## Encryption Explained

### LSB Steganography (Hidden Metadata)

Each pixel byte has 8 bits. We modify only the **Least Significant Bit** (LSB):

```
Original pixel: 11010110 (214)
Our bit:        1
Result:         11010111 (215)  ← Changed by 1 (invisible to eye)

Original pixel: 11010110 (214)
Our bit:        0
Result:         11010110 (214)  ← No change
```

**Embedded data:**
```json
{
  "usernames": ["alice", "bob", "charlie"],
  "quota": 5
}
```

This is hidden in the first ~200-500 pixels (depending on metadata size).

### Pixel Scrambling (Visual Encryption)

**Before scrambling:**
```
Pixel 0: Red(255), Green(100), Blue(50)
Pixel 1: Red(128), Green(200), Blue(75)
Pixel 2: Red(64), Green(150), Blue(100)
...
```

**After scrambling (Fisher-Yates with seed):**
```
Pixel 0: Red(64), Green(150), Blue(100)   ← Was pixel 2
Pixel 1: Red(255), Green(100), Blue(50)   ← Was pixel 0
Pixel 2: Red(128), Green(200), Blue(75)   ← Was pixel 1
...
```

Result: Image looks like **random noise** but is **reversible** with the correct seed.

## Troubleshooting

### "Server not responding"
```bash
# Check if server is running
ps aux | grep cloud-node

# Start server if not running
cd ..
cargo run --release --bin cloud-node -- 1 10.40.59.43:8001
```

### "Connection refused"
```bash
# Check server address is correct
# Format: <ip>:<port>
# Example: 10.40.59.43:8001

# Verify server is listening on that port
netstat -tulpn | grep 8001
```

### "Image too large for UDP"
```bash
# Server automatically resizes images
# But if original is huge (>5MB), pre-resize it:
python3 -c "
from PIL import Image
img = Image.open('huge_image.jpg')
img.thumbnail((1024, 1024))
img.save('resized.jpg')
"
```

### Images not displaying (headless server)
```bash
# If running on headless server, images won't open
# Check the saved files instead:
ls -lh client_output/
ls -lh server_output/

# Or download files to local machine:
scp user@server:~/path/client_output/*.jpg .
```

### Decryption fails "Invalid metadata length"
```bash
# This means the encrypted image is corrupted
# Possible causes:
# 1. UDP packet loss (rare)
# 2. File was modified after encryption
# 3. Wrong file (not actually encrypted)

# Solution: Re-run test_client.py to get fresh encrypted image
```

## Example Output

### test_client.py output:
```
======================================================================
STEP 1: Loading Original Image
======================================================================
✓ Loaded image: ../test_image.jpg
✓ Size: 23456 bytes (22.91 KB)
✓ Dimensions: 640x480
✓ Format: JPEG

📷 Opening original image for viewing...

======================================================================
STEP 2: Sending to Server for Encryption
======================================================================
✓ Request ID: test_img_1730505600
✓ Target users: ['alice', 'bob', 'charlie']
✓ Quota: 5
✓ Message size: 31457 bytes (30.72 KB)

📤 Sending to server 10.40.59.43:8001...
✓ Request sent, waiting for response...
✓ Response received in 0.78s
✓ Response size: 28934 bytes (28.26 KB)

======================================================================
STEP 3: Processing Encrypted Image
======================================================================
✓ Encrypted image received: 28934 bytes (28.26 KB)
✓ Saved encrypted image to: client_output/encrypted_image.jpg
✓ Encrypted dimensions: 640x480

🔒 Opening ENCRYPTED image for viewing...
   (Should look like scrambled noise)

======================================================================
TEST COMPLETE - SUMMARY
======================================================================
✓ Original image loaded and displayed
✓ Image sent to server (22.91 KB)
✓ Encrypted image received (28.26 KB)
✓ Processing time: 0.78s
✓ Encrypted image displayed (scrambled)
✓ Files saved to: client_output/
```

### decrypt_client.py output:
```
======================================================================
Extracting Hidden Metadata from LSBs
======================================================================
✓ Metadata length: 56 bytes
✓ Extracted metadata:
   - Usernames: ['alice', 'bob', 'charlie']
   - Quota: 5

======================================================================
Calculating Seed for Unscrambling
======================================================================
✓ Seed calculated: 12345678901234567890
   (Derived from usernames + quota hash)

======================================================================
Unscrambling Pixels (Reverse Fisher-Yates)
======================================================================
✓ Generated 307199 swap operations
✓ Pixels unscrambled successfully

======================================================================
DECRYPTION COMPLETE - SUMMARY
======================================================================
✓ Extracted metadata from LSBs:
   - Authorized users: ['alice', 'bob', 'charlie']
   - Remaining views: 5
✓ Calculated seed from metadata: 12345678901234567890
✓ Unscrambled 1228800 pixel bytes
✓ Original image restored!
```

## Advanced Usage

### Testing with Different Parameters

```bash
# Different target users
python3 test_client.py 10.40.59.43:8001 image.jpg
# (Modify test_client.py to change usernames/quota)

# Different server nodes
python3 test_client.py 10.40.44.230:8002 image.jpg
python3 test_client.py 10.40.33.156:8003 image.jpg

# Larger images
python3 test_client.py 10.40.59.43:8001 big_image.jpg
# (Server will automatically resize if needed)
```

### Comparing Original vs Decrypted

```bash
# After running both test_client.py and decrypt_client.py:

# View side-by-side
eog ../test_image.jpg client_output/decrypted_image.png &

# Compare file sizes
ls -lh ../test_image.jpg client_output/encrypted_image.jpg client_output/decrypted_image.png

# Check pixel differences (should be minimal due to JPEG compression)
python3 -c "
from PIL import Image
import numpy as np

orig = np.array(Image.open('../test_image.jpg'))
decr = np.array(Image.open('client_output/decrypted_image.png').convert('RGB'))

# Resize if dimensions differ
if orig.shape != decr.shape:
    print('Note: Dimensions differ due to server resizing')
else:
    diff = np.abs(orig.astype(float) - decr.astype(float)).mean()
    print(f'Average pixel difference: {diff:.2f} (0 = perfect match)')
"
```

## Integration with Main System

This testing suite uses the same encryption/decryption logic as the main distributed system:

- **Server encryption**: `src/encryption.rs` - `encrypt_image()`
- **Client decryption**: `decrypt_client.py` (Python port of Rust logic)

The Rust server handles encryption automatically when receiving `EncryptionRequest` messages.

## Notes

- 📸 **Image windows auto-open**: Close them to continue execution
- 🔄 **Repeatable**: Run tests multiple times - uses timestamps for unique IDs
- 💾 **Output preserved**: Files saved to `client_output/` and `server_output/`
- 🌐 **Works remotely**: Can test across different machines on network
- 🔒 **Secure**: Metadata hidden, pixels scrambled, reversible only with correct seed

## Credits

Created for testing the Distributed Image Sharing Cloud encryption system.
See `../REQUEST_FLOW_DETAILED.md` for complete technical documentation.
