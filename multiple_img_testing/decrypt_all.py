#!/usr/bin/env python3
"""
Decrypt and verify all encrypted images
"""

import json
import sys
import hashlib
from pathlib import Path
from PIL import Image
import io


def extract_metadata(pixels):
    """Extract metadata from LSBs of pixel data"""
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

    return metadata


def calculate_seed(metadata):
    """Calculate deterministic seed from metadata"""
    hasher = hashlib.sha256()
    for username in metadata['usernames']:
        hasher.update(username.encode('utf-8'))
    hasher.update(str(metadata['quota']).encode('utf-8'))

    hash_bytes = hasher.digest()
    seed = int.from_bytes(hash_bytes[:8], byteorder='big')

    return seed


def unscramble_pixels(pixels, seed, width, height):
    """Unscramble pixels using reverse Fisher-Yates shuffle"""
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

    # Apply swaps in REVERSE order to unscramble
    for i, j in reversed(swap_indices):
        idx_i = i * 4
        idx_j = j * 4

        # Swap RGBA pixels (4 bytes each)
        for k in range(4):
            pixels[idx_i + k], pixels[idx_j + k] = pixels[idx_j + k], pixels[idx_i + k]

    return bytes(pixels)


def decrypt_image(encrypted_image_path):
    """Decrypt an encrypted image"""
    try:
        # Load encrypted image
        with open(encrypted_image_path, 'rb') as f:
            encrypted_data = f.read()

        img = Image.open(io.BytesIO(encrypted_data))
        width, height = img.size

        # Convert to RGBA
        rgba_img = img.convert('RGBA')
        pixels = rgba_img.tobytes()

        # Extract metadata
        metadata = extract_metadata(pixels)

        # Calculate seed
        seed = calculate_seed(metadata)

        # Unscramble pixels
        unscrambled_pixels = unscramble_pixels(pixels, seed, width, height)

        # Create decrypted image
        decrypted_img = Image.frombytes('RGBA', (width, height), unscrambled_pixels)

        return {
            'success': True,
            'image': decrypted_img,
            'metadata': metadata,
            'dimensions': (width, height)
        }

    except Exception as e:
        return {
            'success': False,
            'error': str(e)
        }


def main():
    print("=" * 80)
    print(" DECRYPT ALL ENCRYPTED IMAGES")
    print("=" * 80)
    print()

    # Load config
    config_path = Path(__file__).parent / "config.json"
    with open(config_path, 'r') as f:
        config = json.load(f)

    encrypted_dir = Path(config['output_config']['encrypted_dir'])
    decrypted_dir = Path(config['output_config']['decrypted_dir'])
    decrypted_dir.mkdir(parents=True, exist_ok=True)

    # Find all encrypted images
    encrypted_files = sorted(encrypted_dir.glob("encrypted_*.png"))

    if not encrypted_files:
        print("No encrypted images found!")
        return False

    print(f"Found {len(encrypted_files)} encrypted images")
    print()

    successes = 0
    failures = 0
    results = []

    for enc_file in encrypted_files:
        print(f"Decrypting {enc_file.name}...", end=" ")

        result = decrypt_image(enc_file)

        if result['success']:
            print("✓")

            # Save decrypted image
            dec_filename = enc_file.name.replace("encrypted_", "decrypted_")
            dec_path = decrypted_dir / dec_filename

            result['image'].save(dec_path)

            successes += 1
            results.append({
                'file': enc_file.name,
                'success': True,
                'metadata': result['metadata'],
                'dimensions': result['dimensions'],
                'output': str(dec_path)
            })
        else:
            print(f"✗ {result['error']}")
            failures += 1
            results.append({
                'file': enc_file.name,
                'success': False,
                'error': result['error']
            })

    print()
    print("=" * 80)
    print("DECRYPTION RESULTS:")
    print(f"  Total: {len(encrypted_files)}")
    print(f"  Success: {successes} ({successes/len(encrypted_files)*100:.1f}%)")
    print(f"  Failure: {failures} ({failures/len(encrypted_files)*100:.1f}%)")
    print()

    # Save results
    results_file = Path(config['output_config']['metrics_dir']) / "decryption_results.json"
    with open(results_file, 'w') as f:
        json.dump({
            'total': len(encrypted_files),
            'successes': successes,
            'failures': failures,
            'success_rate': successes / len(encrypted_files) * 100,
            'results': results
        }, f, indent=2)

    print(f"✓ Results saved to: {results_file}")
    print("=" * 80)

    return failures == 0


if __name__ == '__main__':
    success = main()
    sys.exit(0 if success else 1)
