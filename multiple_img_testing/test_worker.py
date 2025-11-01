#!/usr/bin/env python3
"""
Test Worker - Handles sending images from a single process
"""

import json
import socket
import time
import random
import sys
import os
from pathlib import Path
import base64
import uuid

# Import image generator from same directory
from generate_test_image import generate_test_image


# Chunking functions (same as single_img_testing)
CHUNK_SIZE = 45000

def fragment_message(data):
    """Fragment a large message into chunks using base64 encoding"""
    data_bytes = data if isinstance(data, bytes) else data.encode('utf-8')
    data_len = len(data_bytes)

    if data_len <= CHUNK_SIZE:
        encoded = base64.b64encode(data_bytes).decode('ascii')
        return [{"SinglePacket": encoded}]

    total_chunks = (data_len + CHUNK_SIZE - 1) // CHUNK_SIZE
    chunk_id = str(uuid.uuid4())

    chunks = []
    for chunk_index in range(total_chunks):
        start = chunk_index * CHUNK_SIZE
        end = min(start + CHUNK_SIZE, data_len)
        chunk_data = data_bytes[start:end]
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
    """Receive and reassemble chunked messages with automatic retransmission requests"""
    sock.settimeout(5.0)  # Use short timeout to detect when stream ends

    incomplete = {}
    chunk_id_tracker = None
    server_addr = None
    start_time = time.time()
    retransmit_attempts = 0
    MAX_RETRANSMIT_ATTEMPTS = 3

    while True:
        if time.time() - start_time > timeout:
            raise socket.timeout("Overall timeout exceeded")
            
        try:
            # Receive chunk
            chunk_data, addr = sock.recvfrom(65535)
            if server_addr is None:
                server_addr = addr
            chunk_msg = json.loads(chunk_data.decode('utf-8'))

            # Check if single packet
            if "SinglePacket" in chunk_msg:
                encoded = chunk_msg["SinglePacket"]
                decoded = base64.b64decode(encoded)
                return decoded

            # Multi-packet
            if "MultiPacket" in chunk_msg:
                mp = chunk_msg["MultiPacket"]
                chunk_id = mp["chunk_id"]
                chunk_index = mp["chunk_index"]
                total_chunks = mp["total_chunks"]
                
                chunk_id_tracker = chunk_id

                # Base64 decode the chunk data
                encoded_data = mp["data"]
                data = base64.b64decode(encoded_data)

                # Store chunk
                if chunk_id not in incomplete:
                    incomplete[chunk_id] = {"total": total_chunks, "chunks": {}}

                incomplete[chunk_id]["chunks"][chunk_index] = data

                # Check if complete
                if len(incomplete[chunk_id]["chunks"]) == total_chunks:
                    # Reassemble in order
                    complete_data = b""
                    for i in range(total_chunks):
                        complete_data += incomplete[chunk_id]["chunks"][i]

                    return complete_data
                    
        except socket.timeout:
            # Timeout means no more packets are coming - check if we're missing any
            if chunk_id_tracker and chunk_id_tracker in incomplete and server_addr:
                received = len(incomplete[chunk_id_tracker]["chunks"])
                total = incomplete[chunk_id_tracker]["total"]
                
                if received < total and retransmit_attempts < MAX_RETRANSMIT_ATTEMPTS:
                    # Find missing chunks
                    missing = [i for i in range(total) if i not in incomplete[chunk_id_tracker]["chunks"]]
                    
                    retransmit_attempts += 1
                    
                    # Send retransmit request
                    retransmit_request = {
                        "RetransmitRequest": {
                            "chunk_id": chunk_id_tracker,
                            "missing_indices": missing[:50]  # Limit to first 50 to avoid huge request
                        }
                    }
                    request_bytes = json.dumps(retransmit_request).encode('utf-8')
                    sock.sendto(request_bytes, server_addr)
                    
                    # Reset timeout and continue receiving
                    sock.settimeout(5.0)
                    continue
                elif received < total:
                    # Max retransmits reached
                    missing = [i for i in range(total) if i not in incomplete[chunk_id_tracker]["chunks"]]
                    raise Exception(f"UDP packet loss: missing {len(missing)}/{total} chunks after {retransmit_attempts} retries")
            raise


class TestWorker:
    def __init__(self, process_id, config):
        self.process_id = process_id
        self.config = config
        self.servers = config['server_config']['servers']
        self.timeout = config['server_config']['request_timeout']
        self.retry_attempts = config['server_config']['retry_attempts']

        self.results = []

    def send_image_to_server(self, image_data, image_id, server_address):
        """Send image to server and get encrypted version"""
        request_id = f"P{self.process_id:02d}_I{image_id:03d}_{int(time.time())}"

        message = {
            "EncryptionRequest": {
                "request_id": request_id,
                "client_username": f"worker_{self.process_id}",
                "image_data": list(image_data),
                "usernames": self.config['encryption_config']['usernames'],
                "quota": self.config['encryption_config']['quota'],
                "forwarded": False
            }
        }

        message_bytes = json.dumps(message).encode('utf-8')
        chunks = fragment_message(message_bytes)

        host, port = server_address.split(':')
        port = int(port)

        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)

        start_time = time.time()

        try:
            # Send all chunks
            for chunk in chunks:
                chunk_bytes = json.dumps(chunk).encode('utf-8')
                sock.sendto(chunk_bytes, (host, port))

            # Receive response
            response_data = reassemble_chunks(sock, timeout=self.timeout)
            elapsed = time.time() - start_time

            response = json.loads(response_data.decode('utf-8'))

            if response.get('EncryptionResponse', {}).get('success'):
                encrypted_data = bytes(response['EncryptionResponse']['encrypted_image'])
                return {
                    'success': True,
                    'encrypted_data': encrypted_data,
                    'latency': elapsed,
                    'server': server_address,
                    'request_id': request_id
                }
            else:
                error = response.get('EncryptionResponse', {}).get('error', 'Unknown')
                return {
                    'success': False,
                    'error': error,
                    'latency': elapsed,
                    'server': server_address,
                    'request_id': request_id
                }

        except socket.timeout:
            return {
                'success': False,
                'error': 'Timeout',
                'latency': time.time() - start_time,
                'server': server_address,
                'request_id': request_id
            }
        except Exception as e:
            return {
                'success': False,
                'error': str(e),
                'latency': time.time() - start_time,
                'server': server_address,
                'request_id': request_id
            }
        finally:
            sock.close()

    # def decrypt_image(self, encrypted_data, image_id, server_address):
    #     """Send encrypted image to server and get decrypted version"""
    #     request_id = f"P{self.process_id:02d}_I{image_id:03d}_DEC_{int(time.time())}"

    #     message = {
    #         "DecryptionRequest": {
    #             "request_id": request_id,
    #             "client_username": f"worker_{self.process_id}",
    #             "encrypted_image": list(encrypted_data),
    #             "usernames": self.config['encryption_config']['usernames'],
    #             "quota": self.config['encryption_config']['quota']
    #         }
    #     }

    #     message_bytes = json.dumps(message).encode('utf-8')
    #     chunks = fragment_message(message_bytes)

    #     host, port = server_address.split(':')
    #     port = int(port)

    #     sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)

    #     start_time = time.time()

    #     try:
    #         # Send all chunks
    #         for chunk in chunks:
    #             chunk_bytes = json.dumps(chunk).encode('utf-8')
    #             sock.sendto(chunk_bytes, (host, port))

    #         # Receive response
    #         response_data = reassemble_chunks(sock, timeout=self.timeout)
    #         elapsed = time.time() - start_time

    #         response = json.loads(response_data.decode('utf-8'))

    #         if response.get('DecryptionResponse', {}).get('success'):
    #             decrypted_data = bytes(response['DecryptionResponse']['decrypted_image'])
    #             return {
    #                 'success': True,
    #                 'decrypted_data': decrypted_data,
    #                 'latency': elapsed,
    #                 'server': server_address,
    #                 'request_id': request_id
    #             }
    #         else:
    #             error = response.get('DecryptionResponse', {}).get('error', 'Unknown')
    #             return {
    #                 'success': False,
    #                 'error': error,
    #                 'latency': elapsed,
    #                 'server': server_address,
    #                 'request_id': request_id
    #             }

    #     except socket.timeout:
    #         return {
    #             'success': False,
    #             'error': 'Timeout',
    #             'latency': time.time() - start_time,
    #             'server': server_address,
    #             'request_id': request_id
    #         }
    #     except Exception as e:
    #         return {
    #             'success': False,
    #             'error': str(e),
    #             'latency': time.time() - start_time,
    #             'server': server_address,
    #             'request_id': request_id
    #         }
    #     finally:
    #         sock.close()

    def run(self):
        """Run the worker process"""
        print(f"[Process {self.process_id}] Starting...")

        images_per_process = self.config['test_config']['images_per_process']
        width = self.config['test_config']['image_width']
        height = self.config['test_config']['image_height']
        quality = self.config['test_config']['image_quality']

        output_dir = Path(self.config['output_config']['output_dir'])
        test_images_dir = Path(self.config['output_config']['test_images_dir'])
        encrypted_dir = Path(self.config['output_config']['encrypted_dir'])

        process_start = time.time()

        for img_id in range(1, images_per_process + 1):
            img_start = time.time()

            # Generate unique test image with random dimensions (300x300 to 600x600)
            print(f"[Process {self.process_id}] Generating image {img_id}/{images_per_process}...")
            image_data = generate_test_image(
                self.process_id,
                img_id,
                width,
                height,
                quality,
                random_size=True,
                max_width=600,
                max_height=600
            )

            # Save test image if configured
            if self.config['output_config']['save_test_images']:
                test_img_path = test_images_dir / f"test_image_{self.process_id}_{img_id}.jpg"
                with open(test_img_path, 'wb') as f:
                    f.write(image_data)

            # Select random server
            server = random.choice(self.servers)

            # Send with retries
            result = None
            for attempt in range(1, self.retry_attempts + 1):
                print(f"[Process {self.process_id}] Sending image {img_id} to {server} (attempt {attempt})...")

                result = self.send_image_to_server(image_data, img_id, server)

                if result['success']:
                    print(f"[Process {self.process_id}] ✓ Image {img_id} encrypted successfully ({result['latency']:.2f}s)")

                    # Save encrypted image
                    if self.config['output_config']['save_encrypted_images']:
                        enc_path = encrypted_dir / f"encrypted_{self.process_id}_{img_id}.png"
                        with open(enc_path, 'wb') as f:
                            f.write(result['encrypted_data'])

                    # # Save encrypted image (decryption will be performed in batch later)
                    # if self.config['output_config']['save_encrypted_images']:
                    #     enc_path = encrypted_dir / f"encrypted_{self.process_id}_{img_id}.png"
                    #     with open(enc_path, 'wb') as f:
                    #         f.write(result['encrypted_data'])

                    # # Do not decrypt immediately here to avoid duplicate decryption
                    # # Decryption will be handled by the post-test step `decrypt_all.py`
                    # result['decryption_scheduled'] = True

                    break
                else:
                    print(f"[Process {self.process_id}] ✗ Image {img_id} failed: {result['error']}")
                    if attempt < self.retry_attempts:
                        time.sleep(1)  # Wait before retry

            # Record result
            result['process_id'] = self.process_id
            result['image_id'] = img_id
            result['image_size'] = len(image_data)
            result['total_time'] = time.time() - img_start
            self.results.append(result)

        process_elapsed = time.time() - process_start

        # Calculate process statistics
        successes = sum(1 for r in self.results if r['success'])
        failures = len(self.results) - successes
        # decryption_successes = sum(1 for r in self.results if r.get('decryption_success', False))
        # decryption_failures = successes - decryption_successes  # Only count if encryption succeeded

        print(f"\n[Process {self.process_id}] COMPLETE")
        print(f"  Total time: {process_elapsed:.2f}s")
        print(f"  Encryption success: {successes}/{images_per_process}")
        print(f"  Encryption failure: {failures}/{images_per_process}")
        # print(f"  Decryption success: {decryption_successes}/{successes}")
        # print(f"  Decryption failure: {decryption_failures}/{successes}")
        # print(f"  Overall success rate: {(decryption_successes/images_per_process*100):.1f}%")

        return {
            'process_id': self.process_id,
            'results': self.results,
            'total_time': process_elapsed,
            'success_count': successes,
            # 'failure_count': failures,
            # 'decryption_success_count': decryption_successes,
            # 'decryption_failure_count': decryption_failures
        }


if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: python3 test_worker.py <process_id>")
        sys.exit(1)

    process_id = int(sys.argv[1])

    # Load config
    with open('config.json', 'r') as f:
        config = json.load(f)

    worker = TestWorker(process_id, config)
    result = worker.run()

    # Save results
    output_dir = Path(config['output_config']['metrics_dir'])
    result_file = output_dir / f"process_{process_id}_results.json"
    with open(result_file, 'w') as f:
        json.dump(result, f, indent=2)

    print(f"[Process {process_id}] Results saved to {result_file}")
