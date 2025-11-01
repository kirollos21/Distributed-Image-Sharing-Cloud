#!/usr/bin/env python3
"""
Test Coordinator - Spawns multiple worker processes and collects results
"""

import json
import multiprocessing
import time
import sys
from pathlib import Path
from test_worker import TestWorker


def run_worker(process_id, config, return_dict):
    """Run a worker and store results"""
    try:
        worker = TestWorker(process_id, config)
        result = worker.run()
        return_dict[process_id] = result
    except Exception as e:
        print(f"[Process {process_id}] ERROR: {e}")
        import traceback
        traceback.print_exc()
        return_dict[process_id] = {
            'process_id': process_id,
            'error': str(e),
            'success_count': 0,
            'failure_count': config['test_config']['images_per_process'],
            'results': []
        }


def main():
    print("=" * 80)
    print(" MULTIPLE IMAGE STRESS TEST - COORDINATOR")
    print("=" * 80)

    # Load configuration
    config_path = Path(__file__).parent / "config.json"
    with open(config_path, 'r') as f:
        config = json.load(f)

    num_processes = config['test_config']['num_processes']
    images_per_process = config['test_config']['images_per_process']
    total_images = num_processes * images_per_process

    print(f"\nConfiguration:")
    print(f"  Processes: {num_processes}")
    print(f"  Images per process: {images_per_process}")
    print(f"  Total images: {total_images}")
    print(f"  Servers: {', '.join(config['server_config']['servers'])}")
    print(f"  Image size: {config['test_config']['image_width']}x{config['test_config']['image_height']}")
    print()

    # Create output directories
    for dir_key in ['output_dir', 'test_images_dir', 'encrypted_dir', 'decrypted_dir', 'metrics_dir']:
        Path(config['output_config'][dir_key]).mkdir(parents=True, exist_ok=True)

    # Start timer
    overall_start = time.time()

    print(f"Starting {num_processes} worker processes...")
    print()

    # Use Manager for shared dict
    manager = multiprocessing.Manager()
    return_dict = manager.dict()

    # Spawn worker processes
    processes = []
    for process_id in range(1, num_processes + 1):
        p = multiprocessing.Process(
            target=run_worker,
            args=(process_id, config, return_dict)
        )
        p.start()
        processes.append(p)
        print(f"✓ Spawned Process {process_id}")

    print(f"\nAll {num_processes} processes started. Waiting for completion...")
    print()

    # Wait for all processes to complete
    for p in processes:
        p.join()

    overall_elapsed = time.time() - overall_start

    print()
    print("=" * 80)
    print(" ALL PROCESSES COMPLETED")
    print("=" * 80)
    print()

    # Aggregate results
    all_results = []
    total_successes = 0
    total_failures = 0
    total_latency = 0
    latencies = []

    server_stats = {}
    for server in config['server_config']['servers']:
        server_stats[server] = {'success': 0, 'failure': 0, 'latency': []}

    for process_id in range(1, num_processes + 1):
        if process_id in return_dict:
            process_result = return_dict[process_id]
            total_successes += process_result.get('success_count', 0)
            total_failures += process_result.get('failure_count', 0)

            for result in process_result.get('results', []):
                all_results.append(result)

                if result['success']:
                    latencies.append(result['latency'])
                    total_latency += result['latency']

                    # Track per-server stats
                    server = result.get('server')
                    if server:
                        server_stats[server]['success'] += 1
                        server_stats[server]['latency'].append(result['latency'])
                else:
                    server = result.get('server')
                    if server:
                        server_stats[server]['failure'] += 1

    # Calculate metrics
    success_rate = (total_successes / total_images * 100) if total_images > 0 else 0
    failure_rate = (total_failures / total_images * 100) if total_images > 0 else 0
    avg_latency = (total_latency / total_successes) if total_successes > 0 else 0
    throughput = total_images / overall_elapsed if overall_elapsed > 0 else 0

    # Calculate latency percentiles
    if latencies:
        latencies.sort()
        p50 = latencies[len(latencies) // 2]
        p95 = latencies[int(len(latencies) * 0.95)]
        p99 = latencies[int(len(latencies) * 0.99)]
        min_latency = min(latencies)
        max_latency = max(latencies)
    else:
        p50 = p95 = p99 = min_latency = max_latency = 0

    # Print summary
    print("OVERALL RESULTS:")
    print(f"  Total images: {total_images}")
    print(f"  Successful: {total_successes} ({success_rate:.1f}%)")
    print(f"  Failed: {total_failures} ({failure_rate:.1f}%)")
    print(f"  Total time: {overall_elapsed:.2f}s")
    print(f"  Throughput: {throughput:.2f} images/sec")
    print()

    print("LATENCY STATISTICS:")
    print(f"  Average: {avg_latency:.3f}s")
    print(f"  Median (P50): {p50:.3f}s")
    print(f"  P95: {p95:.3f}s")
    print(f"  P99: {p99:.3f}s")
    print(f"  Min: {min_latency:.3f}s")
    print(f"  Max: {max_latency:.3f}s")
    print()

    print("PER-SERVER STATISTICS:")
    for server, stats in server_stats.items():
        total = stats['success'] + stats['failure']
        server_success_rate = (stats['success'] / total * 100) if total > 0 else 0
        avg_server_latency = (sum(stats['latency']) / len(stats['latency'])) if stats['latency'] else 0

        print(f"  {server}:")
        print(f"    Total requests: {total}")
        print(f"    Success: {stats['success']} ({server_success_rate:.1f}%)")
        print(f"    Failure: {stats['failure']}")
        print(f"    Avg latency: {avg_server_latency:.3f}s")

    # Save aggregated metrics
    metrics = {
        'test_config': config['test_config'],
        'overall': {
            'total_images': total_images,
            'successful': total_successes,
            'failed': total_failures,
            'success_rate': success_rate,
            'failure_rate': failure_rate,
            'total_time': overall_elapsed,
            'throughput': throughput
        },
        'latency': {
            'average': avg_latency,
            'median': p50,
            'p95': p95,
            'p99': p99,
            'min': min_latency,
            'max': max_latency,
            'all_latencies': latencies
        },
        'per_server': server_stats,
        'all_results': all_results
    }

    metrics_file = Path(config['output_config']['metrics_dir']) / "aggregated_metrics.json"
    with open(metrics_file, 'w') as f:
        json.dump(metrics, f, indent=2)

    print()
    print(f"✓ Metrics saved to: {metrics_file}")
    print()
    print("=" * 80)

    return success_rate == 100.0


if __name__ == '__main__':
    success = main()
    sys.exit(0 if success else 1)
