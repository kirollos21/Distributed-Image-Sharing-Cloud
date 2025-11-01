#!/usr/bin/env python3
"""
Generate a 720p test image with patterns for testing encryption/decryption
"""

from PIL import Image, ImageDraw, ImageFont
import sys

def create_720p_test_image(output_path="test_image_720p.jpg"):
    """Create a 720p (1280x720) test image with patterns"""

    # Create 720p image
    width, height = 1280, 720
    img = Image.new('RGB', (width, height))
    draw = ImageDraw.Draw(img)

    print(f"Creating 720p test image: {width}x{height}")

    # Background gradient
    for y in range(height):
        # Create a nice gradient from blue to purple
        r = int(100 + (y / height) * 100)
        g = int(50 + (y / height) * 50)
        b = int(200 - (y / height) * 50)
        draw.rectangle([(0, y), (width, y+1)], fill=(r, g, b))

    # Add some geometric patterns
    # Circles
    for i in range(5):
        x = 200 + i * 200
        y = 150
        radius = 80
        color = (255 - i*40, 100 + i*30, 150 + i*20)
        draw.ellipse([x-radius, y-radius, x+radius, y+radius], fill=color)

    # Rectangles
    for i in range(4):
        x1 = 250 + i * 250
        y1 = 400
        x2 = x1 + 150
        y2 = y1 + 100
        color = (200, 150 + i*25, 100 + i*30)
        draw.rectangle([x1, y1, x2, y2], fill=color)

    # Add text
    try:
        # Try to use a default font, fallback to basic if not available
        font = ImageFont.load_default()
    except:
        font = None

    # Title
    title_text = "720p TEST IMAGE - 1280x720"
    title_bbox = draw.textbbox((0, 0), title_text, font=font)
    title_width = title_bbox[2] - title_bbox[0]
    title_x = (width - title_width) // 2
    draw.text((title_x, 50), title_text, fill=(255, 255, 255), font=font)

    # Description
    desc_text = "For testing multi-packet UDP transmission & encryption"
    desc_bbox = draw.textbbox((0, 0), desc_text, font=font)
    desc_width = desc_bbox[2] - desc_bbox[0]
    desc_x = (width - desc_width) // 2
    draw.text((desc_x, 600), desc_text, fill=(255, 255, 255), font=font)

    # Resolution info
    info_text = f"Resolution: {width} x {height} pixels"
    draw.text((20, height - 40), info_text, fill=(255, 255, 255), font=font)

    # Save as JPEG with good quality
    img.save(output_path, 'JPEG', quality=85)

    # Get file size
    import os
    file_size = os.path.getsize(output_path)

    print(f"✓ Created: {output_path}")
    print(f"✓ Resolution: {width}x{height}")
    print(f"✓ File size: {file_size} bytes ({file_size/1024:.2f} KB)")
    print(f"✓ Quality: 85")

    return output_path

if __name__ == '__main__':
    output_path = sys.argv[1] if len(sys.argv) > 1 else "test_image_720p.jpg"
    create_720p_test_image(output_path)
    print(f"\n💡 Use this image for testing: {output_path}")
