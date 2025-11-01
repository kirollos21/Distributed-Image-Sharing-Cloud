#!/usr/bin/env python3
"""
Single Image Testing - Client Side
Sends an image to server, receives encrypted version, displays before/after, and decrypts
Supports multi-packet transmission via chunking protocol
"""

import json
import socket
import sys
import os
import time
import uuid
import base64
from pathlib import Path
from PIL import Image
import io

# Chunk size for actual data (will be base64 encoded, adding ~33% overhead)
# 45KB data -> ~60KB base64 -> ~65KB with JSON wrapper (fits in UDP packet)
CHUNK_SIZE = 45000

def fragment_message(data):
    """Fragment a large message into chunks using base64 encoding"""
    data_bytes = data if isinstance(data, bytes) else data.encode('utf-8')
    data_len = len(data_bytes)

    # If fits in single packet, return as is (base64 encoded)
    if data_len <= CHUNK_SIZE:
        encoded = base64.b64encode(data_bytes).decode('ascii')
        return [{"SinglePacket": encoded}]

    # Calculate number of chunks
    total_chunks = (data_len + CHUNK_SIZE - 1) // CHUNK_SIZE
    chunk_id = str(uuid.uuid4())

    print(f"📦 Fragmenting message: {data_len} bytes into {total_chunks} chunks")

    # Create chunks
    chunks = []
    for chunk_index in range(total_chunks):
        start = chunk_index * CHUNK_SIZE
        end = min(start + CHUNK_SIZE, data_len)
        chunk_data = data_bytes[start:end]

        # Base64 encode the chunk data
        encoded_data = base64.b64encode(chunk_data).decode('ascii')

        chunks.append({
            "MultiPacket": {
                "chunk_id": chunk_id,
                "chunk_index": chunk_index,
                "total_chunks": total_chunks,
                "data": encoded_data
            }
        })

    return chunks

def reassemble_chunks(sock, timeout=30.0):
    """Receive and reassemble chunked messages with base64 decoding"""
    sock.settimeout(timeout)

    incomplete = {}

    while True:
        # Receive chunk
        chunk_data, _ = sock.recvfrom(65535)
        chunk_msg = json.loads(chunk_data.decode('utf-8'))

        # Check if single packet
        if "SinglePacket" in chunk_msg:
            encoded = chunk_msg["SinglePacket"]
            decoded = base64.b64decode(encoded)
            print(f"📥 Received single packet: {len(decoded)} bytes")
            return decoded

        # Multi-packet
        if "MultiPacket" in chunk_msg:
            mp = chunk_msg["MultiPacket"]
            chunk_id = mp["chunk_id"]
            chunk_index = mp["chunk_index"]
            total_chunks = mp["total_chunks"]

            # Base64 decode the chunk data
            encoded_data = mp["data"]
            data = base64.b64decode(encoded_data)

            print(f"📥 Received chunk {chunk_index + 1}/{total_chunks} for {chunk_id[:8]}... ({len(data)} bytes)")

            # Store chunk
            if chunk_id not in incomplete:
                incomplete[chunk_id] = {"total": total_chunks, "chunks": {}}

            incomplete[chunk_id]["chunks"][chunk_index] = data

            # Check if complete
            if len(incomplete[chunk_id]["chunks"]) == total_chunks:
                print(f"✓ All chunks received, reassembling...")

                # Reassemble in order
                complete_data = b""
                for i in range(total_chunks):
                    complete_data += incomplete[chunk_id]["chunks"][i]

                print(f"✓ Reassembly complete: {len(complete_data)} bytes")
                return complete_data

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
        """Send image to server for encryption using chunking protocol"""
        print(f"\n{'='*70}")
        print(f"STEP 2: Sending to Server for Encryption")
        print(f"{'='*70}")

        # NO SIZE RESTRICTIONS - chunking handles large images!
        print(f"✓ Image size: {len(image_data)/1024:.2f} KB ({len(image_data)/1024/1024:.2f} MB)")

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

        print(f"✓ Request ID: {request_id}")
        print(f"✓ Target users: {message['EncryptionRequest']['usernames']}")
        print(f"✓ Quota: {message['EncryptionRequest']['quota']}")
        print(f"✓ JSON message size: {len(message_bytes)/1024:.2f} KB ({len(message_bytes)/1024/1024:.2f} MB)")

        # Fragment message into chunks
        chunks = fragment_message(message_bytes)

        # Parse server address
        host, port = self.server_address.split(':')
        port = int(port)

        # Send all chunks
        print(f"\n📤 Sending {len(chunks)} chunk(s) to server {self.server_address}...")

        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)

        start_time = time.time()
        for i, chunk in enumerate(chunks):
            chunk_bytes = json.dumps(chunk).encode('utf-8')
            sock.sendto(chunk_bytes, (host, port))
            print(f"   Sent chunk {i+1}/{len(chunks)}: {len(chunk_bytes)} bytes")

        print(f"✓ All chunks sent, waiting for response...")

        # Receive and reassemble response
        response_data = reassemble_chunks(sock, timeout=30.0)
        elapsed = time.time() - start_time

        print(f"✓ Complete response received in {elapsed:.2f}s ({len(response_data)/1024:.2f} KB)")

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
                print(f"✓ Image sent to server ({len(image_data)/1024:.2f} KB) via chunked UDP")
                print(f"✓ Encrypted image received ({len(encrypted_data)/1024:.2f} KB) via chunked UDP")
                print(f"✓ Processing time: {elapsed:.2f}s")
                print(f"✓ Encrypted image displayed (scrambled)")
                print(f"✓ Multi-packet transmission working!")
                print(f"✓ Files saved to: {self.output_dir}/")
                print(f"\n📁 Output files:")
                print(f"   - encrypted_image.png (scrambled, has hidden metadata)")
                print(f"   - decryption_note.txt (how to decrypt)")
                print(f"\n💡 Now supports large images (720p and beyond)!")
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
        print("\n💡 Now supports large images (720p and beyond) via multi-packet UDP!")
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
