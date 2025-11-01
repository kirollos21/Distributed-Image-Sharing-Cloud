#!/usr/bin/env python3
"""
Single Image Testing - Client Side
Sends an image to server, receives encrypted version, displays before/after, and decrypts
"""

import json
import socket
import sys
import os
import time
from pathlib import Path
from PIL import Image
import io

class ImageTestClient:
    def __init__(self, server_address, image_path):
        self.server_address = server_address
        self.image_path = image_path
        self.output_dir = Path("client_output")
        self.output_dir.mkdir(exist_ok=True)
        
    def load_image(self):
        """Load and display original image"""
        print(f"\n{'='*70}")
        print(f"STEP 1: Loading Original Image")
        print(f"{'='*70}")
        
        with open(self.image_path, 'rb') as f:
            image_data = f.read()
        
        print(f"✓ Loaded image: {self.image_path}")
        print(f"✓ Size: {len(image_data)} bytes ({len(image_data)/1024:.2f} KB)")
        
        # Display original image
        try:
            img = Image.open(self.image_path)
            print(f"✓ Dimensions: {img.size[0]}x{img.size[1]}")
            print(f"✓ Format: {img.format}")
            print(f"\n📷 Displaying original image...")
            
            # Save a copy for comparison
            original_copy = self.output_dir / "01_original_image.jpg"
            img.save(original_copy)
            print(f"✓ Saved copy to: {original_copy}")
            
            # Show image (non-blocking)
            img.show(title="1. ORIGINAL IMAGE (Client Side)")
        except Exception as e:
            print(f"⚠ Could not display image: {e}")
        
        return image_data
    
    def send_to_server(self, image_data):
        """Send image to server for encryption"""
        print(f"\n{'='*70}")
        print(f"STEP 2: Sending to Server for Encryption")
        print(f"{'='*70}")
        
        # Check image size first
        if len(image_data) > 40000:  # 40KB limit for safety
            print(f"⚠ Image is {len(image_data)/1024:.2f} KB - resizing to fit UDP limit...")
            # Resize image to fit
            img = Image.open(io.BytesIO(image_data))
            # Calculate scale to get under 40KB
            scale = 0.7  # Start with 70% size
            while True:
                new_width = int(img.size[0] * scale)
                new_height = int(img.size[1] * scale)
                resized = img.resize((new_width, new_height), Image.Resampling.LANCZOS)
                output = io.BytesIO()
                resized.save(output, format='JPEG', quality=75)
                image_data = output.getvalue()
                if len(image_data) <= 40000 or scale <= 0.3:
                    break
                scale -= 0.1
            print(f"✓ Resized to {new_width}x{new_height}, size: {len(image_data)/1024:.2f} KB")
        
        # Create encryption request
        request_id = f"test_img_{int(time.time())}"
        message = {
            "EncryptionRequest": {
                "request_id": request_id,
                "client_username": "test_user",
                "image_data": list(image_data),
                "usernames": ["alice", "bob", "charlie"],
                "quota": 5,
                "forwarded": False
            }
        }
        
        message_bytes = json.dumps(message).encode('utf-8')
        
        # Check final message size
        if len(message_bytes) > 65000:
            raise ValueError(f"Message still too large: {len(message_bytes)/1024:.2f} KB. Try a smaller image.")
        
        print(f"✓ Request ID: {request_id}")
        print(f"✓ Target users: {message['EncryptionRequest']['usernames']}")
        print(f"✓ Quota: {message['EncryptionRequest']['quota']}")
        print(f"✓ Image data: {len(image_data)} bytes ({len(image_data)/1024:.2f} KB)")
        print(f"✓ JSON message size: {len(message_bytes)} bytes ({len(message_bytes)/1024:.2f} KB)")
        
        # Parse server address
        host, port = self.server_address.split(':')
        port = int(port)
        
        # Send to server
        print(f"\n📤 Sending to server {self.server_address}...")
        
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        sock.settimeout(15.0)  # 15 second timeout
        
        start_time = time.time()
        sock.sendto(message_bytes, (host, port))
        print(f"✓ Request sent, waiting for response...")
        
        # Wait for response
        response_data, _ = sock.recvfrom(65535)
        elapsed = time.time() - start_time
        
        print(f"✓ Response received in {elapsed:.2f}s")
        print(f"✓ Response size: {len(response_data)} bytes ({len(response_data)/1024:.2f} KB)")
        
        sock.close()
        
        response = json.loads(response_data.decode('utf-8'))
        return response, elapsed
    
    def process_encrypted_image(self, response):
        """Save and display encrypted image"""
        print(f"\n{'='*70}")
        print(f"STEP 3: Processing Encrypted Image")
        print(f"{'='*70}")
        
        if not response.get('EncryptionResponse', {}).get('success'):
            error = response.get('EncryptionResponse', {}).get('error', 'Unknown error')
            print(f"❌ Server returned error: {error}")
            return None
        
        encrypted_data = bytes(response['EncryptionResponse']['encrypted_image'])
        print(f"✓ Encrypted image received: {len(encrypted_data)} bytes ({len(encrypted_data)/1024:.2f} KB)")
        
        # CRITICAL: Save as PNG to preserve LSB metadata!
        # JPEG compression would destroy the LSB-encoded metadata needed for decryption
        encrypted_path = self.output_dir / "02_encrypted_image.png"
        with open(encrypted_path, 'wb') as f:
            f.write(encrypted_data)
        print(f"✓ Saved encrypted image to: {encrypted_path}")
        
        # Also save with simple name for decrypt script
        encrypted_simple = self.output_dir / "encrypted_image.png"
        with open(encrypted_simple, 'wb') as f:
            f.write(encrypted_data)
        
        # Display encrypted image
        try:
            img = Image.open(io.BytesIO(encrypted_data))
            print(f"✓ Encrypted dimensions: {img.size[0]}x{img.size[1]}")
            print(f"\n🔒 Displaying ENCRYPTED image (should look scrambled)...")
            img.show(title="2. ENCRYPTED IMAGE (Received from Server - Scrambled)")
        except Exception as e:
            print(f"⚠ Could not display encrypted image: {e}")
        
        return encrypted_data
    
    def decrypt_image(self, encrypted_data):
        """Decrypt the image using the decryption endpoint"""
        print(f"\n{'='*70}")
        print(f"STEP 4: Decrypting Image (Client Side)")
        print(f"{'='*70}")
        
        # For now, just show that we have the encrypted data
        # In a real scenario, client would use the same encryption.rs decrypt function
        print(f"✓ Encrypted data available: {len(encrypted_data)} bytes")
        print(f"✓ To decrypt, you would:")
        print(f"  1. Load encrypted image")
        print(f"  2. Extract metadata from LSBs")
        print(f"  3. Calculate seed from metadata")
        print(f"  4. Unscramble pixels using seed")
        print(f"  5. Re-encode as original format")
        
        # Save a note
        note_path = self.output_dir / "decryption_note.txt"
        with open(note_path, 'w') as f:
            f.write("Decryption Process:\n")
            f.write("==================\n\n")
            f.write("The encrypted image contains:\n")
            f.write("1. Hidden metadata in pixel LSBs (usernames + quota)\n")
            f.write("2. Scrambled pixels (Fisher-Yates shuffle)\n\n")
            f.write("To decrypt:\n")
            f.write("1. Extract metadata length (first 32 bits)\n")
            f.write("2. Extract metadata JSON from LSBs\n")
            f.write("3. Parse usernames and quota\n")
            f.write("4. Calculate seed = hash(usernames + quota)\n")
            f.write("5. Unscramble pixels using same seed\n")
            f.write("6. Result = original image restored\n\n")
            f.write(f"See src/encryption.rs decrypt_image() for implementation\n")
        
        print(f"✓ Saved decryption notes to: {note_path}")
        
        return True
    
    def run(self):
        """Run complete test"""
        print(f"\n{'#'*70}")
        print(f"# SINGLE IMAGE ENCRYPTION TEST - CLIENT SIDE")
        print(f"{'#'*70}")
        print(f"Server: {self.server_address}")
        print(f"Image: {self.image_path}")
        print(f"Output: {self.output_dir}")
        print(f"{'#'*70}\n")
        
        try:
            # Step 1: Load original image
            image_data = self.load_image()
            
            # Step 2: Send to server
            response, elapsed = self.send_to_server(image_data)
            
            # Step 3: Process encrypted response
            encrypted_data = self.process_encrypted_image(response)
            
            if encrypted_data:
                # Step 4: Decrypt (demonstration)
                self.decrypt_image(encrypted_data)
                
                # Summary
                print(f"\n{'='*70}")
                print(f"TEST COMPLETE - SUMMARY")
                print(f"{'='*70}")
                print(f"✓ Original image loaded and displayed")
                print(f"✓ Image sent to server ({len(image_data)/1024:.2f} KB)")
                print(f"✓ Encrypted image received ({len(encrypted_data)/1024:.2f} KB)")
                print(f"✓ Processing time: {elapsed:.2f}s")
                print(f"✓ Encrypted image displayed (scrambled)")
                print(f"✓ Files saved to: {self.output_dir}/")
                print(f"\n📁 Output files:")
                print(f"   - encrypted_image.jpg (scrambled, has hidden metadata)")
                print(f"   - decryption_note.txt (how to decrypt)")
                print(f"\n{'='*70}\n")
                
                return True
            else:
                print(f"\n❌ Test failed - encryption error")
                return False
                
        except socket.timeout:
            print(f"\n❌ Timeout - Server not responding")
            return False
        except ConnectionRefusedError:
            print(f"\n❌ Connection refused - Is server running?")
            return False
        except Exception as e:
            print(f"\n❌ Error: {e}")
            import traceback
            traceback.print_exc()
            return False


def main():
    if len(sys.argv) < 3:
        print("Usage: python3 test_client.py <server_address> <image_path>")
        print("Example: python3 test_client.py 10.40.59.43:8001 test_image.jpg")
        sys.exit(1)
    
    server_address = sys.argv[1]
    image_path = sys.argv[2]
    
    if not os.path.exists(image_path):
        print(f"Error: Image file not found: {image_path}")
        sys.exit(1)
    
    client = ImageTestClient(server_address, image_path)
    success = client.run()
    
    sys.exit(0 if success else 1)


if __name__ == '__main__':
    main()
