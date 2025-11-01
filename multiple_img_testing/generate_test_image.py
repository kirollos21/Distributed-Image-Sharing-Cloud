#!/usr/bin/env python3
"""
Generate unique test images for stress testing
Each image has unique patterns and identifiers
"""

from PIL import Image, ImageDraw, ImageFont
import random
import hashlib

def generate_test_image(process_id, image_id, width=1280, height=720, quality=85):
    """
    Generate a unique test image with identifiable patterns

    Args:
        process_id: Process number (1-based)
        image_id: Image number within process (1-based)
        width: Image width
        height: Image height
        quality: JPEG quality

    Returns:
        bytes: JPEG image data
    """

    # Create unique seed based on IDs
    seed = f"{process_id}_{image_id}"
    random.seed(hashlib.md5(seed.encode()).hexdigest())

    # Create image with unique background gradient
    img = Image.new('RGB', (width, height))
    draw = ImageDraw.Draw(img)

    # Generate unique color scheme based on process_id
    base_hue = (process_id * 36) % 360

    # Background gradient
    for y in range(height):
        progress = y / height
        r = int(100 + progress * (50 + (process_id * 10) % 100))
        g = int(80 + progress * (40 + (image_id * 8) % 80))
        b = int(150 - progress * (30 + (process_id * 5) % 50))

        # Clamp values
        r, g, b = min(255, r), min(255, g), min(255, b)
        draw.rectangle([(0, y), (width, y+1)], fill=(r, g, b))

    # Add unique geometric patterns
    num_circles = 3 + (process_id % 5)
    for i in range(num_circles):
        x = random.randint(100, width - 100)
        y = random.randint(100, height - 100)
        radius = random.randint(30, 80)
        color = (
            random.randint(100, 255),
            random.randint(100, 255),
            random.randint(100, 255)
        )
        draw.ellipse([x-radius, y-radius, x+radius, y+radius], fill=color)

    # Add rectangles
    num_rects = 2 + (image_id % 4)
    for i in range(num_rects):
        x1 = random.randint(50, width - 200)
        y1 = random.randint(50, height - 150)
        x2 = x1 + random.randint(100, 200)
        y2 = y1 + random.randint(80, 150)
        color = (
            random.randint(150, 255),
            random.randint(150, 255),
            random.randint(100, 200)
        )
        draw.rectangle([x1, y1, x2, y2], fill=color, outline=(0, 0, 0), width=2)

    # Add text identifiers
    try:
        font = ImageFont.load_default()
    except:
        font = None

    # Title with process and image ID
    title_text = f"Process {process_id} - Image {image_id}"
    draw.text((20, 20), title_text, fill=(255, 255, 255), font=font)

    # Resolution info
    res_text = f"{width}x{height}"
    draw.text((20, 40), res_text, fill=(255, 255, 255), font=font)

    # Unique identifier hash
    identifier = f"P{process_id:02d}I{image_id:03d}"
    hash_text = f"ID: {identifier} | Hash: {hashlib.md5(seed.encode()).hexdigest()[:8]}"
    draw.text((20, height - 40), hash_text, fill=(255, 255, 255), font=font)

    # Add process color bar at top
    bar_height = 10
    process_color = (
        (process_id * 25) % 256,
        (process_id * 40) % 256,
        (process_id * 60) % 256
    )
    draw.rectangle([0, 0, width, bar_height], fill=process_color)

    # Add image number bar at bottom
    image_color = (
        (image_id * 30) % 256,
        (image_id * 50) % 256,
        (image_id * 70) % 256
    )
    draw.rectangle([0, height-bar_height, width, height], fill=image_color)

    # Convert to bytes
    import io
    output = io.BytesIO()
    img.save(output, format='JPEG', quality=quality)
    return output.getvalue()


def save_test_image(process_id, image_id, output_path, width=1280, height=720, quality=85):
    """
    Generate and save a test image to file
    """
    image_data = generate_test_image(process_id, image_id, width, height, quality)

    with open(output_path, 'wb') as f:
        f.write(image_data)

    return len(image_data)


if __name__ == '__main__':
    import sys

    if len(sys.argv) < 4:
        print("Usage: python3 generate_test_image.py <process_id> <image_id> <output_path>")
        print("Example: python3 generate_test_image.py 1 1 test_image_1_1.jpg")
        sys.exit(1)

    process_id = int(sys.argv[1])
    image_id = int(sys.argv[2])
    output_path = sys.argv[3]

    size = save_test_image(process_id, image_id, output_path)
    print(f"Generated test image: {output_path} ({size} bytes)")
