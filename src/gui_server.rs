use crate::node::{CloudNode, NodeStats};
use eframe::egui;
use egui::{Color32, RichText, Ui};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

const MAX_LOG_ENTRIES: usize = 1000;

pub struct ServerMonitorApp {
    // Node reference
    node: Option<Arc<CloudNode>>,

    // Monitored node ID (for display)
    monitored_node_id: Option<u32>,

    // Node statistics (reserved for future use)
    #[allow(dead_code)]
    current_stats: Option<NodeStats>,

    // Logs
    log_entries: Arc<RwLock<VecDeque<LogEntry>>>,

    // UI state
    selected_tab: Tab,
    auto_scroll_logs: bool,
    log_filter: String,

    // Runtime for async operations
    runtime: Option<Arc<tokio::runtime::Runtime>>,
}

#[derive(PartialEq)]
enum Tab {
    Overview,
    Logs,
    Metrics,
    Network,
}

impl Default for Tab {
    fn default() -> Self {
        Tab::Overview
    }
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Debug,
}

impl ServerMonitorApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let runtime = Arc::new(
            tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"),
        );

        let log_entries = Arc::new(RwLock::new(VecDeque::new()));

        // Add welcome message
        let welcome_entry = LogEntry {
            timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
            level: LogLevel::Info,
            message: "Server Monitor initialized. Connect to a cloud node to see real-time data.".to_string(),
        };

        let log_entries_clone = log_entries.clone();
        runtime.spawn(async move {
            let mut logs = log_entries_clone.write().await;
            logs.push_back(welcome_entry);
        });

        Self {
            node: None,
            monitored_node_id: None,
            current_stats: None,
            log_entries,
            selected_tab: Tab::Overview,
            auto_scroll_logs: true,
            log_filter: String::new(),
            runtime: Some(runtime),
        }
    }

    pub fn with_node(mut self, node: Arc<CloudNode>) -> Self {
        self.node = Some(node);
        self
    }

    pub fn set_monitored_node_id(&mut self, node_id: u32) {
        self.monitored_node_id = Some(node_id);
    }

    fn add_log(&self, level: LogLevel, message: String) {
        let log_entries = self.log_entries.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();

        runtime.spawn(async move {
            let mut logs = log_entries.write().await;
            logs.push_back(LogEntry {
                timestamp: chrono::Utc::now().format("%H:%M:%S").to_string(),
                level,
                message,
            });

            // Keep only last MAX_LOG_ENTRIES
            while logs.len() > MAX_LOG_ENTRIES {
                logs.pop_front();
            }
        });
    }

    #[allow(dead_code)]
    fn update_stats(&mut self) {
        if let Some(node) = &self.node {
            let node = node.clone();
            let runtime = self.runtime.as_ref().unwrap().clone();

            // Spawn a task to get stats
            runtime.spawn(async move {
                node.get_stats().await
            });

            // For now, we'll use polling. In a production app, you'd use channels.
        }
    }

    fn render_overview_tab(&mut self, ui: &mut Ui) {
        ui.heading("📊 Node Overview");
        ui.add_space(10.0);

        if self.node.is_none() {
            ui.label(RichText::new("⚠ No node connected").color(Color32::from_rgb(255, 165, 0)).size(16.0));
            ui.label("This monitor is running in standalone mode.");
            ui.label("To monitor a live node, start the server-gui with a node ID.");
            return;
        }

        // Node info
        if let Some(node) = &self.node {
            ui.group(|ui| {
                ui.label(RichText::new("Node Information").size(16.0).strong());
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Node ID:");
                    ui.label(RichText::new(format!("{}", node.id)).strong());
                });

                ui.horizontal(|ui| {
                    ui.label("Address:");
                    ui.label(RichText::new(&node.address).strong());
                });
            });

            ui.add_space(10.0);
        }

        // Get current stats (simulated for now)
        ui.group(|ui| {
            ui.label(RichText::new("Current Status").size(16.0).strong());
            ui.separator();

            // In a real implementation, you'd poll the node for stats
            let status_color = Color32::from_rgb(0, 200, 0);
            ui.horizontal(|ui| {
                ui.label("State:");
                ui.label(RichText::new("ACTIVE").color(status_color).strong());
            });

            ui.horizontal(|ui| {
                ui.label("Load:");
                ui.add(egui::ProgressBar::new(0.65).text("65%"));
            });

            ui.horizontal(|ui| {
                ui.label("Queue Length:");
                ui.label("3 requests");
            });

            ui.horizontal(|ui| {
                ui.label("Processed Requests:");
                ui.label("1,247");
            });

            ui.horizontal(|ui| {
                ui.label("Is Coordinator:");
                ui.label(RichText::new("YES").color(Color32::from_rgb(0, 150, 255)).strong());
            });
        });

        ui.add_space(10.0);

        // Peer information
        ui.group(|ui| {
            ui.label(RichText::new("Peer Nodes").size(16.0).strong());
            ui.separator();

            if let Some(node) = &self.node {
                for (peer_id, peer_addr) in &node.peer_addresses {
                    ui.horizontal(|ui| {
                        ui.label(format!("Node {}:", peer_id));
                        ui.label(peer_addr);
                        ui.label(RichText::new("●").color(Color32::from_rgb(0, 200, 0)));
                    });
                }
            }
        });
    }

    fn render_logs_tab(&mut self, ui: &mut Ui) {
        ui.heading("📋 System Logs");
        ui.add_space(10.0);

        // Controls
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.auto_scroll_logs, "Auto-scroll");

            ui.separator();

            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.log_filter);

            if ui.button("Clear Logs").clicked() {
                let log_entries = self.log_entries.clone();
                let runtime = self.runtime.as_ref().unwrap().clone();
                runtime.spawn(async move {
                    let mut logs = log_entries.write().await;
                    logs.clear();
                });
            }
        });

        ui.separator();

        // Log entries
        let runtime = self.runtime.as_ref().unwrap().clone();
        let log_entries = self.log_entries.clone();

        // Get logs (this is a simplified version - in production you'd use proper async)
        let logs_display = runtime.block_on(async {
            let logs = log_entries.read().await;
            logs.iter().cloned().collect::<Vec<_>>()
        });

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(self.auto_scroll_logs)
            .show(ui, |ui| {
                for entry in &logs_display {
                    // Apply filter
                    if !self.log_filter.is_empty()
                        && !entry.message.to_lowercase().contains(&self.log_filter.to_lowercase())
                    {
                        continue;
                    }

                    let (color, prefix) = match entry.level {
                        LogLevel::Info => (Color32::from_rgb(100, 200, 255), "ℹ"),
                        LogLevel::Warning => (Color32::from_rgb(255, 165, 0), "⚠"),
                        LogLevel::Error => (Color32::from_rgb(255, 50, 50), "❌"),
                        LogLevel::Debug => (Color32::GRAY, "🔍"),
                    };

                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&entry.timestamp).color(Color32::GRAY).size(11.0));
                        ui.label(RichText::new(prefix).color(color));
                        ui.label(&entry.message);
                    });
                }
            });

        // Simulate adding logs periodically
        if ui.input(|i| i.time % 5.0 < 0.016) {
            // Every ~5 seconds
            let messages = vec![
                "Election initiated by Node 1",
                "Processing encryption request req_1234",
                "Node 3 entering FAILED state",
                "Re-election triggered",
                "Coordinator elected: Node 2",
                "Encryption completed successfully",
                "Node 3 entering RECOVERING state",
                "State synchronized with coordinator",
            ];

            let msg = messages[rand::random::<usize>() % messages.len()];
            self.add_log(LogLevel::Info, msg.to_string());
        }
    }

    fn render_metrics_tab(&mut self, ui: &mut Ui) {
        ui.heading("📈 Performance Metrics");
        ui.add_space(10.0);

        // Request statistics
        ui.group(|ui| {
            ui.label(RichText::new("Request Statistics").size(16.0).strong());
            ui.separator();

            egui::Grid::new("metrics_grid").striped(true).show(ui, |ui| {
                ui.label("Total Requests:");
                ui.label(RichText::new("1,247").strong());
                ui.end_row();

                ui.label("Successful:");
                ui.label(RichText::new("1,228").color(Color32::from_rgb(0, 200, 0)));
                ui.end_row();

                ui.label("Failed:");
                ui.label(RichText::new("19").color(Color32::from_rgb(255, 100, 100)));
                ui.end_row();

                ui.label("Success Rate:");
                ui.label(RichText::new("98.5%").strong());
                ui.end_row();

                ui.label("Avg Latency:");
                ui.label("524 ms");
                ui.end_row();

                ui.label("P95 Latency:");
                ui.label("1,250 ms");
                ui.end_row();
            });
        });

        ui.add_space(10.0);

        // Load over time (placeholder)
        ui.group(|ui| {
            ui.label(RichText::new("Load History").size(16.0).strong());
            ui.separator();

            ui.label(RichText::new("📊 Load graph would be displayed here").color(Color32::GRAY));
            ui.label("In a full implementation, this would show real-time load graphs");
        });

        ui.add_space(10.0);

        // Election history
        ui.group(|ui| {
            ui.label(RichText::new("Recent Elections").size(16.0).strong());
            ui.separator();

            let elections = vec![
                ("14:23:45", "Node 2", "0.50"),
                ("14:22:10", "Node 1", "0.45"),
                ("14:20:33", "Node 3", "0.60"),
            ];

            egui::Grid::new("elections_grid").striped(true).show(ui, |ui| {
                ui.label(RichText::new("Time").strong());
                ui.label(RichText::new("Winner").strong());
                ui.label(RichText::new("Load").strong());
                ui.end_row();

                for (time, winner, load) in elections {
                    ui.label(time);
                    ui.label(winner);
                    ui.label(load);
                    ui.end_row();
                }
            });
        });
    }

    fn render_network_tab(&mut self, ui: &mut Ui) {
        ui.heading("🌐 Network Status");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("Cluster Overview").size(16.0).strong());
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Total Nodes:");
                ui.label(RichText::new("3").strong());
            });

            ui.horizontal(|ui| {
                ui.label("Active Nodes:");
                ui.label(RichText::new("3").color(Color32::from_rgb(0, 200, 0)).strong());
            });

            ui.horizontal(|ui| {
                ui.label("Failed Nodes:");
                ui.label(RichText::new("0").strong());
            });

            ui.horizontal(|ui| {
                ui.label("Current Coordinator:");
                ui.label(RichText::new("Node 2").color(Color32::from_rgb(0, 150, 255)).strong());
            });
        });

        ui.add_space(10.0);

        // Node status table
        ui.group(|ui| {
            ui.label(RichText::new("Node Details").size(16.0).strong());
            ui.separator();

            egui::Grid::new("nodes_grid").striped(true).show(ui, |ui| {
                ui.label(RichText::new("Node").strong());
                ui.label(RichText::new("State").strong());
                ui.label(RichText::new("Load").strong());
                ui.label(RichText::new("Queue").strong());
                ui.label(RichText::new("Role").strong());
                ui.end_row();

                let nodes = vec![
                    ("Node 1", "ACTIVE", 0.65, 2, ""),
                    ("Node 2", "ACTIVE", 0.50, 1, "COORDINATOR"),
                    ("Node 3", "ACTIVE", 0.75, 3, ""),
                ];

                for (name, state, load, queue, role) in nodes {
                    ui.label(name);

                    let state_color = match state {
                        "ACTIVE" => Color32::from_rgb(0, 200, 0),
                        "FAILED" => Color32::from_rgb(255, 50, 50),
                        "RECOVERING" => Color32::from_rgb(255, 165, 0),
                        _ => Color32::WHITE,
                    };
                    ui.label(RichText::new(state).color(state_color));

                    ui.add(egui::ProgressBar::new(load as f32).text(format!("{:.0}%", load * 100.0)));
                    ui.label(format!("{}", queue));
                    if !role.is_empty() {
                        ui.label(RichText::new(role).color(Color32::from_rgb(0, 150, 255)).strong());
                    } else {
                        ui.label("");
                    }
                    ui.end_row();
                }
            });
        });
    }
}

impl eframe::App for ServerMonitorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Repaint continuously for live updates
        ctx.request_repaint();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🖥️  Distributed Image Cloud - Server Monitor");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(node) = &self.node {
                        ui.label(RichText::new(format!("Node {}", node.id))
                            .color(Color32::from_rgb(0, 200, 255))
                            .strong());
                    } else if let Some(node_id) = self.monitored_node_id {
                        ui.label(RichText::new(format!("Monitoring Node {}", node_id))
                            .color(Color32::from_rgb(0, 200, 255))
                            .strong());
                    } else {
                        ui.label(RichText::new("Standalone Mode")
                            .color(Color32::GRAY));
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.selected_tab, Tab::Overview, "📊 Overview");
                ui.selectable_value(&mut self.selected_tab, Tab::Logs, "📋 Logs");
                ui.selectable_value(&mut self.selected_tab, Tab::Metrics, "📈 Metrics");
                ui.selectable_value(&mut self.selected_tab, Tab::Network, "🌐 Network");
            });

            ui.separator();
            ui.add_space(10.0);

            match self.selected_tab {
                Tab::Overview => self.render_overview_tab(ui),
                Tab::Logs => self.render_logs_tab(ui),
                Tab::Metrics => self.render_metrics_tab(ui),
                Tab::Network => self.render_network_tab(ui),
            }
        });
    }
}
