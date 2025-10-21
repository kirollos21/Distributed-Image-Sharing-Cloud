use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Metrics collected during stress testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestMetrics {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub request_durations_ms: Vec<u64>,
    pub load_balancing_decisions: Vec<LoadBalancingDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingDecision {
    pub timestamp: DateTime<Utc>,
    pub selected_node: u32,
    pub node_loads: Vec<(u32, f64)>,
}

impl StressTestMetrics {
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            start_time: Utc::now(),
            end_time: None,
            request_durations_ms: Vec::new(),
            load_balancing_decisions: Vec::new(),
        }
    }

    pub fn record_request(&mut self, success: bool, duration_ms: u64) {
        self.total_requests += 1;
        if success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }
        self.request_durations_ms.push(duration_ms);
    }

    pub fn record_load_balancing(&mut self, selected_node: u32, node_loads: Vec<(u32, f64)>) {
        self.load_balancing_decisions.push(LoadBalancingDecision {
            timestamp: Utc::now(),
            selected_node,
            node_loads,
        });
    }

    pub fn finish(&mut self) {
        self.end_time = Some(Utc::now());
    }

    pub fn duration_seconds(&self) -> f64 {
        if let Some(end_time) = self.end_time {
            (end_time - self.start_time).num_milliseconds() as f64 / 1000.0
        } else {
            (Utc::now() - self.start_time).num_milliseconds() as f64 / 1000.0
        }
    }

    pub fn throughput(&self) -> f64 {
        let duration = self.duration_seconds();
        if duration > 0.0 {
            self.total_requests as f64 / duration
        } else {
            0.0
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests > 0 {
            (self.successful_requests as f64 / self.total_requests as f64) * 100.0
        } else {
            0.0
        }
    }

    pub fn avg_latency_ms(&self) -> f64 {
        if !self.request_durations_ms.is_empty() {
            let sum: u64 = self.request_durations_ms.iter().sum();
            sum as f64 / self.request_durations_ms.len() as f64
        } else {
            0.0
        }
    }

    pub fn p95_latency_ms(&self) -> u64 {
        if self.request_durations_ms.is_empty() {
            return 0;
        }
        let mut sorted = self.request_durations_ms.clone();
        sorted.sort();
        let index = (sorted.len() as f64 * 0.95) as usize;
        sorted[index.min(sorted.len() - 1)]
    }

    pub fn print_summary(&self) {
        println!("\n{:=<60}", "");
        println!("{:^60}", "STRESS TEST RESULTS");
        println!("{:=<60}", "");
        println!();
        println!("Total Duration:        {:.2} seconds", self.duration_seconds());
        println!("Total Requests:        {}", self.total_requests);
        println!("Successful:            {}", self.successful_requests);
        println!("Failed:                {}", self.failed_requests);
        println!("Success Rate:          {:.2}%", self.success_rate());
        println!("Throughput:            {:.2} requests/second", self.throughput());
        println!();
        println!("Latency Statistics:");
        println!("  Average:             {:.2} ms", self.avg_latency_ms());
        println!("  P95:                 {} ms", self.p95_latency_ms());
        println!();
        println!("Load Balancing Decisions: {}", self.load_balancing_decisions.len());

        // Show sample of load balancing decisions
        if !self.load_balancing_decisions.is_empty() {
            println!();
            println!("Sample Load Balancing Decisions:");
            let sample_size = 5.min(self.load_balancing_decisions.len());
            for i in 0..sample_size {
                let idx = (i * self.load_balancing_decisions.len()) / sample_size;
                let decision = &self.load_balancing_decisions[idx];
                println!(
                    "  [{}] Selected Node {}: loads = {:?}",
                    decision.timestamp.format("%H:%M:%S"),
                    decision.selected_node,
                    decision.node_loads
                );
            }
        }

        println!();
        println!("{:=<60}", "");
    }
}

/// Thread-safe metrics collector
pub type MetricsCollector = Arc<Mutex<StressTestMetrics>>;

pub fn new_metrics_collector() -> MetricsCollector {
    Arc::new(Mutex::new(StressTestMetrics::new()))
}
