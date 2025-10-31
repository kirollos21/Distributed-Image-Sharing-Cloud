#!/usr/bin/env python3
"""
Create a TINY test image for UDP testing
Generates a ~8KB JPEG image that will work with the 10KB limit
UDP requires both request AND response to fit in 65KB packets
"""

try:
    from PIL import Image, ImageDraw, ImageFont
    import random
except ImportError:
    print("Installing required package...")
    import subprocess
    subprocess.check_call(["pip3", "install", "pillow"])
    from PIL import Image, ImageDraw, ImageFont
    import random

# Create a TINY colorful image (must be <10KB)
width, height = 100, 80
image = Image.new('RGB', (width, height))
draw = ImageDraw.Draw(image)

# Draw colorful gradient background
for y in range(height):
    r = int((y / height) * 255)
    g = int((1 - y / height) * 255)
    b = 128
    draw.rectangle([0, y, width, y+1], fill=(r, g, b))

# Draw some shapes
draw.ellipse([20, 20, 80, 80], fill=(255, 255, 0), outline=(0, 0, 0))
draw.rectangle([100, 50, 180, 130], fill=(255, 100, 100), outline=(0, 0, 0))
draw.polygon([(120, 10), (140, 40), (100, 40)], fill=(100, 255, 100), outline=(0, 0, 0))

# Add text
try:
    draw.text((10, 120), "Test Image", fill=(255, 255, 255))
except:
    pass

# Save with JPEG compression to keep size VERY small
image.save('test_image_small.jpg', 'JPEG', quality=70)

# Check size
import os
size_bytes = os.path.getsize('test_image_small.jpg')
size_kb = size_bytes / 1024

print(f"✅ Created test_image_small.jpg")
print(f"   Size: {size_kb:.1f} KB ({size_bytes} bytes)")
print(f"   Dimensions: {width}x{height}")
print("")
if size_kb < 10:
    print(f"   ✅ Within UDP limit (10 KB) - should work!")
else:
    print(f"   ⚠️  Above UDP limit (10 KB) - will be compressed")
print("")
print("⚠️  UDP requires TINY images!")
print("   Request + Response must BOTH fit in 65KB UDP packets")
print("")
print("Use this image to test the client GUI!")
