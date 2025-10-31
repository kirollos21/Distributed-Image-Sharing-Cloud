#!/usr/bin/env python3
"""
Stress Test with Node Failure Simulation
Tests system behavior under load with node failures
"""

import json
import socket
import time
import sys
import os
import subprocess
import signal
from datetime import datetime
from pathlib import Path
import argparse
import threading

class FailureStressTestClient:
    def __init__(self, client_id, server_addresses, test_image_path):
        self.client_id = client_id
        self.server_addresses = server_addresses
        self.test_image_path = test_image_path
        self.results = []
        self.is_running = True
        
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
        
        # Try each server until one responds (server-side forwarding handles load balancing)
        for server_addr in self.server_addresses:
            try:
                host, port = server_addr.split(':')
                port = int(port)
                
                sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
                sock.settimeout(5.0)
                
                start_time = time.time()
                sock.sendto(message_bytes, (host, port))
                
                response_data, _ = sock.recvfrom(65535)
                elapsed = time.time() - start_time
                
                response = json.loads(response_data.decode('utf-8'))
                sock.close()
                
                return response, elapsed, server_addr
                
            except socket.timeout:
                continue
            except Exception as e:
                continue
        
        return None, None, None
    
    def run_continuous_stress(self, num_images, username, target_users, failure_event=None):
        """Run stress test while monitoring for failures"""
        print(f"\n[Client {self.client_id}] Starting continuous stress: {num_images} images")
        
        success_count = 0
        failure_count = 0
        total_time = 0
        response_times = []
        failures_during_test = []
        
        start_test_time = time.time()
        
        for i in range(num_images):
            if not self.is_running:
                break
                
            request_id = f"client_{self.client_id}_failure_test_{i}_{int(time.time())}"
            
            # Mark if a failure event is active
            failure_active = failure_event is not None and failure_event.is_set()
            
            response, elapsed, server_used = self.send_encryption_request(
                request_id,
                username,
                self.image_data,
                target_users,
                5
            )
            
            if response and response.get('EncryptionResponse', {}).get('success'):
                success_count += 1
                response_times.append(elapsed)
                total_time += elapsed
                
                if (i + 1) % 100 == 0:
                    avg_time = total_time / success_count if success_count > 0 else 0
                    fail_indicator = " [FAILURE ACTIVE]" if failure_active else ""
                    print(f"[Client {self.client_id}] Progress: {i+1}/{num_images} | "
                          f"Success: {success_count} | Failures: {failure_count} | "
                          f"Avg: {avg_time*1000:.2f}ms{fail_indicator}")
            else:
                failure_count += 1
                failures_during_test.append({
                    'request_num': i,
                    'failure_active': failure_active,
                    'timestamp': time.time()
                })
        
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
            "failures_during_test": failures_during_test,
            "timestamp": datetime.now().isoformat()
        }
        
        return result

def simulate_node_failure(node_num, duration, failure_event):
    """Simulate a node failure by marking the event"""
    print(f"\n{'!'*60}")
    print(f"! SIMULATING NODE {node_num} FAILURE FOR {duration}s")
    print(f"{'!'*60}\n")
    
    failure_event.set()
    time.sleep(duration)
    
    print(f"\n{'!'*60}")
    print(f"! NODE {node_num} RECOVERY COMPLETE")
    print(f"{'!'*60}\n")
    
    failure_event.clear()

def main():
    parser = argparse.ArgumentParser(description='Stress test with failure simulation')
    parser.add_argument('client_id', type=int, help='Client ID')
    parser.add_argument('--servers', type=str, required=True,
                       help='Comma-separated server addresses')
    parser.add_argument('--username', type=str, required=True, help='Your username')
    parser.add_argument('--target-users', type=str, required=True,
                       help='Comma-separated target usernames')
    parser.add_argument('--image', type=str, default='test_image.jpg',
                       help='Path to test image')
    parser.add_argument('--num-images', type=int, default=15000,
                       help='Number of images to send (default: 15000)')
    parser.add_argument('--failure-mode', type=str, choices=['none', 'one', 'two'], 
                       default='none',
                       help='Failure mode: none, one (1 node), two (2 nodes)')
    parser.add_argument('--failure-start', type=int, default=30,
                       help='Seconds after test start to trigger failure (default: 30)')
    parser.add_argument('--failure-duration', type=int, default=60,
                       help='Failure duration in seconds (default: 60)')
    parser.add_argument('--output', type=str, default=None,
                       help='Output file for results')
    
    args = parser.parse_args()
    
    # Parse addresses and users
    server_addresses = [addr.strip() for addr in args.servers.split(',')]
    target_users = [user.strip() for user in args.target_users.split(',')]
    
    # Default output file
    if args.output is None:
        timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
        args.output = f"failure_stress_client{args.client_id}_{args.failure_mode}_{timestamp}.json"
    
    # Check test image
    if not os.path.exists(args.image):
        print(f"ERROR: Test image not found: {args.image}")
        sys.exit(1)
    
    # Create client
    client = FailureStressTestClient(args.client_id, server_addresses, args.image)
    
    print(f"\n{'='*60}")
    print(f"FAILURE STRESS TEST CONFIGURATION")
    print(f"{'='*60}")
    print(f"Client ID: {args.client_id}")
    print(f"Username: {args.username}")
    print(f"Images to send: {args.num_images}")
    print(f"Failure Mode: {args.failure_mode}")
    if args.failure_mode != 'none':
        print(f"Failure Start: {args.failure_start}s into test")
        print(f"Failure Duration: {args.failure_duration}s")
    print(f"Output: {args.output}")
    print(f"{'='*60}\n")
    
    if args.failure_mode != 'none':
        print("IMPORTANT: You must manually stop the node(s) when prompted!")
        print(f"  - For 'one' mode: Stop 1 node")
        print(f"  - For 'two' mode: Stop 2 nodes")
        print("")
    
    print("Press Enter to start testing...")
    input()
    
    # Setup failure event
    failure_event = threading.Event()
    
    # Start failure simulation thread if needed
    if args.failure_mode != 'none':
        num_nodes = 1 if args.failure_mode == 'one' else 2
        
        def trigger_failure():
            time.sleep(args.failure_start)
            
            print(f"\n{'#'*60}")
            print(f"# MANUAL FAILURE TRIGGER")
            print(f"# NOW: Stop {num_nodes} node(s) by pressing Ctrl+C in their terminal(s)")
            print(f"# They will be considered 'failed' for {args.failure_duration}s")
            print(f"# Then restart them")
            print(f"{'#'*60}\n")
            
            simulate_node_failure(num_nodes, args.failure_duration, failure_event)
        
        failure_thread = threading.Thread(target=trigger_failure, daemon=True)
        failure_thread.start()
    else:
        failure_event = None
    
    # Run test
    result = client.run_continuous_stress(
        args.num_images,
        args.username,
        target_users,
        failure_event
    )
    
    # Add failure metadata
    result['failure_mode'] = args.failure_mode
    result['failure_start'] = args.failure_start if args.failure_mode != 'none' else None
    result['failure_duration'] = args.failure_duration if args.failure_mode != 'none' else None
    
    # Print summary
    print(f"\n{'='*60}")
    print(f"FAILURE STRESS TEST COMPLETE")
    print(f"{'='*60}")
    print(f"Mode: {args.failure_mode}")
    print(f"Images sent: {result['num_images']}")
    print(f"Successes: {result['success_count']} ({result['success_rate']:.2f}%)")
    print(f"Failures: {result['failure_count']}")
    print(f"Avg response time: {result['avg_response_time']*1000:.2f}ms")
    print(f"Throughput: {result['throughput']:.2f} req/s")
    print(f"{'='*60}\n")
    
    # Save results
    with open(args.output, 'w') as f:
        json.dump([result], f, indent=2)
    
    print(f"Results saved to: {args.output}")

if __name__ == '__main__':
    main()
