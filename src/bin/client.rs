use distributed_image_cloud::client::Client;
use distributed_image_cloud::messages::Message;
use env_logger::Env;
use log::info;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <client_id> [num_requests]", args[0]);
        eprintln!("Example: {} 1 5", args[0]);
        std::process::exit(1);
    }

    let client_id: usize = args[1].parse().expect("Invalid client ID");
    let num_requests: usize = if args.len() > 2 {
        args[2].parse().expect("Invalid num_requests")
    } else {
        1
    };

    // Cloud node addresses
    let cloud_addresses = vec![
        "127.0.0.1:8001".to_string(),
        "127.0.0.1:8002".to_string(),
        "127.0.0.1:8003".to_string(),
    ];

    info!("Starting Client {}", client_id);
    info!("Will send {} requests", num_requests);

    let client = Client::new(client_id, cloud_addresses);

    for i in 0..num_requests {
        info!("[Client {}] Sending request {}/{}", client_id, i + 1, num_requests);

        let (success, duration) = client.run_test_request(i).await;

        if success {
            info!(
                "[Client {}] Request {}/{} succeeded in {}ms",
                client_id,
                i + 1,
                num_requests,
                duration
            );
        } else {
            info!(
                "[Client {}] Request {}/{} failed after {}ms",
                client_id,
                i + 1,
                num_requests,
                duration
            );
        }

        // Small delay between requests
        if i < num_requests - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    info!("[Client {}] Completed all requests", client_id);

    Ok(())
}
