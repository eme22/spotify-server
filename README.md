# Spotify Playback Socket.IO Server (Rust)

A high-performance Rust implementation of a Socket.IO server for controlling Spotify playback. This server provides real-time communication between Spotify clients (via Spicetify plugins) and web interfaces, enabling remote control of Spotify playback.

## Features

- **Socket.IO Protocol**: Full Socket.IO v4 compatibility using `socketioxide`
- **System Tray Application**: Runs in background with system tray icon and context menu
- **Client Type Management**: Automatic detection and routing between Spotify clients and web interfaces
- **Real-time Player Data**: Live track information broadcasting with metadata support
- **Spotify Control Commands**: Complete playback control (Play/Pause, Next, Previous, Shuffle, Repeat)
- **Custom Track Requests**: Support for requesting specific tracks via Spotify URIs
- **CORS Support**: Cross-origin resource sharing for web clients
- **Environment Configuration**: Configurable host and port via environment variables
- **Structured Logging**: Comprehensive logging with the `tracing` crate

## Requirements

- Rust 1.70+ (latest stable recommended)
- Cargo (comes with Rust)

## Installation

1. Clone or download this project
2. Navigate to the project directory
3. Build the project:
   ```bash
   cargo build --release
   ```

## Usage

### Running as Background Application (Recommended)

The server runs as a background application with a system tray icon:

```bash
# Build and run in background mode
cargo build --release
./target/release/spotify-server.exe
```

Or use the convenience scripts:
```bash
# Windows Batch
./start-background.bat

# PowerShell
./start-background.ps1
```

When running in background mode:
- The server starts with a system tray icon
- Right-click the tray icon to see the context menu
- Select "Exit" from the context menu to gracefully shutdown the server
- The application window is hidden and runs silently

### Testing the System Tray

To verify the system tray is working correctly:

1. **Check for Icon**: Look for the Spotify Server icon in your Windows system tray (notification area, usually bottom-right corner)
2. **Test Context Menu**: Right-click the tray icon to open the context menu
3. **Verify Exit Function**: Click "Exit" in the context menu to gracefully shutdown the server
4. **Monitor Logs**: Check console output for tray events like:
   ```
   [Tray] System tray icon created successfully!
   [Tray] Starting Windows message loop...
   [Tray] Exit menu item clicked, shutting down...
   ```

### Running in Development Mode

```bash
cargo run
```

Or run the release build directly:
```bash
cargo run --release
```

### Environment Variables

- `IP`: Server host address (default: `127.0.0.1`)
- `PORT`: Server port (default: `8443`)

Example:
```bash
$env:IP="0.0.0.0"; $env:PORT="3000"; cargo run
```

## Architecture

The server implements a sophisticated client routing system:

### Client Types
- **Spotify Clients**: Identified when they send `command` events (typically Spicetify plugins)
- **Web Interface Clients**: Identified when they send `input` events (web browsers, control panels)
- **Unknown Clients**: Unidentified clients that haven't sent identifying events

### Data Structures

#### SpotifyMetadata
```rust
pub struct SpotifyMetadata {
    pub artist_name: Option<String>,
    pub album_title: Option<String>,
    pub title: Option<String>,
    pub duration: Option<String>,
    pub image_url: Option<String>,
}
```

#### PlayerData
```rust
pub struct PlayerData {
    pub track_title: String,
    pub artist_name: String,
    pub album_title: String,
    pub duration: String,
    pub image_url: String,
    pub uri: String,
    pub raw_data: String,
}
```

### Communication Flow
1. **Spotify clients** send track data via `command` events
2. **Web clients** send control commands via `input` events or direct command events
3. **Server** routes commands only to registered Spotify clients
4. **Server** broadcasts player data updates to all connected clients

## Socket.IO API

The server accepts Socket.IO connections and supports the following events:

### Client to Server Events

| Event | Description | Data Type | Client Type | Example |
|-------|-------------|-----------|-------------|---------|
| `command` | Send track data/updates | `string` or `SpotifyTrackData` | Spotify | Track metadata or simple title |
| `input` | Send control commands | `string` | Web Interface | `"PlayPause"`, `"Next"`, etc. |
| `PlayPause` | Toggle play/pause | none | Any | `socket.emit("PlayPause")` |
| `Next` | Skip to next track | none | Any | `socket.emit("Next")` |
| `Prev` | Go to previous track | none | Any | `socket.emit("Prev")` |
| `Shuffle` | Toggle shuffle mode | none | Any | `socket.emit("Shuffle")` |
| `Repeat` | Toggle repeat mode | none | Any | `socket.emit("Repeat")` |
| `getdata` | Request current player data | none | Any | `socket.emit("getdata")` |
| `request` | Request specific track | `string` | Any | `socket.emit("request", "spotify:track:...")` |

### Server to Client Events

| Event | Description | Data Type | Target | Example |
|-------|-------------|-----------|--------|---------|
| `input` | Command for Spotify client | `string` | Spotify only | `"PlayPause"`, `"request ..."` |
| `player_data` | Current player information | `PlayerData` | All clients | Player state with track info |

## Compatibility

This server is **fully compatible** with:
- Socket.IO v4 clients and libraries
- Spicetify plugins that implement the Socket.IO protocol
- Web browsers with Socket.IO client libraries
- Any client that follows the event patterns documented above

## Development

### Building
```bash
cargo build
```

### Running in development mode
```bash
cargo run
```

### Running tests
```bash
cargo test
```

### Checking code
```bash
cargo check
cargo clippy
```

## Testing

The project includes `player-status.html`, a comprehensive test client that demonstrates:
- Socket.IO connectivity and status monitoring
- All supported playback control commands
- Real-time player data display with album artwork
- Custom request functionality
- Auto-refresh capabilities

Open `player-status.html` in a browser while the server is running to test functionality.

## Dependencies

- **tokio**: Async runtime for Rust (v1.0 with full features)
- **axum**: Modern web framework built on tokio and hyper (v0.7)
- **socketioxide**: Socket.IO implementation for Rust (v0.13)
- **serde**: Serialization framework with derive support (v1.0)
- **serde_json**: JSON support for serde (v1.0)
- **tower**: Service trait and utilities (v0.4)
- **tower-http**: HTTP middleware including CORS (v0.5)
- **http**: HTTP types and utilities (v1.0)
- **tracing**: Structured logging (v0.1)
- **tracing-subscriber**: Logging subscriber implementation (v0.3)

## Technical Implementation

### Core Technologies
- **Async/await**: Non-blocking I/O for handling multiple concurrent connections
- **RwLock**: Thread-safe shared state management for player data and client registry
- **Event-driven Architecture**: Real-time bidirectional communication via Socket.IO events
- **Type Safety**: Rust's type system ensures memory safety and prevents runtime errors

### Server Features
- **Client Detection**: Automatic identification of Spotify vs. Web Interface clients
- **Command Routing**: Smart routing of commands only to appropriate client types
- **State Management**: Persistent player state with track metadata and connection info
- **Error Handling**: Robust error handling with proper Result types
- **Logging**: Structured logging with connection events and command tracking

## License

This project maintains compatibility with the original implementation licensing.
