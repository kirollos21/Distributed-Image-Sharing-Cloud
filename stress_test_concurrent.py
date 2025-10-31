#!/usr/bin/env python3
"""
Distributed Image Cloud - Concurrent Stress Testing Client
Runs multiple concurrent clients (threads) to simulate real-world load
"""

import json
import socket
import time
import sys
import os
from datetime import datetime
import argparse
import threading
from statistics import mean, median, stdev
import math

class ConcurrentStressTest:
    def __init__(self, server_addresses, test_image_path, username, target_users):
        self.server_addresses = server_addresses
        self.test_image_path = test_image_path
        self.username = username
        self.target_users = target_users
        
        # Thread-safe results collection
        self.results_lock = threading.Lock()
        self.all_results = []
        self.success_count = 0
        self.failure_count = 0
        
        # Load test image
        with open(test_image_path, 'rb') as f:
            self.image_data = f.read()
        
        print(f"Loaded test image: {len(self.image_data)} bytes")
    
    def send_encryption_request(self, request_id, image_data):
        """Send a single encryption request to the cloud"""
        message = {
            "EncryptionRequest": {
                "request_id": request_id,
                "client_username": self.username,
                "image_data": list(image_data),
                "usernames": self.target_users,
                "quota": 5,
                "forwarded": False
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
                continue
            except Exception as e:
                continue
        
        return None, None, None
    
    def client_thread(self, client_id, num_requests_per_client):
        """Thread function: each client sends num_requests_per_client requests"""
        thread_results = []
        
        for i in range(num_requests_per_client):
            request_id = f"{self.username}_client{client_id}_req{i}_{int(time.time()*1000000)}"
            
            response, elapsed, server_used = self.send_encryption_request(
                request_id,
                self.image_data
            )
            
            result = {
                'client_id': client_id,
                'request_num': i,
                'success': False,
                'response_time': elapsed,
                'server': server_used,
                'timestamp': time.time()
            }
            
            if response and response.get('EncryptionResponse', {}).get('success'):
                result['success'] = True
                result['message'] = response.get('EncryptionResponse', {}).get('message', '')
                
                with self.results_lock:
                    self.success_count += 1
            else:
                error = response.get('EncryptionResponse', {}).get('error', 'Unknown') if response else 'No response'
                result['error'] = error
                
                with self.results_lock:
                    self.failure_count += 1
            
            thread_results.append(result)
            
            # Small delay between requests from same client to avoid overwhelming
            if i < num_requests_per_client - 1:
                time.sleep(0.01)  # 10ms delay
        
        # Add all results to global list
        with self.results_lock:
            self.all_results.extend(thread_results)
    
    def run_concurrent_test(self, num_clients, num_requests_per_client):
        """Run concurrent stress test with multiple client threads"""
        print(f"\n{'='*70}")
        print(f"CONCURRENT STRESS TEST")
        print(f"{'='*70}")
        print(f"Number of Clients: {num_clients}")
        print(f"Requests per Client: {num_requests_per_client}")
        print(f"Total Requests: {num_clients * num_requests_per_client}")
        print(f"Username: {self.username}")
        print(f"Target Users: {self.target_users}")
        print(f"Servers: {self.server_addresses}")
        print(f"{'='*70}\n")
        
        # Reset counters
        self.all_results = []
        self.success_count = 0
        self.failure_count = 0
        
        # Create threads
        threads = []
        start_time = time.time()
        
        print(f"Starting {num_clients} concurrent clients...\n")
        
        for client_id in range(num_clients):
            thread = threading.Thread(
                target=self.client_thread,
                args=(client_id, num_requests_per_client)
            )
            threads.append(thread)
            thread.start()
            
            # Small delay between thread starts to avoid thundering herd
            if (client_id + 1) % 50 == 0:
                time.sleep(0.01)
                print(f"Started {client_id + 1}/{num_clients} clients...")
        
        # Wait for all threads to complete
        print(f"\nWaiting for all clients to complete...\n")
        
        completed = 0
        while completed < num_clients:
            alive_count = sum(1 for t in threads if t.is_alive())
            new_completed = num_clients - alive_count
            
            if new_completed > completed:
                completed = new_completed
                if completed % 10 == 0 or completed == num_clients:
                    print(f"Progress: {completed}/{num_clients} clients completed")
            
            time.sleep(0.1)
        
        # Join all threads
        for thread in threads:
            thread.join()
        
        end_time = time.time()
        total_test_time = end_time - start_time
        
        # Calculate statistics
        total_requests = num_clients * num_requests_per_client
        success_rate = (self.success_count / total_requests) * 100 if total_requests > 0 else 0
        throughput = self.success_count / total_test_time if total_test_time > 0 else 0
        
        # Response time statistics (only successful requests)
        successful_response_times = [r['response_time'] for r in self.all_results if r['success'] and r['response_time']]
        
        if successful_response_times:
            avg_response = mean(successful_response_times)
            median_response = median(successful_response_times)
            min_response = min(successful_response_times)
            max_response = max(successful_response_times)
            stdev_response = stdev(successful_response_times) if len(successful_response_times) > 1 else 0
            
            # Percentiles
            sorted_times = sorted(successful_response_times)
            p95_idx = int(len(sorted_times) * 0.95)
            p99_idx = int(len(sorted_times) * 0.99)
            p95 = sorted_times[p95_idx] if p95_idx < len(sorted_times) else sorted_times[-1]
            p99 = sorted_times[p99_idx] if p99_idx < len(sorted_times) else sorted_times[-1]
        else:
            avg_response = median_response = min_response = max_response = stdev_response = p95 = p99 = 0
        
        # Server distribution
        server_counts = {}
        for result in self.all_results:
            if result['success'] and result['server']:
                server_counts[result['server']] = server_counts.get(result['server'], 0) + 1
        
        # Print results
        print(f"\n{'='*70}")
        print(f"TEST COMPLETE - {num_clients} clients × {num_requests_per_client} requests = {total_requests} total")
        print(f"{'='*70}")
        print(f"\nSUCCESS METRICS:")
        print(f"  Success: {self.success_count}/{total_requests} ({success_rate:.2f}%)")
        print(f"  Failures: {self.failure_count}")
        print(f"\nTIMING METRICS:")
        print(f"  Total test time: {total_test_time:.2f}s")
        print(f"  Throughput: {throughput:.2f} requests/sec")
        
        if successful_response_times:
            print(f"\nRESPONSE TIME STATISTICS:")
            print(f"  Average: {avg_response*1000:.2f}ms")
            print(f"  Median: {median_response*1000:.2f}ms")
            print(f"  Min: {min_response*1000:.2f}ms")
            print(f"  Max: {max_response*1000:.2f}ms")
            print(f"  Std Dev: {stdev_response*1000:.2f}ms")
            print(f"  P95: {p95*1000:.2f}ms")
            print(f"  P99: {p99*1000:.2f}ms")
        
        if server_counts:
            print(f"\nSERVER DISTRIBUTION:")
            for server, count in sorted(server_counts.items()):
                percentage = (count / self.success_count) * 100 if self.success_count > 0 else 0
                print(f"  {server}: {count} requests ({percentage:.1f}%)")
        
        print(f"{'='*70}\n")
        
        return {
            'num_clients': num_clients,
            'num_requests_per_client': num_requests_per_client,
            'total_requests': total_requests,
            'success_count': self.success_count,
            'failure_count': self.failure_count,
            'success_rate': success_rate,
            'total_test_time': total_test_time,
            'throughput': throughput,
            'avg_response_time': avg_response,
            'median_response_time': median_response,
            'min_response_time': min_response,
            'max_response_time': max_response,
            'stdev_response_time': stdev_response,
            'p95_response_time': p95,
            'p99_response_time': p99,
            'server_distribution': server_counts,
            'timestamp': datetime.now().isoformat(),
            'detailed_results': self.all_results
        }
    
    def save_results(self, result, output_file):
        """Save test results to JSON file"""
        with open(output_file, 'w') as f:
            json.dump(result, f, indent=2)
        print(f"Results saved to: {output_file}")


def main():
    parser = argparse.ArgumentParser(description='Concurrent stress test for distributed image cloud')
    parser.add_argument('--clients', type=int, required=True,
                       help='Number of concurrent clients (threads)')
    parser.add_argument('--requests-per-client', type=int, required=True,
                       help='Number of requests each client should send')
    parser.add_argument('--servers', type=str, required=True,
                       help='Comma-separated server addresses (e.g., 192.168.4.2:8001,192.168.4.3:8002)')
    parser.add_argument('--username', type=str, required=True,
                       help='Your username')
    parser.add_argument('--target-users', type=str, required=True,
                       help='Comma-separated target usernames')
    parser.add_argument('--image', type=str, default='test_image.jpg',
                       help='Path to test image')
    parser.add_argument('--output', type=str, default=None,
                       help='Output file for results (default: auto-generated)')
    
    args = parser.parse_args()
    
    # Parse server addresses
    server_addresses = [addr.strip() for addr in args.servers.split(',')]
    
    # Parse target users
    target_users = [user.strip() for user in args.target_users.split(',')]
    
    # Default output file
    if args.output is None:
        timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
        args.output = f"stress_results_concurrent_{args.clients}clients_{args.requests_per_client}req_{timestamp}.json"
    
    # Check if test image exists
    if not os.path.exists(args.image):
        print(f"ERROR: Test image not found: {args.image}")
        print(f"Create one with: python3 create_test_image.py")
        sys.exit(1)
    
    # Create tester and run
    tester = ConcurrentStressTest(server_addresses, args.image, args.username, target_users)
    result = tester.run_concurrent_test(args.clients, args.requests_per_client)
    tester.save_results(result, args.output)
    
    print(f"\nTo analyze results, run:")
    print(f"  python3 -c \"import json; data=json.load(open('{args.output}')); print(json.dumps({{k:v for k,v in data.items() if k!='detailed_results'}}, indent=2))\"")


if __name__ == '__main__':
    main()
