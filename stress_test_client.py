#!/usr/bin/env python3
"""
Distributed Image Cloud - Stress Testing Client
Sends multiple images to test system performance and failure tolerance
"""

import json
import socket
import time
import sys
import os
from datetime import datetime
from pathlib import Path
import argparse

class StressTestClient:
    def __init__(self, client_id, server_addresses, test_image_path):
        self.client_id = client_id
        self.server_addresses = server_addresses
        self.test_image_path = test_image_path
        self.results = []
        
        # Load test image
        with open(test_image_path, 'rb') as f:
            self.image_data = f.read()
        
        print(f"[Client {client_id}] Loaded test image: {len(self.image_data)} bytes")
    
    def send_encryption_request(self, request_id, username, image_data, target_users, quota):
        """Send a single encryption request to the cloud"""
        message = {
            "EncryptionRequest": {
                "request_id": request_id,
                "client_username": username,
                "image_data": list(image_data),
                "usernames": target_users,
                "quota": quota
            }
        }
        
        message_bytes = json.dumps(message).encode('utf-8')
        
        # Try each server until one responds
        for server_addr in self.server_addresses:
            try:
                host, port = server_addr.split(':')
                port = int(port)
                
                sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
                sock.settimeout(5.0)  # 5 second timeout
                
                start_time = time.time()
                sock.sendto(message_bytes, (host, port))
                
                # Wait for response
                response_data, _ = sock.recvfrom(65535)
                elapsed = time.time() - start_time
                
                response = json.loads(response_data.decode('utf-8'))
                sock.close()
                
                return response, elapsed, server_addr
                
            except socket.timeout:
                print(f"[Client {self.client_id}] Timeout on {server_addr}, trying next...")
                continue
            except Exception as e:
                print(f"[Client {self.client_id}] Error with {server_addr}: {e}")
                continue
        
        return None, None, None
    
    def run_stress_test(self, num_images, username, target_users, output_file):
        """Run stress test by sending num_images requests"""
        print(f"\n{'='*60}")
        print(f"[Client {self.client_id}] Starting stress test: {num_images} images")
        print(f"[Client {self.client_id}] Username: {username}")
        print(f"[Client {self.client_id}] Target users: {target_users}")
        print(f"{'='*60}\n")
        
        success_count = 0
        failure_count = 0
        total_time = 0
        response_times = []
        
        start_test_time = time.time()
        
        for i in range(num_images):
            request_id = f"client_{self.client_id}_stress_{num_images}_{i}_{int(time.time())}"
            
            response, elapsed, server_used = self.send_encryption_request(
                request_id,
                username,
                self.image_data,
                target_users,
                5  # Default quota
            )
            
            if response and response.get('EncryptionResponse', {}).get('success'):
                success_count += 1
                response_times.append(elapsed)
                total_time += elapsed
                
                # Progress update every 100 images
                if (i + 1) % 100 == 0:
                    avg_time = total_time / success_count if success_count > 0 else 0
                    print(f"[Client {self.client_id}] Progress: {i+1}/{num_images} | "
                          f"Success: {success_count} | Failures: {failure_count} | "
                          f"Avg: {avg_time*1000:.2f}ms")
            else:
                failure_count += 1
                error = response.get('EncryptionResponse', {}).get('error', 'Unknown') if response else 'No response'
                print(f"[Client {self.client_id}] FAILED request {i+1}: {error}")
        
        end_test_time = time.time()
        total_test_time = end_test_time - start_test_time
        
        # Calculate statistics
        avg_response_time = total_time / success_count if success_count > 0 else 0
        min_response_time = min(response_times) if response_times else 0
        max_response_time = max(response_times) if response_times else 0
        throughput = success_count / total_test_time if total_test_time > 0 else 0
        success_rate = (success_count / num_images) * 100
        
        result = {
            "client_id": self.client_id,
            "num_images": num_images,
            "success_count": success_count,
            "failure_count": failure_count,
            "success_rate": success_rate,
            "total_test_time": total_test_time,
            "avg_response_time": avg_response_time,
            "min_response_time": min_response_time,
            "max_response_time": max_response_time,
            "throughput": throughput,
            "timestamp": datetime.now().isoformat()
        }
        
        # Print summary
        print(f"\n{'='*60}")
        print(f"[Client {self.client_id}] STRESS TEST COMPLETE - {num_images} images")
        print(f"{'='*60}")
        print(f"Success: {success_count}/{num_images} ({success_rate:.2f}%)")
        print(f"Failures: {failure_count}")
        print(f"Total time: {total_test_time:.2f}s")
        print(f"Avg response time: {avg_response_time*1000:.2f}ms")
        print(f"Min response time: {min_response_time*1000:.2f}ms")
        print(f"Max response time: {max_response_time*1000:.2f}ms")
        print(f"Throughput: {throughput:.2f} requests/sec")
        print(f"{'='*60}\n")
        
        # Save results
        self.results.append(result)
        self.save_results(output_file)
        
        return result
    
    def save_results(self, output_file):
        """Save test results to JSON file"""
        with open(output_file, 'w') as f:
            json.dump(self.results, f, indent=2)
        print(f"[Client {self.client_id}] Results saved to: {output_file}")

def main():
    parser = argparse.ArgumentParser(description='Stress test the distributed image cloud')
    parser.add_argument('client_id', type=int, help='Client ID (1, 2, or 3)')
    parser.add_argument('--servers', type=str, required=True, 
                       help='Comma-separated server addresses (e.g., 192.168.4.2:8001,192.168.4.3:8002)')
    parser.add_argument('--username', type=str, required=True, help='Your username')
    parser.add_argument('--target-users', type=str, required=True,
                       help='Comma-separated target usernames')
    parser.add_argument('--image', type=str, default='test_image.jpg',
                       help='Path to test image')
    parser.add_argument('--start', type=int, default=10000,
                       help='Starting number of images (default: 10000)')
    parser.add_argument('--end', type=int, default=20000,
                       help='Ending number of images (default: 20000)')
    parser.add_argument('--step', type=int, default=2500,
                       help='Step size (default: 2500, gives 10k, 12.5k, 15k, 17.5k, 20k)')
    parser.add_argument('--gap', type=int, default=10,
                       help='Seconds to wait between tests (default: 10)')
    parser.add_argument('--output', type=str, default=None,
                       help='Output file for results (default: stress_results_clientX.json)')
    
    args = parser.parse_args()
    
    # Parse server addresses
    server_addresses = [addr.strip() for addr in args.servers.split(',')]
    
    # Parse target users
    target_users = [user.strip() for user in args.target_users.split(',')]
    
    # Default output file
    if args.output is None:
        timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
        args.output = f"stress_results_client{args.client_id}_{timestamp}.json"
    
    # Check if test image exists
    if not os.path.exists(args.image):
        print(f"ERROR: Test image not found: {args.image}")
        print(f"Create one with: python3 create_test_image.py")
        sys.exit(1)
    
    # Create client
    client = StressTestClient(args.client_id, server_addresses, args.image)
    
    # Calculate test points
    test_points = []
    current = args.start
    while current <= args.end:
        test_points.append(current)
        current += args.step
    
    print(f"\n{'='*60}")
    print(f"STRESS TEST CONFIGURATION")
    print(f"{'='*60}")
    print(f"Client ID: {args.client_id}")
    print(f"Username: {args.username}")
    print(f"Target Users: {target_users}")
    print(f"Servers: {server_addresses}")
    print(f"Test Points: {test_points}")
    print(f"Gap Between Tests: {args.gap}s")
    print(f"Output File: {args.output}")
    print(f"{'='*60}\n")
    
    print("Press Enter to start testing...")
    input()
    
    # Run tests
    for i, num_images in enumerate(test_points):
        print(f"\n\n{'#'*60}")
        print(f"# TEST {i+1}/{len(test_points)}: {num_images} images")
        print(f"{'#'*60}\n")
        
        client.run_stress_test(num_images, args.username, target_users, args.output)
        
        # Wait before next test (except after last test)
        if i < len(test_points) - 1:
            print(f"\nWaiting {args.gap} seconds before next test...")
            for remaining in range(args.gap, 0, -1):
                print(f"  {remaining}...", end='\r')
                time.sleep(1)
            print("\n")
    
    print(f"\n{'='*60}")
    print(f"ALL TESTS COMPLETE!")
    print(f"{'='*60}")
    print(f"Results saved to: {args.output}")
    print(f"Run analysis with: python3 analyze_stress_results.py {args.output}")
    print(f"{'='*60}\n")

if __name__ == '__main__':
    main()
