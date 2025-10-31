#!/usr/bin/env python3
"""
Quick Stress Test - 10 Concurrent Clients
Tests basic functionality with 10 simultaneous clients sending requests
"""

import json
import socket
import time
import sys
import statistics
import threading
from datetime import datetime

class QuickStressTest:
    def __init__(self, server_addresses, test_image_path='test_image.jpg'):
        self.server_addresses = server_addresses
        self.results_lock = threading.Lock()
        self.all_results = []
        
        # Load test image
        with open(test_image_path, 'rb') as f:
            self.image_data = f.read()
        
        print(f"\n{'='*70}")
        print(f"QUICK STRESS TEST - 10 CONCURRENT CLIENTS")
        print(f"{'='*70}")
        print(f"Test image size: {len(self.image_data):,} bytes")
        print(f"Servers: {', '.join(server_addresses)}")
        print(f"Concurrent clients: 10")
        print(f"{'='*70}\n")
    
    def send_request(self, client_id, request_id, username, target_users):
        """Send a single encryption request"""
        message = {
            "EncryptionRequest": {
                "request_id": request_id,
                "client_username": username,
                "image_data": list(self.image_data),
                "usernames": target_users,
                "quota": 5,
                "forwarded": False
            }
        }
        
        message_bytes = json.dumps(message).encode('utf-8')
        
        # Try first server
        for server_addr in self.server_addresses:
            try:
                host, port = server_addr.split(':')
                port = int(port)
                
                sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
                sock.settimeout(10.0)
                
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
    
    def client_thread(self, client_id, username, target_users):
        """Thread function for each concurrent client"""
        request_id = f"client_{client_id}_stress_{int(time.time() * 1000)}"
        
        start_time = time.time()
        response, elapsed, server_used = self.send_request(client_id, request_id, username, target_users)
        
        result = {
            "client_id": client_id,
            "request_id": request_id,
            "success": response and response.get('EncryptionResponse', {}).get('success', False),
            "elapsed": elapsed,
            "server": server_used,
            "timestamp": time.time()
        }
        
        with self.results_lock:
            self.all_results.append(result)
            
        if result["success"]:
            print(f"Client {client_id:2d}: ✅ Success in {elapsed*1000:6.2f}ms via {server_used}")
        else:
            error = response.get('EncryptionResponse', {}).get('error', 'Unknown') if response else 'No response'
            print(f"Client {client_id:2d}: ❌ Failed: {error}")
    
    def run_test(self, username, target_users):
        """Run stress test with 10 concurrent clients"""
        num_clients = 10
        
        start_test_time = time.time()
        
        print(f"Starting test at {datetime.now().strftime('%H:%M:%S')}")
        print(f"Username: {username}")
        print(f"Target users: {', '.join(target_users)}\n")
        
        # Create and start all client threads
        threads = []
        for i in range(num_clients):
            thread = threading.Thread(target=self.client_thread, args=(i+1, username, target_users))
            threads.append(thread)
            thread.start()
        
        # Wait for all threads to complete
        for thread in threads:
            thread.join()
        
        end_test_time = time.time()
        total_time = end_test_time - start_test_time
        
        # Analyze results
        success_count = sum(1 for r in self.all_results if r["success"])
        failure_count = num_clients - success_count
        response_times = [r["elapsed"] for r in self.all_results if r["success"]]
        server_distribution = {}
        
        for result in self.all_results:
            if result["success"] and result["server"]:
                server_distribution[result["server"]] = server_distribution.get(result["server"], 0) + 1
        
        # Calculate detailed metrics
        num_clients = 10
        success_rate = (success_count / num_clients) * 100
        throughput = success_count / total_time if total_time > 0 else 0
        
        print(f"\n{'='*70}")
        print(f"TEST RESULTS - 10 CONCURRENT CLIENTS")
        print(f"{'='*70}")
        print(f"\n📊 SUCCESS METRICS:")
        print(f"   Success:        {success_count:2d}/{num_clients} ({success_rate:.1f}%)")
        print(f"   Failures:       {failure_count:2d}")
        
        if response_times:
            avg_time = statistics.mean(response_times)
            min_time = min(response_times)
            max_time = max(response_times)
            median_time = statistics.median(response_times)
            stdev_time = statistics.stdev(response_times) if len(response_times) > 1 else 0
            
            print(f"\n⏱️  RESPONSE TIME METRICS:")
            print(f"   Average:        {avg_time*1000:7.2f}ms")
            print(f"   Median:         {median_time*1000:7.2f}ms")
            print(f"   Min:            {min_time*1000:7.2f}ms")
            print(f"   Max:            {max_time*1000:7.2f}ms")
            print(f"   Std Dev:        {stdev_time*1000:7.2f}ms")
        
        print(f"\n🚀 THROUGHPUT:")
        print(f"   Total time:     {total_time:7.2f}s")
        print(f"   Throughput:     {throughput:7.2f} req/s")
        
        if server_distribution:
            print(f"\n🌐 SERVER DISTRIBUTION:")
            for server, count in sorted(server_distribution.items()):
                percentage = (count / success_count) * 100
                print(f"   {server:20s}: {count:2d} requests ({percentage:5.1f}%)")
        
        print(f"\n{'='*70}\n")
        
        return {
            "total_clients": 10,
            "success": success_count,
            "failures": failure_count,
            "success_rate": success_rate,
            "total_time": total_time,
            "throughput": throughput,
            "response_times": {
                "avg_ms": avg_time * 1000 if response_times else 0,
                "median_ms": median_time * 1000 if response_times else 0,
                "min_ms": min_time * 1000 if response_times else 0,
                "max_ms": max_time * 1000 if response_times else 0,
                "stdev_ms": stdev_time * 1000 if response_times else 0
            },
            "server_distribution": server_distribution
        }

if __name__ == "__main__":
    if len(sys.argv) < 4:
        print("Usage: python3 stress_test_10.py <servers> <username> <target_users>")
        print("Example: python3 stress_test_10.py 127.0.0.1:8001,127.0.0.1:8002 alice bob,charlie")
        sys.exit(1)
    
    servers = sys.argv[1].split(',')
    username = sys.argv[2]
    target_users = sys.argv[3].split(',')
    
    tester = QuickStressTest(servers)
    result = tester.run_test(username, target_users)
    
    # Save results
    with open('stress_test_10_results.json', 'w') as f:
        json.dump(result, f, indent=2)
    print(f"Results saved to: stress_test_10_results.json")
