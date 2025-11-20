# Encryption Testing Guide

This directory contains scripts to test the new image steganography encryption.

## Quick Start

### Option 1: Automated Test (Recommended)

```bash
# Run the test script (it will guide you)
./test_encryption.sh
```

The script will:
1. Build the test binary if needed
2. Find or prompt for a test image
3. Run encryption/decryption tests
4. Show detailed results
5. Offer to open the images for visual comparison

### Option 2: Manual Test

```bash
# 1. Create a test image (requires Python + PIL)
python3 create_test_image.py

# 2. Build the test binary
cargo build --release --bin test_encryption

# 3. Run the test
./target/release/test_encryption test_input.jpg
```

### Option 3: Use Your Own Image

```bash
# Copy your image
cp /path/to/your/photo.jpg test_input.jpg

# Run the test
./test_encryption.sh
```

## What Gets Tested

The test performs:

1. **Encryption Test**
   - Reads original image
   - Encrypts with test metadata (alice, bob, charlie, quota=5)
   - Saves encrypted result as PNG
   - Measures time and size

2. **Decryption Test**
   - Decrypts the encrypted image
   - Extracts metadata (usernames, quota)
   - Saves decrypted result
   - Measures time

3. **Verification**
   - Compares metadata (usernames and quota)
   - Byte-by-byte comparison of original vs decrypted
   - Reports any differences

4. **Performance Metrics**
   - Encryption speed (MB/s)
   - Decryption speed (MB/s)
   - File size changes

## Expected Results

### Successful Test Output:

```
==============================================
  ✅ ALL TESTS PASSED
  • Encryption successful
  • Decryption successful
  • Metadata preserved
  • Data integrity verified (perfect match)
==============================================

📁 Generated files:
   • test_input.jpg - Original input
   • test_encrypted.png - Encrypted (looks like gradient)
   • test_decrypted.jpg - Decrypted result
```

### Visual Verification:

- **test_input.jpg**: Your original image
- **test_encrypted.png**: Should look like a **colorful gradient pattern** (not the original!)
- **test_decrypted.jpg**: Should look **exactly like** test_input.jpg

## Understanding the Results

### File Sizes:
```
test_input.jpg:     50 KB  (original JPEG)
test_encrypted.png: 1.4 MB (PNG with hidden data)
test_decrypted.jpg: 50 KB  (restored original)
```

**Why is encrypted larger?**
- Encrypted file is PNG (lossless) to preserve LSB data
- Cover image dimensions are at least 800×600
- Cover must be large enough to hide the original

### Performance:
```
Encryption: 25.50ms (1.96 MB/s)
Decryption: 18.23ms (2.74 MB/s)
Total time: 43.73ms
```

Typical speeds:
- **Encryption**: 1-3 MB/s (generates cover + embeds data)
- **Decryption**: 2-5 MB/s (extracts from LSBs)

## Test Files Created

| File | Description | Purpose |
|------|-------------|---------|
| `src/bin/test_encryption.rs` | Rust test binary | Performs encryption/decryption |
| `test_encryption.sh` | Shell script | Automates testing |
| `create_test_image.py` | Python script | Generates test images |
| `test_input.jpg` | Test image | Input for encryption |
| `test_encrypted.png` | Result | Encrypted (gradient cover) |
| `test_decrypted.jpg` | Result | Decrypted (restored) |

## Troubleshooting

### Error: "Input file not found"

**Solution:**
```bash
# Create a test image
python3 create_test_image.py

# Or copy an existing image
cp ~/Pictures/photo.jpg test_input.jpg
```

### Error: "Image too small"

**Cause:** Original image is very large, cover image isn't big enough

**Solution:**
The cover image is automatically sized to fit. If this happens:
1. Use a smaller/compressed input image
2. Or modify `generate_cover_image()` in encryption.rs to use larger dimensions

### Error: "Build failed"

**Solution:**
```bash
# Clean and rebuild
cargo clean
cargo build --release --bin test_encryption
```

### Error: "PIL/Pillow not found" (Python script)

**Solution:**
```bash
# Install Pillow
pip3 install Pillow

# Or use system package manager
sudo apt install python3-pil  # Ubuntu/Debian
```

### Test passes but images look wrong

**Check:**
1. Open test_encrypted.png - should be gradient, NOT original image
2. Open test_decrypted.jpg - should match test_input.jpg
3. If decrypted doesn't match, check console for byte differences

## Advanced Testing

### Test with Different Image Sizes:

```bash
# Small image (fast test)
python3 create_test_image.py test_small.jpg 400 300
./target/release/test_encryption test_small.jpg

# Large image (stress test)
python3 create_test_image.py test_large.jpg 1920 1080
./target/release/test_encryption test_large.jpg
```

### Test with Different Formats:

```bash
# Test PNG input
convert test_input.jpg test_input.png
./target/release/test_encryption test_input.png

# Test with actual photos
./target/release/test_encryption ~/Pictures/vacation.jpg
```

### Performance Benchmarking:

```bash
# Run multiple times and average
for i in {1..10}; do
    ./target/release/test_encryption test_input.jpg 2>&1 | grep "Total time"
done
```

### Test with Minimal Logging:

```bash
# Quiet mode (less output)
RUST_LOG=error ./target/release/test_encryption test_input.jpg

# Verbose mode (debug info)
RUST_LOG=debug ./target/release/test_encryption test_input.jpg
```

## Integration with System

### Running from Anywhere:

```bash
# Add to PATH (optional)
export PATH="$PATH:/media/kirollos/Data/Distributed Systems/Cloud Project"

# Then run from anywhere
cd ~
test_encryption.sh
```

### Automated Testing in CI:

```yaml
# Example GitHub Actions
- name: Test Encryption
  run: |
    python3 create_test_image.py
    cargo test --release --bin test_encryption
```

## Cleanup

Remove test files:
```bash
rm test_input.jpg test_encrypted.png test_decrypted.jpg
```

Keep test binary for future use:
```bash
# Binary is in: target/release/test_encryption
# Reuse it without rebuilding
```

## Success Criteria

✅ **Test passes if:**
- Encryption completes without errors
- Decryption completes without errors
- Metadata matches (usernames + quota)
- **All bytes match** between original and decrypted (0 differences)
- Encrypted image looks like gradient (visually)
- Decrypted image looks like original (visually)

❌ **Test fails if:**
- Any operation errors
- Metadata doesn't match
- Byte differences found
- Encrypted image looks like original (steganography failed)
- Decrypted image doesn't match original

## Next Steps

After testing passes:

1. ✅ Encryption method is working correctly
2. ✅ Safe to use in the distributed system
3. ✅ Can now test with actual nodes and clients
4. ✅ Ready for stress testing with `multiple_img_testing/`

## Support

If tests fail consistently:
1. Check `NEW_ENCRYPTION_METHOD.md` for implementation details
2. Enable debug logging: `RUST_LOG=debug`
3. Verify you rebuilt after code changes: `cargo build --release`
4. Check if original image is valid (can be opened normally)
