# New Encryption Method: Image-on-Image Steganography

## Overview

The encryption has been changed from pixel scrambling to **classic image steganography**, where one image is hidden inside another.

## How It Works

### Encryption Process:

1. **Input**: Original image to encrypt
2. **Generate Cover Image**: Create a gradient pattern "encryption key" image
3. **Hide Original**: Embed the entire original image data into the LSBs (Least Significant Bits) of the cover image pixels
4. **Embed Metadata**: Also hide usernames and viewing quota in LSBs
5. **Output**: PNG image that looks like the cover (encryption key) but contains the hidden original

### What Gets Embedded (in order):

```
[4 bytes: metadata length]
[N bytes: metadata JSON (usernames, quota)]
[4 bytes: original image length]
[M bytes: original image data (full JPEG/PNG bytes)]
```

### Decryption Process:

1. **Input**: Encrypted image (looks like cover/key image)
2. **Extract from LSBs**: Read the hidden data bit by bit from pixel LSBs
3. **Parse Structure**: Extract metadata length → metadata → image length → image data
4. **Output**: Original image fully restored

## Key Features

✅ **Visual Steganography**: Encrypted image looks like a colorful gradient (the "encryption key")
✅ **Lossless**: Original image is perfectly preserved (byte-for-byte identical after decryption)
✅ **Invisible**: Changes to cover image are imperceptible (only LSB modified)
✅ **Secure**: Without knowing the structure, hidden data is nearly impossible to detect
✅ **Includes Metadata**: Usernames and viewing quota still embedded

## Cover Image (Encryption Key)

The cover image is auto-generated as a **gradient pattern**:
- Width: At least 800px or original width (whichever is larger)
- Height: At least 600px or original height (whichever is larger)
- Pattern: RGB gradient from red→green→blue
- Format: PNG (lossless to preserve LSBs)

### Example Cover Pattern:
```
Top-left (0,0): RGB(0, 0, 0)
Top-right (W,0): RGB(255, 0, 128)
Bottom-left (0,H): RGB(0, 255, 128)
Bottom-right (W,H): RGB(255, 255, 255)
```

## Capacity Requirements

For an image to be successfully encrypted:

```
Required bits = (metadata_size + 4 + image_size + 4) × 8
Available bits = cover_width × cover_height × 3 (RGB channels)
```

### Example:
- Cover: 800×600 = 480,000 pixels × 3 = 1,440,000 bits available
- Can hide: ~180KB of data
- Typical JPEG (50KB) + metadata (200 bytes) = ~50KB total → ✅ Fits easily

## File Formats

| Stage | Format | Why |
|-------|--------|-----|
| Input (original) | JPEG/PNG | Any format accepted |
| Encrypted output | PNG | Lossless preserves LSBs |
| Decrypted output | Same as input | Original restored byte-for-byte |

## Implementation Details

### LSB Embedding:
- Each pixel byte (R, G, or B) can store 1 bit in its LSB
- Modifying LSB causes ±1 change (imperceptible to human eye)
- Data embedded sequentially across pixels

### Bit Extraction:
```rust
// Embed a bit
pixel[i] = (pixel[i] & 0xFE) | bit_value;  // Clear LSB, set to bit

// Extract a bit
bit_value = pixel[i] & 1;  // Read LSB
```

## Advantages Over Old Method

| Feature | Old (Pixel Scrambling) | New (Image Steganography) |
|---------|----------------------|--------------------------|
| Visual | Scrambled noise | Looks like cover image |
| Detection | Obviously encrypted | Looks innocent |
| Capacity | Limited by image size | Cover must be larger than original |
| Reversibility | Needs exact seed | Deterministic extraction |
| Security | Visual obfuscation | Hidden in plain sight |

## Testing

To test the new encryption:

### 1. Rebuild the project:
```bash
cd "/media/kirollos/Data/Distributed Systems/Cloud Project"
cargo build --release
```

### 2. Test with stress test:
```bash
cd multiple_img_testing
./quick_test.sh
```

### 3. Check encrypted images:
- They should look like gradient patterns (the "encryption key")
- Not scrambled noise
- When decrypted, original image is restored perfectly

## Code Changes

### Modified Files:
- `src/encryption.rs` - Complete rewrite of encryption/decryption logic

### Removed:
- ❌ `calculate_seed()` - No longer needed
- ❌ `scramble_pixels()` - No longer needed
- ❌ `unscramble_pixels()` - No longer needed
- ❌ SHA256 hashing for seed - No longer needed

### Added:
- ✅ `generate_cover_image()` - Creates encryption key pattern
- ✅ `embed_u32()` - Embeds 32-bit integers
- ✅ `embed_bytes()` - Embeds byte arrays
- ✅ `extract_u32()` - Extracts 32-bit integers
- ✅ `extract_bytes()` - Extracts byte arrays

## Security Notes

### Visual Security:
- Encrypted image looks like innocent gradient pattern
- Casual observer cannot tell it contains hidden data
- Statistical analysis might detect LSB embedding

### Data Security:
- No password/key required (data is just hidden, not encrypted)
- Anyone with the encrypted image can extract hidden data if they know the structure
- For true encryption, combine with AES or similar

### Metadata Security:
- Usernames and quota are hidden alongside image
- Only authorized users can view (enforced by node logic)
- Metadata extracted during decryption

## Future Enhancements

Possible improvements:

1. **Custom Cover Images**: Allow users to provide their own cover image
2. **Compression**: Compress original before embedding (fit larger images)
3. **Encryption**: Add AES encryption before embedding (true security)
4. **Error Correction**: Add Reed-Solomon codes for corruption resistance
5. **Adaptive LSB**: Use 2 LSBs for larger capacity (slight quality loss)

## Comparison

### Before (Pixel Scrambling):
```
[Original Image]
    ↓ Scramble pixels
[Scrambled Image] ← Looks like random noise
    ↓ Embed metadata in LSBs
[Final Encrypted] ← Still looks like noise
```

### After (Image Steganography):
```
[Original Image] + [Cover Image (gradient)]
    ↓ Embed original into cover's LSBs
[Encrypted Image] ← Looks like gradient pattern
    ↓ Extract from LSBs
[Original Image] ← Perfectly restored
```

## Performance

- **Encryption**: Slightly slower (generates cover image)
- **Decryption**: Faster (no unscrambling needed)
- **File Size**: Larger (cover image dimensions, not original)
- **Quality**: Perfect (lossless extraction)

## Verification

After rebuilding, you can verify it works:

1. Encrypt an image → Should look like gradient pattern
2. Save encrypted image to disk
3. Open in image viewer → Should see gradient, not original
4. Decrypt → Should get exact original back
5. Compare bytes → Original and decrypted should match exactly

## Summary

The new encryption method provides:
- ✅ True steganography (hiding in plain sight)
- ✅ Perfect restoration of original image
- ✅ Visual camouflage (looks like encryption key)
- ✅ Metadata preservation (usernames, quota)
- ✅ Lossless and deterministic

The encrypted images now look like harmless gradient patterns instead of obvious scrambled data!
