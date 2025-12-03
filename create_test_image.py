#!/usr/bin/env python3
"""
Create a simple test image for encryption testing
"""

from PIL import Image, ImageDraw, ImageFont
import random

def create_test_image(filename="test_input.jpg", width=800, height=600):
    """Create a colorful test image with text"""

    # Create image with gradient background
    img = Image.new('RGB', (width, height))
    draw = ImageDraw.Draw(img)

    # Create gradient background
    for y in range(height):
        r = int((y / height) * 255)
        g = int(128 + (y / height) * 127)
        b = int(255 - (y / height) * 128)
        draw.line([(0, y), (width, y)], fill=(r, g, b))

    # Add some random circles
    for _ in range(20):
        x = random.randint(0, width)
        y = random.randint(0, height)
        radius = random.randint(20, 80)
        color = (random.randint(0, 255), random.randint(0, 255), random.randint(0, 255))
        draw.ellipse([x-radius, y-radius, x+radius, y+radius], fill=color, outline=(0, 0, 0), width=2)

    # Add text
    try:
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 48)
    except:
        font = ImageFont.load_default()

    text = "TEST IMAGE"
    # Get text bounding box
    bbox = draw.textbbox((0, 0), text, font=font)
    text_width = bbox[2] - bbox[0]
    text_height = bbox[3] - bbox[1]

    text_x = (width - text_width) // 2
    text_y = (height - text_height) // 2

    # Draw text with shadow
    draw.text((text_x+3, text_y+3), text, font=font, fill=(0, 0, 0))
    draw.text((text_x, text_y), text, font=font, fill=(255, 255, 255))

    # Save
    img.save(filename, quality=95)
    print(f"✅ Created test image: {filename}")
    print(f"   Size: {width}x{height}")
    print(f"   Format: JPEG")

    return filename

if __name__ == "__main__":
    import sys

    if len(sys.argv) > 1:
        filename = sys.argv[1]
    else:
        filename = "test_input.jpg"

    if len(sys.argv) > 3:
        width = int(sys.argv[2])
        height = int(sys.argv[3])
    else:
        width, height = 800, 600

    create_test_image(filename, width, height)
