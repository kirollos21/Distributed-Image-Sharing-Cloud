#!/usr/bin/env python3
from PIL import Image, ImageDraw, ImageFont

# Create 160x120 image (won't be resized: 160*120*1.5=28800 < 30000)
img = Image.new('RGB', (160, 120))
draw = ImageDraw.Draw(img)

# Gradient background
for y in range(120):
    r = int(255 * (y / 120))
    g = int(128 * (1 - y / 120))
    b = int(200 * (y / 120))
    draw.line([(0, y), (160, y)], fill=(r, g, b))

# Shapes
draw.ellipse([10, 10, 50, 50], fill=(255, 0, 0), outline=(255, 255, 0), width=2)
draw.rectangle([60, 20, 100, 60], fill=(0, 255, 0), outline=(0, 0, 255), width=2)
draw.polygon([(110, 20), (130, 40), (120, 60), (100, 60), (90, 40)], 
             fill=(255, 255, 0), outline=(255, 0, 255), width=1)

# Text
try:
    font = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf', 14)
except:
    font = ImageFont.load_default()

text = 'TEST IMAGE'
bbox = draw.textbbox((0, 0), text, font=font)
text_x = (160 - (bbox[2] - bbox[0])) // 2
draw.text((text_x+1, 96), text, fill=(0, 0, 0), font=font)
draw.text((text_x, 95), text, fill=(255, 255, 255), font=font)

# Save
img.save('../test_image.jpg', 'JPEG', quality=70, optimize=True)

import os
size = os.path.getsize('../test_image.jpg')
print(f'✓ Created test image: ../test_image.jpg')
print(f'✓ Dimensions: 160x120')
print(f'✓ File size: {size} bytes ({size/1024:.2f} KB)')
print(f'✓ Estimated pixel data: {160*120*1.5:.0f} bytes (< 30000, no resize!)')
print(f'\n✅ This image will NOT be resized by the server!')
print(f'✅ Decryption will work correctly!')
