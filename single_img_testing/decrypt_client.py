#!/usr/bin/env python3
"""
Image Decryption Client
Decrypts an encrypted image by extracting metadata and unscrambling pixels
"""

import sys
import json
import hashlib
from pathlib import Path
from PIL import Image
import io

class ImageDecryptor:
    def __init__(self, encrypted_image_path):
        self.encrypted_image_path = Path(encrypted_image_path)
        self.output_dir = Path("client_output")
        self.output_dir.mkdir(exist_ok=True)
    
    def extract_metadata(self, pixels):
        """Extract metadata from LSBs of pixel data"""
        print(f"\n{'='*70}")
        print(f"Extracting Hidden Metadata from LSBs")
        print(f"{'='*70}")
        
        # Extract metadata length from first 32 bits (4 bytes)
        len_bytes = bytearray(4)
        for i in range(4):
            byte = 0
            for bit in range(8):
                pixel_index = i * 8 + bit
                bit_value = pixels[pixel_index] & 1  # Get LSB
                byte = (byte << 1) | bit_value
            len_bytes[i] = byte
        
        metadata_len = int.from_bytes(len_bytes, byteorder='big')
        print(f"✓ Metadata length: {metadata_len} bytes")
        
        if metadata_len == 0 or metadata_len > 10000:
            raise ValueError(f"Invalid metadata length: {metadata_len}")
        
        # Extract metadata bytes from LSBs (starting after length)
        start_offset = 32
        metadata_bytes = bytearray(metadata_len)
        
        for i in range(metadata_len):
            byte = 0
            for bit in range(8):
                pixel_index = start_offset + i * 8 + bit
                if pixel_index >= len(pixels):
                    raise ValueError("Unexpected end of pixel data")
                bit_value = pixels[pixel_index] & 1
                byte = (byte << 1) | bit_value
            metadata_bytes[i] = byte
        
        # Parse JSON metadata
        metadata_json = metadata_bytes.decode('utf-8')
        metadata = json.loads(metadata_json)
        
        print(f"✓ Extracted metadata:")
        print(f"   - Usernames: {metadata['usernames']}")
        print(f"   - Quota: {metadata['quota']}")
        
        return metadata
    
    def calculate_seed(self, metadata):
        """Calculate deterministic seed from metadata"""
        print(f"\n{'='*70}")
        print(f"Calculating Seed for Unscrambling")
        print(f"{'='*70}")
        
        # Create hash from usernames and quota (same as Rust implementation)
        hasher = hashlib.sha256()
        for username in metadata['usernames']:
            hasher.update(username.encode('utf-8'))
        hasher.update(str(metadata['quota']).encode('utf-8'))
        
        # Convert to u64 equivalent
        hash_bytes = hasher.digest()
        seed = int.from_bytes(hash_bytes[:8], byteorder='big')
        
        print(f"✓ Seed calculated: {seed}")
        print(f"   (Derived from usernames + quota hash)")
        
        return seed
    
    def unscramble_pixels(self, pixels, seed, width, height):
        """Unscramble pixels using reverse Fisher-Yates shuffle"""
        print(f"\n{'='*70}")
        print(f"Unscrambling Pixels (Reverse Fisher-Yates)")
        print(f"{'='*70}")
        
        # NOTE: The LSBs were modified after scrambling for metadata storage
        # This causes minor (±1) color differences in ~500 bytes
        # These differences are visually imperceptible but prevent exact pixel match
        
        pixels = bytearray(pixels)
        num_pixels = len(pixels) // 4  # RGBA pixels
        
        # Generate same sequence of swaps as encryption
        swap_indices = []
        rng_state = seed
        
        # LCG constants (same as Rust)
        multiplier = 6364136223846793005
        increment = 1442695040888963407
        
        for i in range(num_pixels - 1, 0, -1):
            # Generate pseudo-random index
            rng_state = (rng_state * multiplier + increment) & 0xFFFFFFFFFFFFFFFF
            j = rng_state % (i + 1)
            swap_indices.append((i, j))
        
        print(f"✓ Generated {len(swap_indices)} swap operations")
        
        # Apply swaps in REVERSE order to unscramble
        for i, j in reversed(swap_indices):
            idx_i = i * 4
            idx_j = j * 4
            
            # Swap RGBA pixels (4 bytes each)
            for k in range(4):
                pixels[idx_i + k], pixels[idx_j + k] = pixels[idx_j + k], pixels[idx_i + k]
        
        print(f"✓ Pixels unscrambled successfully")
        print(f"⚠️  Note: LSB modifications cause minor (±1) color differences")
        
        return bytes(pixels)
    
    def decrypt(self):
        """Decrypt the encrypted image"""
        print(f"\n{'#'*70}")
        print(f"# IMAGE DECRYPTION PROCESS")
        print(f"{'#'*70}")
        print(f"Encrypted image: {self.encrypted_image_path}")
        print(f"Output directory: {self.output_dir}")
        print(f"{'#'*70}\n")
        
        # Step 1: Load encrypted image
        print(f"{'='*70}")
        print(f"STEP 1: Loading Encrypted Image")
        print(f"{'='*70}")
        
        with open(self.encrypted_image_path, 'rb') as f:
            encrypted_data = f.read()
        
        img = Image.open(io.BytesIO(encrypted_data))
        width, height = img.size
        print(f"✓ Loaded encrypted image: {width}x{height}")
        print(f"✓ Size: {len(encrypted_data)} bytes ({len(encrypted_data)/1024:.2f} KB)")
        
        # Show encrypted image
        print(f"\n🔒 Displaying ENCRYPTED image (scrambled)...")
        img.show(title="2. ENCRYPTED IMAGE (Before Decryption - Scrambled)")
        
        # Convert to RGBA
        rgba_img = img.convert('RGBA')
        pixels = rgba_img.tobytes()
        print(f"✓ Converted to RGBA: {len(pixels)} bytes")
        
        # Step 2: Extract metadata
        metadata = self.extract_metadata(pixels)
        
        # Step 3: Calculate seed
        seed = self.calculate_seed(metadata)
        
        # Step 4: Unscramble pixels
        unscrambled_pixels = self.unscramble_pixels(pixels, seed, width, height)
        
        # Step 5: Create decrypted image
        print(f"\n{'='*70}")
        print(f"STEP 5: Creating Decrypted Image")
        print(f"{'='*70}")
        
        decrypted_img = Image.frombytes('RGBA', (width, height), unscrambled_pixels)
        
        # Save decrypted image
        decrypted_path = self.output_dir / "03_decrypted_image.png"
        decrypted_img.save(decrypted_path)
        print(f"✓ Saved decrypted image to: {decrypted_path}")
        
        # Also save with simple name
        decrypted_simple = self.output_dir / "decrypted_image.png"
        decrypted_img.save(decrypted_simple)
        
        # Show decrypted image
        print(f"\n🔓 Displaying DECRYPTED image (original restored)...")
        decrypted_img.show(title="3. DECRYPTED IMAGE (Original Restored!)")
        
        # Summary
        print(f"\n{'='*70}")
        print(f"DECRYPTION COMPLETE - SUMMARY")
        print(f"{'='*70}")
        print(f"✓ Extracted metadata from LSBs:")
        print(f"   - Authorized users: {metadata['usernames']}")
        print(f"   - Remaining views: {metadata['quota']}")
        print(f"✓ Calculated seed from metadata: {seed}")
        print(f"✓ Unscrambled {len(unscrambled_pixels)} pixel bytes")
        print(f"✓ Original image restored!")
        print(f"\n📁 Output files:")
        print(f"   - {decrypted_path}")
        print(f"   - {decrypted_simple}")
        print(f"\n{'='*70}\n")
        
        return True


def main():
    if len(sys.argv) < 2:
        print("Usage: python3 decrypt_client.py <encrypted_image_path>")
        print("Example: python3 decrypt_client.py client_output/encrypted_image.jpg")
        sys.exit(1)
    
    encrypted_image_path = sys.argv[1]
    
    if not Path(encrypted_image_path).exists():
        print(f"Error: Encrypted image not found: {encrypted_image_path}")
        sys.exit(1)
    
    try:
        decryptor = ImageDecryptor(encrypted_image_path)
        success = decryptor.decrypt()
        sys.exit(0 if success else 1)
    except Exception as e:
        print(f"\n❌ Decryption failed: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == '__main__':
    main()
