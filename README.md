# Spotify Playback Socket.IO Server (Rust)

A high-performance Rust implementation of a Socket.IO server for controlling Spotify playback via web interfaces.

## Features

- **Socket.IO Protocol**: Full compatibility with Socket.IO v4 clients
- **System Tray Integration**: Background operation with tray icon and context menu
- **Real-time Control**: Play/Pause, Next, Previous, Shuffle, Repeat commands
- **Live Player Data**: Track information broadcasting with metadata
- **Cross-platform**: Works on Windows, macOS, and Linux

> [!NOTE]
> All advanced features and latest real-time control options require the [Spotify Playback API](https://github.com/eme22/spotify-playback-api) plugin.

## Quick Start

1. **Install the Spicetify extension** (required):
   
   > [!IMPORTANT]
   > To unlock all the **new and advanced features** of this server, you must use the Spotify Playback API plugin.
   
   ```bash
   # Download playbackapi.js from https://github.com/eme22/spotify-playback-api
   spicetify config extensions playbackapi.js
   spicetify apply
   ```
   
   *(Alternatively, for basic features, you can use the legacy `playbackapick.js` extension from [here](https://github.com/CrazyKitty357/spotify-playback-api-ck)).*

2. **Build and run the server**:
   ```bash
   cargo build --release
   ./target/release/spotify-server.exe
   ```

3. **Test the connection**: Open `player-status.html` in your browser

## Usage

### Background Mode (Recommended)
```bash
cargo build --release
./target/release/spotify-server.exe
```
- System tray icon appears
- Right-click tray icon → "Exit" to shutdown

### Development Mode
```bash
cargo run -- -dev
```
- Console window visible for debugging

### Configuration
Environment variables:
- `IP`: Server host (default: `127.0.0.1`)
- `PORT`: Server port (default: `8443`)

## Socket.IO API

### Control Commands
| Event | Description | Example |
|-------|-------------|---------|
| `PlayPause` | Toggle play/pause | `socket.emit("PlayPause")` |
| `Next` | Skip to next track | `socket.emit("Next")` |
| `Prev` | Previous track | `socket.emit("Prev")` |
| `Shuffle` | Toggle shuffle | `socket.emit("Shuffle")` |
| `Repeat` | Toggle repeat | `socket.emit("Repeat")` |
| `getdata` | Get current player data | `socket.emit("getdata")` |
| `request` | Request specific track | `socket.emit("request", "spotify:track:...")` |

### Data Events
- `player_data`: Broadcasts current track information to all clients
- `command`: Spotify client sends track updates

## Related Projects & Plugins

This server is part of a complete Spotify control ecosystem:

### 🔌 [Spotify Playback API (Required for New Features)](https://github.com/eme22/spotify-playback-api)
The primary playback plugin that connects Spotify to this server. This plugin is **required** to support the newest and advanced features of the server. Download `playbackapi.js` and install it via Spicetify.

### 🎵 [Alternative Spicetify Extension](https://github.com/CrazyKitty357/spotify-playback-api-ck)
A legacy/alternative extension for basic connection and playback control.

### 🐍 [Python HTTP Alternative](https://github.com/CrazyKitty357/spotify-playback-http/tree/less-compat-version)
A Python Flask server providing HTTP/REST API instead of Socket.IO for the same functionality.

### Architecture
```
Spotify (Spicetify) ←→ This Rust Server ←→ Web Clients
```

## Development

```bash
# Build
cargo build

# Run with console (dev mode)
cargo run -- -dev

# Run tests
cargo test

# Check code
cargo clippy
```

## License

MIT License - see [LICENSE](LICENSE) file for details.
