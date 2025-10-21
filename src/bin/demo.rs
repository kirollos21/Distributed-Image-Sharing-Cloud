use distributed_image_cloud::client::run_stress_test;
use distributed_image_cloud::metrics::new_metrics_collector;
use distributed_image_cloud::node::CloudNode;
use env_logger::Env;
use log::warn;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger with more detailed output
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    println!("\n{:=<60}", "");
    println!("{:^60}", "DISTRIBUTED IMAGE SHARING CLOUD - PHASE 1 DEMO");
    println!("{:^60}", "Bully Election Algorithm Implementation");
    println!("{:=<60}", "");
    println!();

    // Configuration
    let num_cloud_nodes = 3;
    let num_clients = 50;
    let requests_per_client = 200; // Total: 10,000 requests
    let total_requests = num_clients * requests_per_client;

    println!("Configuration:");
    println!("  Cloud Nodes:           {}", num_cloud_nodes);
    println!("  Concurrent Clients:    {}", num_clients);
    println!("  Requests per Client:   {}", requests_per_client);
    println!("  Total Requests:        {}", total_requests);
    println!();

    // Node addresses
    let node_configs = vec![
        (1, "127.0.0.1:8001".to_string()),
        (2, "127.0.0.1:8002".to_string()),
        (3, "127.0.0.1:8003".to_string()),
    ];

    // Start cloud nodes
    println!("Starting {} cloud nodes...", num_cloud_nodes);

    let mut node_handles = vec![];

    for (node_id, address) in &node_configs {
        let node_id = *node_id;  // Copy the node_id
        let address = address.clone();

        let mut peer_addresses = HashMap::new();
        for (peer_id, peer_addr) in &node_configs {
            if *peer_id != node_id {
                peer_addresses.insert(*peer_id, peer_addr.clone());
            }
        }

        let node = Arc::new(CloudNode::new(node_id, address.clone(), peer_addresses));
        let node_clone = node.clone();

        let handle = tokio::spawn(async move {
            if let Err(e) = node_clone.start().await {
                warn!("Node {} error: {}", node_id, e);
            }
        });

        node_handles.push((node, handle));
        println!("  ✓ Node {} started on {}", node_id, address);

        // Small delay between node starts
        sleep(Duration::from_millis(100)).await;
    }

    println!();
    println!("All cloud nodes started successfully!");
    println!();

    // Wait for nodes to initialize and perform initial election
    println!("Waiting for initial election...");
    sleep(Duration::from_secs(8)).await;

    // Display initial node status
    println!("\nInitial Node Status:");
    println!("{:-<60}", "");
    for (node, _) in &node_handles {
        let stats = node.get_stats().await;
        println!(
            "Node {}: {} | Load: {:.2} | Queue: {} | Coordinator: {}",
            stats.id,
            stats.state,
            stats.load,
            stats.queue_length,
            if stats.is_coordinator { "YES" } else { "NO" }
        );
    }
    println!("{:-<60}", "");
    println!();

    // Prepare cloud addresses for clients
    let cloud_addresses: Vec<String> = node_configs.iter().map(|(_, addr)| addr.clone()).collect();

    // Initialize metrics
    let metrics = new_metrics_collector();

    println!("{:=<60}", "");
    println!("{:^60}", "STARTING STRESS TEST");
    println!("{:=<60}", "");
    println!();

    // Run stress test
    let metrics_clone = metrics.clone();
    let stress_test_handle = tokio::spawn(async move {
        run_stress_test(num_clients, requests_per_client, cloud_addresses, metrics_clone).await;
    });

    // Monitor progress
    let metrics_monitor = metrics.clone();
    let monitor_handle = tokio::spawn(async move {
        let mut last_count = 0;
        loop {
            sleep(Duration::from_secs(5)).await;

            let m = metrics_monitor.lock().await;
            let current_count = m.total_requests;

            if current_count > last_count {
                let progress = (current_count as f64 / total_requests as f64) * 100.0;
                println!(
                    "Progress: {}/{} ({:.1}%) | Success: {} | Failed: {} | Throughput: {:.2} req/s",
                    current_count,
                    total_requests,
                    progress,
                    m.successful_requests,
                    m.failed_requests,
                    m.throughput()
                );
                last_count = current_count;
            }

            if current_count >= total_requests {
                break;
            }
        }
    });

    // Wait for stress test to complete
    stress_test_handle.await?;
    monitor_handle.abort(); // Stop the monitor

    println!();
    println!("Stress test completed! Collecting final metrics...");
    sleep(Duration::from_secs(2)).await;

    // Display final node status
    println!("\nFinal Node Status:");
    println!("{:-<60}", "");
    for (node, _) in &node_handles {
        let stats = node.get_stats().await;
        println!(
            "Node {}: {} | Load: {:.2} | Processed: {} | Coordinator: {}",
            stats.id,
            stats.state,
            stats.load,
            stats.processed_requests,
            if stats.is_coordinator { "YES" } else { "NO" }
        );
    }
    println!("{:-<60}", "");

    // Print metrics summary
    let final_metrics = metrics.lock().await;
    final_metrics.print_summary();

    println!();
    println!("{:=<60}", "");
    println!("{:^60}", "DEMO COMPLETED SUCCESSFULLY");
    println!("{:=<60}", "");
    println!();
    println!("Key Achievements:");
    println!("  ✓ Load-based Bully election algorithm");
    println!("  ✓ Fault tolerance with random failures (up to 20s)");
    println!("  ✓ State recovery and consistency");
    println!("  ✓ LSB steganography encryption service");
    println!("  ✓ Concurrent request handling with Tokio");
    println!("  ✓ {} concurrent clients", num_clients);
    println!("  ✓ {} total requests processed", final_metrics.total_requests);
    println!();

    // Keep running for a bit to observe more elections and failures
    println!("Demo will continue running for 30 more seconds to observe fault tolerance...");
    println!("Watch for nodes entering FAILED and RECOVERING states.");
    println!();

    for i in 0..6 {
        sleep(Duration::from_secs(5)).await;
        println!("Node Status (T+{}s):", (i + 1) * 5);
        for (node, _) in &node_handles {
            let stats = node.get_stats().await;
            println!(
                "  Node {}: {} | Load: {:.2} | {}",
                stats.id,
                stats.state,
                stats.load,
                if stats.is_coordinator {
                    "[COORDINATOR]"
                } else {
                    ""
                }
            );
        }
        println!();
    }

    println!("Demo finished. Press Ctrl+C to exit.");

    // Keep the program running
    tokio::signal::ctrl_c().await?;
    println!("\nShutting down...");

    Ok(())
}
