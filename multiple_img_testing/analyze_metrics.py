#!/usr/bin/env python3
"""
Analyze metrics and generate comprehensive report
"""

import json
import sys
from pathlib import Path
from datetime import datetime


def format_seconds(seconds):
    """Format seconds in a human-readable way"""
    if seconds < 1:
        return f"{seconds*1000:.0f}ms"
    elif seconds < 60:
        return f"{seconds:.2f}s"
    else:
        mins = int(seconds // 60)
        secs = seconds % 60
        return f"{mins}m {secs:.0f}s"


def main():
    print("=" * 80)
    print(" METRICS ANALYSIS REPORT")
    print("=" * 80)
    print()

    # Load config
    config_path = Path(__file__).parent / "config.json"
    with open(config_path, 'r') as f:
        config = json.load(f)

    metrics_dir = Path(config['output_config']['metrics_dir'])

    # Load aggregated metrics
    metrics_file = metrics_dir / "aggregated_metrics.json"
    if not metrics_file.exists():
        print("No metrics file found! Run test_coordinator.py first.")
        return False

    with open(metrics_file, 'r') as f:
        metrics = json.load(f)

    # Load decryption results if available
    decryption_file = metrics_dir / "decryption_results.json"
    decryption_data = None
    if decryption_file.exists():
        with open(decryption_file, 'r') as f:
            decryption_data = json.load(f)

    # Generate report
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

    report_lines = []
    report_lines.append("=" * 80)
    report_lines.append(" STRESS TEST ANALYSIS REPORT")
    report_lines.append(f" Generated: {timestamp}")
    report_lines.append("=" * 80)
    report_lines.append("")

    # Test Configuration
    report_lines.append("TEST CONFIGURATION:")
    report_lines.append("-" * 80)
    test_cfg = metrics['test_config']
    report_lines.append(f"  Number of processes:    {test_cfg['num_processes']}")
    report_lines.append(f"  Images per process:     {test_cfg['images_per_process']}")
    report_lines.append(f"  Total images:           {test_cfg['total_images']}")
    report_lines.append(f"  Image resolution:       {test_cfg['image_width']}x{test_cfg['image_height']}")
    report_lines.append(f"  Image format:           {test_cfg['image_format']}")
    report_lines.append(f"  Image quality:          {test_cfg['image_quality']}")
    report_lines.append("")

    # Overall Results
    report_lines.append("OVERALL RESULTS:")
    report_lines.append("-" * 80)
    overall = metrics['overall']
    report_lines.append(f"  Total images processed: {overall['total_images']}")
    report_lines.append(f"  Successful:             {overall['successful']} ({overall['success_rate']:.2f}%)")
    report_lines.append(f"  Failed:                 {overall['failed']} ({overall['failure_rate']:.2f}%)")
    report_lines.append(f"  Total execution time:   {format_seconds(overall['total_time'])}")
    report_lines.append(f"  Throughput:             {overall['throughput']:.2f} images/sec")
    report_lines.append("")

    # Latency Statistics
    report_lines.append("LATENCY STATISTICS:")
    report_lines.append("-" * 80)
    latency = metrics['latency']
    report_lines.append(f"  Average:                {format_seconds(latency['average'])}")
    report_lines.append(f"  Median (P50):           {format_seconds(latency['median'])}")
    report_lines.append(f"  95th percentile (P95):  {format_seconds(latency['p95'])}")
    report_lines.append(f"  99th percentile (P99):  {format_seconds(latency['p99'])}")
    report_lines.append(f"  Minimum:                {format_seconds(latency['min'])}")
    report_lines.append(f"  Maximum:                {format_seconds(latency['max'])}")
    report_lines.append("")

    # Per-Server Statistics
    report_lines.append("PER-SERVER STATISTICS:")
    report_lines.append("-" * 80)
    for server, stats in metrics['per_server'].items():
        # Handle both old format (with latency list) and new format (with avg_latency)
        if 'total_requests' in stats:
            # New cleaned format
            total = stats['total_requests']
            success_rate = stats['success_rate']
            avg_latency = stats['avg_latency']
        else:
            # Old format (fallback)
            total = stats['success'] + stats['failure']
            success_rate = (stats['success'] / total * 100) if total > 0 else 0
            avg_latency = (sum(stats.get('latency', [])) / len(stats.get('latency', []))) if stats.get('latency') else 0

        report_lines.append(f"  Server: {server}")
        report_lines.append(f"    Total requests:       {total}")
        report_lines.append(f"    Successful:           {stats['success']} ({success_rate:.2f}%)")
        report_lines.append(f"    Failed:               {stats['failure']}")
        report_lines.append(f"    Average latency:      {format_seconds(avg_latency)}")
        report_lines.append("")

    # Decryption Results
    if decryption_data:
        report_lines.append("DECRYPTION VERIFICATION:")
        report_lines.append("-" * 80)
        report_lines.append(f"  Total images decrypted: {decryption_data['total']}")
        report_lines.append(f"  Successful:             {decryption_data['successes']} ({decryption_data['success_rate']:.2f}%)")
        report_lines.append(f"  Failed:                 {decryption_data['failures']}")
        report_lines.append("")

    # Failure Analysis
    if overall['failed'] > 0:
        report_lines.append("FAILURE ANALYSIS:")
        report_lines.append("-" * 80)

        error_counts = {}
        for result in metrics['all_results']:
            if not result['success']:
                error = result.get('error', 'Unknown')
                error_counts[error] = error_counts.get(error, 0) + 1

        for error, count in sorted(error_counts.items(), key=lambda x: x[1], reverse=True):
            report_lines.append(f"  {error}: {count} occurrences")
        report_lines.append("")

    # Summary
    report_lines.append("=" * 80)
    report_lines.append("SUMMARY:")
    report_lines.append("=" * 80)

    if overall['success_rate'] == 100.0:
        report_lines.append("  ✓ ALL TESTS PASSED - 100% SUCCESS RATE")
    elif overall['success_rate'] >= 95.0:
        report_lines.append(f"  ⚠ MOSTLY SUCCESSFUL - {overall['success_rate']:.2f}% success rate")
    else:
        report_lines.append(f"  ✗ ISSUES DETECTED - Only {overall['success_rate']:.2f}% success rate")

    report_lines.append("")
    report_lines.append(f"  Average latency: {format_seconds(latency['average'])}")
    report_lines.append(f"  Throughput: {overall['throughput']:.2f} images/sec")

    if decryption_data and decryption_data['success_rate'] == 100.0:
        report_lines.append("  ✓ All encrypted images decrypted successfully")
    elif decryption_data:
        report_lines.append(f"  ⚠ Decryption success rate: {decryption_data['success_rate']:.2f}%")

    report_lines.append("")
    report_lines.append("=" * 80)

    # Print report
    report_text = "\n".join(report_lines)
    print(report_text)

    # Save report to file
    report_file = metrics_dir / "analysis_report.txt"
    with open(report_file, 'w') as f:
        f.write(report_text)

    print(f"\n✓ Report saved to: {report_file}")

    # Generate simple plots if matplotlib available
    try:
        import matplotlib.pyplot as plt
        import matplotlib
        matplotlib.use('Agg')  # Non-interactive backend

        print("\nGenerating plots...")

        # Latency distribution
        if latency['all_latencies']:
            plt.figure(figsize=(10, 6))
            plt.hist(latency['all_latencies'], bins=50, edgecolor='black')
            plt.xlabel('Latency (seconds)')
            plt.ylabel('Frequency')
            plt.title('Request Latency Distribution')
            plt.axvline(latency['average'], color='red', linestyle='--', label=f'Average: {latency["average"]:.3f}s')
            plt.axvline(latency['median'], color='green', linestyle='--', label=f'Median: {latency["median"]:.3f}s')
            plt.legend()
            plt.grid(True, alpha=0.3)
            plt.savefig(metrics_dir / 'latency_distribution.png', dpi=150, bbox_inches='tight')
            plt.close()
            print("  ✓ latency_distribution.png")

        # Success rate by server
        plt.figure(figsize=(10, 6))
        servers = list(metrics['per_server'].keys())
        success_rates = []
        for server in servers:
            stats = metrics['per_server'][server]
            total = stats['success'] + stats['failure']
            rate = (stats['success'] / total * 100) if total > 0 else 0
            success_rates.append(rate)

        plt.bar(range(len(servers)), success_rates, color=['green' if r == 100 else 'orange' for r in success_rates])
        plt.xlabel('Server')
        plt.ylabel('Success Rate (%)')
        plt.title('Success Rate by Server')
        plt.xticks(range(len(servers)), servers, rotation=15, ha='right')
        plt.ylim(0, 105)
        plt.grid(True, alpha=0.3, axis='y')
        for i, rate in enumerate(success_rates):
            plt.text(i, rate + 1, f'{rate:.1f}%', ha='center')
        plt.savefig(metrics_dir / 'success_rate_by_server.png', dpi=150, bbox_inches='tight')
        plt.close()
        print("  ✓ success_rate_by_server.png")

        print("  Plots saved to output/metrics/")

    except ImportError:
        print("\nNote: matplotlib not available, skipping plot generation")
        print("Install with: pip install matplotlib")

    return overall['success_rate'] >= 95.0


if __name__ == '__main__':
    success = main()
    sys.exit(0 if success else 1)
