#!/usr/bin/env python3
"""
Analyze stress test results and generate reports
"""

import json
import sys
import argparse
from datetime import datetime
import os

def analyze_results(result_files):
    """Analyze stress test results from multiple clients"""
    
    all_results = {}
    
    # Load all result files
    for filename in result_files:
        if not os.path.exists(filename):
            print(f"WARNING: File not found: {filename}")
            continue
            
        with open(filename, 'r') as f:
            data = json.load(f)
            
        # Group by client
        if isinstance(data, list):
            for result in data:
                client_id = result['client_id']
                if client_id not in all_results:
                    all_results[client_id] = []
                all_results[client_id].append(result)
    
    if not all_results:
        print("ERROR: No valid result files found!")
        return
    
    # Print analysis
    print(f"\n{'='*80}")
    print(f"STRESS TEST ANALYSIS REPORT")
    print(f"{'='*80}")
    print(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"Clients: {len(all_results)}")
    print(f"{'='*80}\n")
    
    # Per-client analysis
    for client_id in sorted(all_results.keys()):
        results = all_results[client_id]
        print(f"\n{'─'*80}")
        print(f"CLIENT {client_id} - {len(results)} tests")
        print(f"{'─'*80}")
        print(f"{'Images':<10} {'Success':<10} {'Failures':<10} {'Rate':<10} "
              f"{'Avg Time':<15} {'Throughput':<15}")
        print(f"{'─'*80}")
        
        for result in results:
            print(f"{result['num_images']:<10} "
                  f"{result['success_count']:<10} "
                  f"{result['failure_count']:<10} "
                  f"{result['success_rate']:>6.2f}%   "
                  f"{result['avg_response_time']*1000:>8.2f}ms     "
                  f"{result['throughput']:>8.2f} req/s")
    
    # Aggregate analysis
    print(f"\n{'='*80}")
    print(f"AGGREGATE ANALYSIS (All Clients Combined)")
    print(f"{'='*80}")
    
    # Group by number of images across all clients
    by_num_images = {}
    for client_id, results in all_results.items():
        for result in results:
            num = result['num_images']
            if num not in by_num_images:
                by_num_images[num] = []
            by_num_images[num].append(result)
    
    print(f"{'Images':<10} {'Total Sent':<12} {'Successes':<12} {'Failures':<12} "
          f"{'Rate':<10} {'Avg Time':<15}")
    print(f"{'─'*80}")
    
    for num_images in sorted(by_num_images.keys()):
        results = by_num_images[num_images]
        
        total_sent = sum(r['num_images'] for r in results)
        total_success = sum(r['success_count'] for r in results)
        total_failures = sum(r['failure_count'] for r in results)
        avg_rate = sum(r['success_rate'] for r in results) / len(results)
        avg_time = sum(r['avg_response_time'] for r in results) / len(results)
        
        print(f"{num_images:<10} "
              f"{total_sent:<12} "
              f"{total_success:<12} "
              f"{total_failures:<12} "
              f"{avg_rate:>6.2f}%   "
              f"{avg_time*1000:>8.2f}ms")
    
    # Performance trends
    print(f"\n{'='*80}")
    print(f"PERFORMANCE TRENDS")
    print(f"{'='*80}")
    
    all_flat_results = []
    for results in all_results.values():
        all_flat_results.extend(results)
    
    if all_flat_results:
        best_throughput = max(all_flat_results, key=lambda x: x['throughput'])
        worst_throughput = min(all_flat_results, key=lambda x: x['throughput'])
        best_response = min(all_flat_results, key=lambda x: x['avg_response_time'])
        worst_response = max(all_flat_results, key=lambda x: x['avg_response_time'])
        
        print(f"\nBest Throughput:")
        print(f"  Client {best_throughput['client_id']}, "
              f"{best_throughput['num_images']} images: "
              f"{best_throughput['throughput']:.2f} req/s")
        
        print(f"\nWorst Throughput:")
        print(f"  Client {worst_throughput['client_id']}, "
              f"{worst_throughput['num_images']} images: "
              f"{worst_throughput['throughput']:.2f} req/s")
        
        print(f"\nBest Response Time:")
        print(f"  Client {best_response['client_id']}, "
              f"{best_response['num_images']} images: "
              f"{best_response['avg_response_time']*1000:.2f}ms")
        
        print(f"\nWorst Response Time:")
        print(f"  Client {worst_response['client_id']}, "
              f"{worst_response['num_images']} images: "
              f"{worst_response['avg_response_time']*1000:.2f}ms")
    
    # Overall statistics
    total_images_sent = sum(r['num_images'] for r in all_flat_results)
    total_successes = sum(r['success_count'] for r in all_flat_results)
    total_failures = sum(r['failure_count'] for r in all_flat_results)
    overall_success_rate = (total_successes / total_images_sent * 100) if total_images_sent > 0 else 0
    
    print(f"\n{'='*80}")
    print(f"OVERALL STATISTICS")
    print(f"{'='*80}")
    print(f"Total Images Sent: {total_images_sent:,}")
    print(f"Total Successes: {total_successes:,}")
    print(f"Total Failures: {total_failures:,}")
    print(f"Overall Success Rate: {overall_success_rate:.2f}%")
    print(f"{'='*80}\n")

def main():
    parser = argparse.ArgumentParser(description='Analyze stress test results')
    parser.add_argument('files', nargs='+', help='Result JSON files to analyze')
    
    args = parser.parse_args()
    
    analyze_results(args.files)

if __name__ == '__main__':
    main()
