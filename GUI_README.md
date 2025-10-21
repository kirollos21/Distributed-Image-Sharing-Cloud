# GUI Applications Guide

The Distributed Image Cloud project includes two graphical user interfaces built with **egui** - a modern, pure-Rust immediate-mode GUI framework.

## Overview

### 1. Client GUI (`client-gui`)
A user-friendly interface for clients to:
- Upload and encrypt images
- Configure authorized users and viewing quotas
- Send encryption requests to the cloud
- View request history
- Save encrypted images

### 2. Server Monitor GUI (`server-gui`)
A real-time monitoring dashboard for cloud nodes:
- View node status and statistics
- Monitor system logs in real-time
- Track performance metrics
- Visualize network topology
- Monitor elections and load balancing

---

## Installation

The GUI dependencies are already included in `Cargo.toml`. Build the GUIs with:

```bash
cargo build --release --bin client-gui
cargo build --release --bin server-gui
```

---

## Running the GUIs

### Client GUI

```bash
# Run the client GUI
cargo run --release --bin client-gui
```

#### Features:

**📤 Upload Tab**
- Click "Choose Image File" to select an image (PNG, JPG, BMP supported)
- Preview your selected image
- Enter comma-separated usernames (e.g., "alice, bob, charlie")
- Set viewing quota (number of times image can be viewed)
- Click "Encrypt Image" to send to cloud
- Save the encrypted result

**📜 History Tab**
- View all encryption requests you've made
- See success/failure status
- Check request duration and parameters

**⚙️ Settings Tab**
- Configure your client ID
- Add/remove cloud node addresses
- Customize connection settings

### Server Monitor GUI

#### Standalone Mode (No Node)
```bash
cargo run --release --bin server-gui
```
Runs a monitoring dashboard in standalone mode with simulated data.

#### Connected to Live Node
```bash
# Monitor Node 1
cargo run --release --bin server-gui 1

# Monitor Node 2
cargo run --release --bin server-gui 2

# Monitor Node 3
cargo run --release --bin server-gui 3
```

#### Features:

**📊 Overview Tab**
- Node ID and address
- Current state (ACTIVE/FAILED/RECOVERING)
- Load metrics with progress bar
- Queue length
- Processed requests count
- Coordinator status
- List of peer nodes

**📋 Logs Tab**
- Real-time system logs
- Color-coded by severity (Info/Warning/Error/Debug)
- Search/filter logs
- Auto-scroll option
- Clear logs button

**📈 Metrics Tab**
- Total requests processed
- Success/failure statistics
- Success rate percentage
- Average and P95 latency
- Recent election history

**🌐 Network Tab**
- Cluster overview
- All nodes status table
- Load distribution
- Current coordinator
- Network health indicators

---

## Usage Examples

### Example 1: Upload and Encrypt an Image

1. **Start Cloud Nodes**
   ```bash
   # Terminal 1
   cargo run --release --bin cloud-node 1

   # Terminal 2
   cargo run --release --bin cloud-node 2

   # Terminal 3
   cargo run --release --bin cloud-node 3
   ```

2. **Launch Client GUI**
   ```bash
   cargo run --release --bin client-gui
   ```

3. **Upload Image**
   - Click "Choose Image File"
   - Select an image from your computer
   - Enter authorized users: `alice, bob`
   - Set quota: `3`
   - Click "Encrypt Image"

4. **Save Result**
   - Wait for success message
   - Click "Save Encrypted Image"
   - Choose destination folder

### Example 2: Monitor a Live Node

1. **Start Node with GUI**
   ```bash
   # Start node 1
   cargo run --release --bin cloud-node 1
   ```

2. **Open Monitor**
   ```bash
   # In another terminal, monitor node 1
   cargo run --release --bin server-gui 1
   ```

3. **Observe**
   - Watch logs in real-time
   - Monitor incoming requests
   - See elections happen
   - Track load changes
   - View state transitions (ACTIVE → FAILED → RECOVERING)

### Example 3: Full Demo with GUI Monitoring

```bash
# Terminal 1: Node 1
cargo run --release --bin cloud-node 1

# Terminal 2: Node 2
cargo run --release --bin cloud-node 2

# Terminal 3: Node 3
cargo run --release --bin cloud-node 3

# Terminal 4: Monitor Node 2
cargo run --release --bin server-gui 2

# Terminal 5: Client GUI
cargo run --release --bin client-gui

# Terminal 6: Run stress test in background
cargo run --release --bin demo
```

Now you can:
- Send requests from the Client GUI
- Watch them being processed in the Server Monitor
- See the stress test running alongside
- Observe load balancing in action

---

## GUI Framework: egui

### Why egui?

✅ **Pure Rust** - No JavaScript, no web tech, pure native performance
✅ **Immediate Mode** - Easy to write, update, and debug
✅ **Cross-Platform** - Windows, macOS, Linux support
✅ **Lightweight** - Small binary size, fast startup
✅ **Real-time** - Perfect for live monitoring
✅ **Async-Friendly** - Works well with Tokio

### Key Dependencies

- `eframe` - Application framework
- `egui` - GUI library
- `egui_extras` - Image support
- `image` - Image loading/processing
- `rfd` - Native file dialogs
- `poll-promise` - Async operations in GUI

---

## Screenshots

### Client GUI

```
┌─────────────────────────────────────────────────────────────┐
│ 🖼️  Distributed Image Cloud - Client            [❓ Help] │
├─────────────────────────────────────────────────────────────┤
│  📤 Upload  │  📜 History  │  ⚙️ Settings                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ 1. Select Image                                       │ │
│  │                                                       │ │
│  │  [📂 Choose Image File]  Selected: photo.jpg         │ │
│  │                                                       │ │
│  │  Preview:                                            │ │
│  │  ┌─────────────────┐                                 │ │
│  │  │                 │                                 │ │
│  │  │   [Image]       │                                 │ │
│  │  │                 │                                 │ │
│  │  └─────────────────┘                                 │ │
│  └───────────────────────────────────────────────────────┘ │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ 2. Configure Encryption                               │ │
│  │                                                       │ │
│  │  Authorized Users: [alice, bob, charlie          ]   │ │
│  │  (Comma-separated usernames)                         │ │
│  │                                                       │ │
│  │  Viewing Quota: [5] views                            │ │
│  └───────────────────────────────────────────────────────┘ │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ 3. Send to Cloud                                      │ │
│  │                                                       │ │
│  │     [🚀 Encrypt Image]                                │ │
│  └───────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Server Monitor GUI

```
┌─────────────────────────────────────────────────────────────┐
│ 🖥️  Distributed Image Cloud - Server Monitor    [Node 2]  │
├─────────────────────────────────────────────────────────────┤
│  📊 Overview  │  📋 Logs  │  📈 Metrics  │  🌐 Network     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ Current Status                                        │ │
│  ├───────────────────────────────────────────────────────┤ │
│  │  State: ACTIVE ●                                      │ │
│  │  Load: [████████████░░░░░░░░] 65%                    │ │
│  │  Queue Length: 3 requests                            │ │
│  │  Processed Requests: 1,247                           │ │
│  │  Is Coordinator: YES                                 │ │
│  └───────────────────────────────────────────────────────┘ │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ Peer Nodes                                            │ │
│  ├───────────────────────────────────────────────────────┤ │
│  │  Node 1: 127.0.0.1:8001  ●                           │ │
│  │  Node 3: 127.0.0.1:8003  ●                           │ │
│  └───────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## Keyboard Shortcuts

### Client GUI
- `Ctrl+O` - Open image file (when in Upload tab)
- `Ctrl+S` - Save encrypted image (after successful encryption)
- `Ctrl+Q` - Quit application

### Server Monitor GUI
- `Ctrl+L` - Focus log filter
- `Ctrl+K` - Clear logs
- `Ctrl+R` - Refresh data
- `Ctrl+Q` - Quit application

---

## Troubleshooting

### GUI Won't Start

**Error: "Failed to create window"**
```bash
# Make sure you have graphics libraries installed

# Ubuntu/Debian
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev

# Fedora
sudo dnf install libxcb-devel libxkbcommon-devel

# macOS
# Should work out of the box
```

### Image Preview Not Showing

- Ensure image format is supported (PNG, JPG, BMP)
- Check file size (very large images may take time to load)
- Try a different image

### Connection Errors in Client GUI

- Verify cloud nodes are running
- Check addresses in Settings tab
- Ensure ports are not blocked by firewall

### Server Monitor Shows No Data

- Confirm node is running (check terminal)
- Try restarting the monitor
- Check that node ID matches

---

## Advanced Configuration

### Custom Themes

Edit the GUI source files to customize colors:

```rust
// In gui_client.rs or gui_server.rs
ctx.set_visuals(egui::Visuals::dark()); // or light()
```

### Window Size

Modify in the binary files:

```rust
let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 900.0]),  // Adjust size
    ..Default::default()
};
```

---

## Performance Tips

1. **Reduce Repaint Frequency**
   - Modify `ctx.request_repaint()` interval
   - Only repaint when data changes

2. **Limit Log Entries**
   - Default: 1000 entries
   - Adjust `MAX_LOG_ENTRIES` constant

3. **Image Preview Size**
   - Large images are scaled down automatically
   - Adjust `max_size` variable for smaller/larger previews

---

## Future Enhancements

Potential improvements for the GUI:

- [ ] Real-time load graphs with historical data
- [ ] Network topology visualization
- [ ] Image decryption viewer
- [ ] Dark/light theme toggle
- [ ] Export metrics to CSV
- [ ] Custom alert notifications
- [ ] Multi-node monitoring (monitor all 3 nodes in one window)
- [ ] P2P discovery service GUI
- [ ] Quota management interface

---

## Contributing to GUI

The GUI code is organized as follows:

```
src/
├── gui_client.rs       # Client GUI implementation
├── gui_server.rs       # Server monitor implementation
└── bin/
    ├── client_gui.rs   # Client GUI launcher
    └── server_gui.rs   # Server monitor launcher
```

To add new features:

1. Modify the respective `gui_*.rs` file
2. Add new tabs or panels as needed
3. Test with `cargo run --bin <gui-name>`
4. Update this README

---

## License

Same license as the main project (Academic use for CSCE 4411).

---

**Enjoy the graphical interfaces!** 🎨

For issues or questions, refer to the main README.md or contact the project maintainer.
