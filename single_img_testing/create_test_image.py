#!/usr/bin/env python3
"""
Create a simple test image for encryption testing
"""

from PIL import Image, ImageDraw, ImageFont
import sys

def create_test_image(filename="test_image.jpg", width=400, height=300):
    """Create a colorful test image with text"""
    
    # Create image with gradient background
    img = Image.new('RGB', (width, height))
    draw = ImageDraw.Draw(img)
    
    # Draw gradient background
    for y in range(height):
        r = int(255 * (y / height))
        g = int(128 * (1 - y / height))
        b = int(200 * (y / height))
        draw.line([(0, y), (width, y)], fill=(r, g, b))
    
    # Draw some shapes
    draw.ellipse([30, 30, 120, 120], fill=(255, 0, 0), outline=(255, 255, 0), width=3)
    draw.rectangle([150, 60, 280, 180], fill=(0, 255, 0), outline=(0, 0, 255), width=3)
    draw.polygon([(320, 60), (370, 120), (350, 180), (290, 180), (270, 120)], 
                 fill=(255, 255, 0), outline=(255, 0, 255), width=2)
    
    # Add text
    try:
        # Try to use default font
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 30)
    except:
        font = ImageFont.load_default()
    
    text = "TEST IMAGE"
    # Get text bounding box for centering
    bbox = draw.textbbox((0, 0), text, font=font)
    text_width = bbox[2] - bbox[0]
    text_height = bbox[3] - bbox[1]
    text_x = (width - text_width) // 2
    text_y = height - 70
    
    # Draw text with shadow
    draw.text((text_x + 2, text_y + 2), text, fill=(0, 0, 0), font=font)
    draw.text((text_x, text_y), text, fill=(255, 255, 255), font=font)
    
    # Add watermark
    watermark = "Encryption Test"
    draw.text((10, 10), watermark, fill=(255, 255, 255, 128))
    
    # Save with lower quality to reduce file size for UDP
    img.save(filename, "JPEG", quality=70, optimize=True)
    
    # Check file size
    import os
    file_size = os.path.getsize(filename)
    
    print(f"✓ Created test image: {filename}")
    print(f"✓ Dimensions: {width}x{height}")
    print(f"✓ File size: {file_size} bytes ({file_size/1024:.2f} KB)")
    
    if file_size > 40000:
        print(f"⚠ Warning: Image is {file_size/1024:.2f} KB (over 40KB)")
        print(f"  Will be automatically resized when sending to server")
    
    # Show the image
    print(f"\n📷 Displaying test image...")
    img.show(title="Test Image Created")
    
    return filename

def main():
    if len(sys.argv) > 1:
        filename = sys.argv[1]
    else:
        filename = "test_image.jpg"
    
    create_test_image(filename)
    print(f"\nYou can now use this image with:")
    print(f"  python3 test_client.py <server:port> {filename}")

if __name__ == '__main__':
    main()
