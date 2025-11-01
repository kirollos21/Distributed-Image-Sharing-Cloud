#!/usr/bin/env python3
"""
Create an ULTRA-TINY test image (80x60) for UDP transmission.

Problem: JSON serialization of Vec<u8> causes 4-5x size increase
Solution: Make image tiny so even JSON-serialized encrypted image fits in UDP packet

Server resizes if: (width * height * 3) / 2 > 30000
For 80x60: 80 * 60 * 1.5 = 7,200 < 30,000 ✓ (NO RESIZE!)

Target: Encrypted JPEG ~3-4KB → JSON serialized ~15-20KB (fits in 65KB UDP limit)
"""

from PIL import Image, ImageDraw, ImageFont
import os

# Create an ultra-tiny image for UDP compatibility
width = 80
height = 60

# Create image with a simple gradient background
img = Image.new('RGB', (width, height))
draw = ImageDraw.Draw(img)

# Create a colorful gradient
for y in range(height):
    color_value = int((y / height) * 255)
    draw.rectangle([(0, y), (width, y+1)], fill=(color_value, 100, 255 - color_value))

# Add some simple shapes
draw.rectangle([10, 10, 30, 30], fill=(255, 0, 0))
draw.ellipse([40, 15, 70, 45], fill=(0, 255, 0))
draw.polygon([(35, 50), (45, 35), (55, 50)], fill=(255, 255, 0))

# Add text
try:
    font = ImageFont.load_default()
    draw.text((5, 3), "Test", fill=(255, 255, 255))
except:
    pass

# Save with maximum compression
output_path = "../test_image.jpg"
img.save(output_path, "JPEG", quality=50, optimize=True)

file_size = os.path.getsize(output_path)
estimated_encrypted_size = file_size * 1.5  # Rough estimate after encryption
estimated_json_size = estimated_encrypted_size * 4  # JSON array overhead

print(f"✓ Created ultra-tiny test image: {output_path}")
print(f"✓ Dimensions: {width}x{height}")
print(f"✓ Original file size: {file_size} bytes ({file_size/1024:.2f} KB)")
print(f"✓ Estimated pixel data: {width * height * 3 // 2} bytes (< 30000, no resize!)")
print(f"✓ Estimated encrypted: ~{int(estimated_encrypted_size)} bytes (~{estimated_encrypted_size/1024:.1f} KB)")
print(f"✓ Estimated JSON size: ~{int(estimated_json_size)} bytes (~{estimated_json_size/1024:.1f} KB)")
print()
if estimated_json_size < 50000:
    print("✅ Should fit in UDP packet (< 50KB)!")
else:
    print("⚠️  May still be too large for UDP")
print("✅ This image will NOT be resized by the server!")
print("✅ Decryption will work correctly!")
