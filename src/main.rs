// Use Windows subsystem to prevent console window by default
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use std::sync::Arc;
use std::collections::HashMap;
use http::Method;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use socketioxide::{
    extract::{Data, SocketRef},
    SocketIo,
};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder,
};
use std::thread;
use std::sync::mpsc;
use image::GenericImageView;
use anyhow;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;

#[cfg(target_os = "windows")]
use windows::Win32::{
    UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG, MessageBoxW, MB_OK, MB_ICONERROR, ShowWindow, SW_HIDE, SW_SHOW},
    System::Console::{AllocConsole, GetConsoleWindow, SetConsoleTitleW},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyMetadata {
    #[serde(rename = "artist_name")]
    pub artist_name: Option<String>,
    #[serde(rename = "album_title")]
    pub album_title: Option<String>,
    #[serde(rename = "title")]
    pub title: Option<String>,
    #[serde(rename = "duration")]
    pub duration: Option<String>,
    #[serde(rename = "image_url")]
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyTrackData {
    pub metadata: Option<SpotifyMetadata>,
    pub uri: Option<String>,
    pub uid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerData {
    pub track_title: String,
    pub artist_name: String,
    pub album_title: String,
    pub duration: String,
    pub image_url: String,
    pub uri: String,
    pub raw_data: String,
}

impl Default for PlayerData {
    fn default() -> Self {
        PlayerData {
            track_title: "No track info yet".to_string(),
            artist_name: "Unknown Artist".to_string(),
            album_title: "Unknown Album".to_string(),
            duration: "0".to_string(),
            image_url: "".to_string(),
            uri: "".to_string(),
            raw_data: "".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ClientType {
    Spotify,
    WebInterface,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub client_type: ClientType,
    pub socket_id: String,
}

type PlayerState = Arc<RwLock<PlayerData>>;
type ClientRegistry = Arc<RwLock<HashMap<String, ClientType>>>;

// Function to show error dialog on Windows
#[cfg(target_os = "windows")]
fn show_error_dialog(message: &str) {
    use windows::core::PCWSTR;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    
    let wide_title: Vec<u16> = OsStr::new("Spotify Server Error")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let wide_message: Vec<u16> = OsStr::new(message)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
      unsafe {
        MessageBoxW(
            None,
            PCWSTR(wide_message.as_ptr()),
            PCWSTR(wide_title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn show_error_dialog(message: &str) {
    eprintln!("Error: {}", message);
}

// Function to hide console window on Windows
#[cfg(target_os = "windows")]
fn hide_console() {
    unsafe {
        let console_window = GetConsoleWindow();
        if !console_window.is_invalid() {
            let _ = ShowWindow(console_window, SW_HIDE);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn hide_console() {
    // No-op on non-Windows platforms
}

// Function to show console window on Windows
#[cfg(target_os = "windows")]
fn show_console() {
    unsafe {
        let console_window = GetConsoleWindow();
        if console_window.is_invalid() {
            // Allocate a new console if one doesn't exist (Windows subsystem mode)
            if AllocConsole().is_ok() {
                // Set console title
                use windows::core::PCWSTR;
                use std::ffi::OsStr;
                use std::os::windows::ffi::OsStrExt;
                
                let title: Vec<u16> = OsStr::new("Spotify Server - Debug Console")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                let _ = SetConsoleTitleW(PCWSTR(title.as_ptr()));
                
                // Redirect stdout, stderr, and stdin to the new console
                redirect_console_streams();
            }
        } else {
            // Console already exists, just show it
            let _ = ShowWindow(console_window, SW_SHOW);
        }
    }
}

#[cfg(target_os = "windows")]
fn redirect_console_streams() {
    use std::ffi::CString;
    
    unsafe {
        // Redirect stdout to console
        if let Ok(conout) = CString::new("CONOUT$") {
            if let Ok(mode) = CString::new("w") {
                libc::freopen(conout.as_ptr(), mode.as_ptr(), libc_stdhandle::stdout());
            }
        }
        
        // Redirect stderr to console
        if let Ok(conout) = CString::new("CONOUT$") {
            if let Ok(mode) = CString::new("w") {
                libc::freopen(conout.as_ptr(), mode.as_ptr(), libc_stdhandle::stderr());
            }
        }
        
        // Redirect stdin to console
        if let Ok(conin) = CString::new("CONIN$") {
            if let Ok(mode) = CString::new("r") {
                libc::freopen(conin.as_ptr(), mode.as_ptr(), libc_stdhandle::stdin());
            }
        }
    }
}

// Helper module to get standard handles as FILE pointers
#[cfg(target_os = "windows")]
mod libc_stdhandle {
    use libc::FILE;
    
    extern "C" {
        #[link_name = "__acrt_iob_func"]
        fn acrt_iob_func(fd: u32) -> *mut FILE;
    }
    
    pub unsafe fn stdin() -> *mut FILE {
        acrt_iob_func(0)
    }
    
    pub unsafe fn stdout() -> *mut FILE {
        acrt_iob_func(1)
    }
    
    pub unsafe fn stderr() -> *mut FILE {
        acrt_iob_func(2)
    }
}

#[cfg(not(target_os = "windows"))]
fn show_console() {
    // No-op on non-Windows platforms
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let dev_mode = args.iter().any(|arg| arg == "-dev" || arg == "--dev");
    
    // Handle console visibility based on dev mode
    if dev_mode {
        show_console();
        println!("[Dev Mode] Console enabled");
    } else {
        hide_console();
    }
    
    // Set up panic handler for error dialogs in non-dev mode
    if !dev_mode {
        std::panic::set_hook(Box::new(|panic_info| {
            let message = if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                format!("Application crashed: {}", s)
            } else if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                format!("Application crashed: {}", s)
            } else {
                "Application crashed with an unknown error".to_string()
            };
            
            let location = if let Some(location) = panic_info.location() {
                format!("\n\nLocation: {}:{}:{}", location.file(), location.line(), location.column())
            } else {
                String::new()
            };
            
            let full_message = format!("{}{}\n\nThe application will now exit.", message, location);
            show_error_dialog(&full_message);
        }));
    }
    
    // Wrap the main logic in a Result to catch and handle errors gracefully
    if let Err(e) = run_application(dev_mode).await {
        let error_message = format!("Failed to start Spotify Server: {}", e);
        if dev_mode {
            eprintln!("{}", error_message);
        } else {
            show_error_dialog(&error_message);
        }
        std::process::exit(1);
    }
    
    Ok(())
}

async fn run_application(dev_mode: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing - only show logs in dev mode or if explicitly enabled
    if dev_mode {
        tracing_subscriber::fmt::init();
        info!("🔧 Running in development mode");
    } else {
        // In production mode, only log errors and above
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::ERROR)
            .init();
    }    // Channel to signal shutdown
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    // --- System Tray Setup ---
    let _tray_thread = thread::spawn(move || {
        // --- Tray icon loading logic (fixed for embedded and alert) ---
        let possible_paths = vec![
            std::env::current_dir().unwrap().join("assets").join("icon.png"),
            std::env::current_exe().unwrap().parent().unwrap().join("assets").join("icon.png"),
            std::env::current_dir().unwrap().join("assets").join("icon.ico"),
            std::env::current_exe().unwrap().parent().unwrap().join("assets").join("icon.ico"),
        ];
        let mut icon_bytes: Option<(Vec<u8>, String)> = None;
        for path in &possible_paths {
            if path.exists() {
                if let Ok(bytes) = std::fs::read(path) {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        icon_bytes = Some((bytes, ext.to_string()));
                        break;
                    }
                }
            }
        }
        // If not found on disk, try embedded assets
        if icon_bytes.is_none() {
            if let Some(embed) = Assets::get("icon.png") {
                icon_bytes = Some((embed.data.to_vec(), "png".to_string()));
            } else if let Some(embed) = Assets::get("icon.ico") {
                icon_bytes = Some((embed.data.to_vec(), "ico".to_string()));
            }
        }
        if icon_bytes.is_none() {
            if dev_mode {
                eprintln!("[Tray] No icon file found in expected locations or embedded assets");
            } else {
                show_error_dialog("No tray icon file found in expected locations or embedded in the executable.");
            }
            return;
        }
        let (icon_data, icon_type) = icon_bytes.unwrap();
        // Try to decode the icon
        let icon = match icon_type.as_str() {
            "png" => match image::load_from_memory(&icon_data) {
                Ok(img) => {
                    let rgba = img.to_rgba8();
                    let (width, height) = img.dimensions();
                    match tray_icon::Icon::from_rgba(rgba.into_raw(), width, height) {
                        Ok(icon) => Some(icon),
                        Err(e) => {
                            if dev_mode {
                                eprintln!("[Tray] Failed to create tray icon: {}", e);
                            } else {
                                show_error_dialog(&format!("Failed to create tray icon: {}", e));
                            }
                            None
                        }
                    }
                }
                Err(e) => {
                    if dev_mode {
                        eprintln!("[Tray] Failed to load icon image: {}", e);
                    } else {
                        show_error_dialog(&format!("Failed to load icon image: {}", e));
                    }
                    None
                }
            },
            "ico" => match image::load_from_memory_with_format(&icon_data, image::ImageFormat::Ico) {
                Ok(img) => {
                    let rgba = img.to_rgba8();
                    let (width, height) = img.dimensions();
                    match tray_icon::Icon::from_rgba(rgba.into_raw(), width, height) {
                        Ok(icon) => Some(icon),
                        Err(e) => {
                            if dev_mode {
                                eprintln!("[Tray] Failed to create tray icon: {}", e);
                            } else {
                                show_error_dialog(&format!("Failed to create tray icon: {}", e));
                            }
                            None
                        }
                    }
                }
                Err(e) => {
                    if dev_mode {
                        eprintln!("[Tray] Failed to load icon image: {}", e);
                    } else {
                        show_error_dialog(&format!("Failed to load icon image: {}", e));
                    }
                    None
                }
            },
            _ => {
                if dev_mode {
                    eprintln!("[Tray] Unsupported icon type: {}", icon_type);
                } else {
                    show_error_dialog(&format!("Unsupported icon type: {}", icon_type));
                }
                None
            }
        };
        if icon.is_none() {
            return;
        }        let icon = icon.unwrap();
        
        if dev_mode {
            println!("[Tray] Creating tray icon...");
        }
        
        // Create menu with explicit ID
        let menu = Menu::new();
        let exit_item = MenuItem::with_id("exit", "Exit", true, None);
          if let Err(e) = menu.append(&exit_item) {
            if dev_mode {
                eprintln!("[Tray] Failed to append exit item: {}", e);
            } else {
                show_error_dialog(&format!("Failed to create tray menu: {}", e));
            }
            return;
        }
        
        // Create tray icon
        let _tray_icon = match TrayIconBuilder::new()
            .with_icon(icon)
            .with_tooltip("Spotify Server")
            .with_menu(Box::new(menu))
            .build()        {
            Ok(tray_icon) => {
                if dev_mode {
                    println!("[Tray] System tray icon created successfully!");
                }
                tray_icon
            }
            Err(e) => {
                if dev_mode {
                    eprintln!("[Tray] Failed to create tray icon: {}", e);
                } else {
                    show_error_dialog(&format!("Failed to create tray icon: {}", e));
                }
                return;
            }
        };
        
        // Start menu event handler in async context
        std::thread::spawn(move || {
            // Use tokio's blocking scheduler for this thread
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                loop {
                    if let Ok(event) = MenuEvent::receiver().try_recv() {
                        let event_id = event.id.0.as_str();                        match event_id {
                            "exit" => {
                                if dev_mode {
                                    println!("[Tray] Exit menu item clicked, shutting down...");
                                }
                                // Send shutdown signal
                                let _ = shutdown_tx.send(());
                                break;
                            }
                            _ => {
                                if dev_mode {
                                    println!("[Tray] Unknown menu event: {}", event_id);
                                }
                            }
                        }
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
            });
        });
          // Run Windows message loop on this thread
        #[cfg(target_os = "windows")]
        {
            if dev_mode {
                println!("[Tray] Starting Windows message loop...");
            }
            unsafe {
                let mut msg = MSG::default();
                loop {
                    let result = GetMessageW(&mut msg, None, 0, 0);
                    if result.0 <= 0 {
                        break;
                    }
                    DispatchMessageW(&msg);
                }
            }
        }
          #[cfg(not(target_os = "windows"))]
        {
            // For non-Windows platforms, just keep the thread alive
            loop {
                std::thread::sleep(std::time::Duration::from_millis(1000));
            }
        }
    });

    // Shared state
    let player_state = PlayerState::default();
    let client_registry = ClientRegistry::default();

    // Create Socket.IO layer
    let (layer, io) = SocketIo::new_layer();

    // Clone player_state for use in Socket.IO handlers
    let player_state_clone = player_state.clone();
    let client_registry_clone = client_registry.clone();

    // Register Socket.IO event handlers
    let io_clone = io.clone();
    io.ns("/", move |socket: SocketRef| {
        let player_state = player_state_clone.clone();
        let client_registry = client_registry_clone.clone();
        let io_for_commands = io_clone.clone();
        
        info!("🔗 Client {} connected", socket.id);

        // Helper function to send commands only to Spotify clients
        let send_to_spotify_clients = {
            let io_clone = io_for_commands.clone();
            let client_registry = client_registry.clone();
            move |command: &str| {
                let io_for_commands = io_clone.clone();
                let client_registry = client_registry.clone();
                let command = command.to_string();
                tokio::spawn(async move {
                    let registry = client_registry.read().await;
                    let has_spotify_clients = registry.values().any(|client_type| matches!(client_type, ClientType::Spotify));
                    
                    if has_spotify_clients {
                        // Send command to all clients (only Spotify clients will process it)
                        if let Err(e) = io_for_commands.emit("input", &command) {
                            if command != "getdata" {
                                info!("❌ Failed to send command '{}': {}", command, e);
                            }
                        } else {
                            if command != "getdata" {
                                info!("✅ Sent command '{}' to Spotify clients", command);
                            }
                        }
                    } else {
                        if command != "getdata" {
                            info!("⚠️ No Spotify clients connected to receive command '{}'", command);
                        }
                    }
                });
            }
        };

        // Handle 'command' events from client (receives track info and responses)
        // This identifies the client as a Spotify plugin
        socket.on("command", {
            let player_state = player_state.clone();
            let client_registry = client_registry.clone();
            let socket_id = socket.id.to_string();
            move |_socket: SocketRef, Data::<serde_json::Value>(data)| {
                let player_state = player_state.clone();
                let client_registry = client_registry.clone();
                let socket_id = socket_id.clone();
                async move {
                    // Register this client as a Spotify client when it sends command events (only if not already registered)
                    {
                        let mut registry = client_registry.write().await;
                        let was_registered = registry.contains_key(&socket_id);
                        if !was_registered {
                            registry.insert(socket_id.clone(), ClientType::Spotify);
                            info!("📱 Registered client {} as Spotify plugin", socket_id);
                        }
                    };

                    let mut player_data = player_state.write().await;
                    let mut track_changed = false;
                    
                    // Store raw data for debugging
                    player_data.raw_data = data.to_string();
                    
                    // Try to parse as Spotify track data
                    if let Ok(spotify_data) = serde_json::from_value::<SpotifyTrackData>(data.clone()) {
                        if let Some(metadata) = spotify_data.metadata {
                            let new_track_title = metadata.title.unwrap_or_else(|| "Unknown Track".to_string());
                            let new_artist_name = metadata.artist_name.unwrap_or_else(|| "Unknown Artist".to_string());
                            let new_album_title = metadata.album_title.unwrap_or_else(|| "Unknown Album".to_string());
                            let new_duration = metadata.duration.unwrap_or_else(|| "0".to_string());
                            
                            // Convert Spotify image URI to CDN URL
                            let new_image_url = metadata.image_url
                                .unwrap_or_else(|| "".to_string())
                                .trim()
                                .to_string();
                            
                            let new_image_url = if new_image_url.starts_with("spotify:image:") {
                                let image_id = new_image_url.strip_prefix("spotify:image:").unwrap_or("");
                                format!("https://i.scdn.co/image/{}", image_id)
                            } else {
                                new_image_url
                            };
                            
                            // Check if track actually changed
                            if new_track_title != player_data.track_title || 
                               new_artist_name != player_data.artist_name ||
                               new_album_title != player_data.album_title {
                                track_changed = true;
                            }
                            
                            player_data.track_title = new_track_title;
                            player_data.artist_name = new_artist_name;
                            player_data.album_title = new_album_title;
                            player_data.duration = new_duration;
                            player_data.image_url = new_image_url;
                        }
                        
                        if let Some(uri) = spotify_data.uri {
                            if uri != player_data.uri {
                                track_changed = true;
                            }
                            player_data.uri = uri;
                        }
                        
                        // Only log and broadcast if track changed
                        if track_changed {
                            info!("🎵 Track changed: '{}' by '{}' from '{}'", 
                                  player_data.track_title, 
                                  player_data.artist_name, 
                                  player_data.album_title);
                            
                            // Broadcast updated player data to all web interface clients
                            let updated_data = player_data.clone();
                            drop(player_data); // Release the write lock
                            
                            tokio::spawn({
                                let io_for_commands = io_for_commands.clone();
                                let client_registry = client_registry.clone();
                                async move {
                                    let registry = client_registry.read().await;
                                    for (_socket_id, client_type) in registry.iter() {
                                        if matches!(client_type, ClientType::WebInterface) {
                                            // Format data to match Python client expectations for broadcasting
                                            let python_compatible_data = serde_json::json!({
                                                "track": {
                                                    "name": updated_data.track_title,
                                                    "artist": updated_data.artist_name,
                                                    "album": updated_data.album_title,
                                                    "image_url": updated_data.image_url,
                                                    "duration_ms": updated_data.duration.parse::<u64>().unwrap_or(0),
                                                    "progress_ms": 0,
                                                    "id": updated_data.uri
                                                }
                                            });
                                            
                                            if let Err(e) = io_for_commands.emit("player_data", &python_compatible_data) {
                                                info!("❌ Failed to broadcast player data update: {}", e);
                                            }
                                        }
                                    }
                                }
                            });
                        }
                    } else if let Ok(track_title) = serde_json::from_value::<String>(data.clone()) {
                        // Fallback: if it's just a string, use it as track title
                        if track_title != player_data.track_title {
                            player_data.track_title = track_title;
                            info!("📝 Track title changed: {}", player_data.track_title);
                        }
                    } else {
                        // Last resort: convert to string representation
                        let new_title = data.to_string();
                        if new_title != player_data.track_title {
                            player_data.track_title = new_title;
                            info!("🔤 Track title updated from raw data");
                        }
                    }
                }
            }
        });

        // Store socket reference for sending commands (for future use)
        let _socket_for_commands = socket.clone();

        // Handle 'input' events from external clients (like web interface)
        // When we receive these, we send them only to Spotify clients
        socket.on("input", {
            let client_registry = client_registry.clone();
            let socket_id = socket.id.to_string();
            let send_to_spotify = send_to_spotify_clients.clone();
            move |_socket: SocketRef, Data::<String>(command)| {
                let client_registry = client_registry.clone();
                let socket_id = socket_id.clone();
                let send_to_spotify = send_to_spotify.clone();
                async move {
                    // Register this client as a web interface client
                    {
                        let mut registry = client_registry.write().await;
                        registry.entry(socket_id.clone()).or_insert(ClientType::WebInterface);
                    }
                    
                    info!("🎮 Received input command from web interface: {}", command);
                    send_to_spotify(&command);
                }
            }
        });

        // Handle direct command events from HTTP clients (like the Python api.py)
        // These should be sent only to Spotify clients as "input" events
        socket.on("PlayPause", {
            let send_to_spotify = send_to_spotify_clients.clone();
            move |_socket: SocketRef| {
                let send_to_spotify = send_to_spotify.clone();
                async move {
                    info!("⏯️ Received PlayPause command from HTTP client - sending to Spotify clients");
                    send_to_spotify("PlayPause");
                }
            }
        });

        socket.on("Next", {
            let send_to_spotify = send_to_spotify_clients.clone();
            move |_socket: SocketRef| {
                let send_to_spotify = send_to_spotify.clone();
                async move {
                    info!("⏭️ Received Next command from HTTP client - sending to Spotify clients");
                    send_to_spotify("Next");
                }
            }
        });

        socket.on("Prev", {
            let send_to_spotify = send_to_spotify_clients.clone();
            move |_socket: SocketRef| {
                let send_to_spotify = send_to_spotify.clone();
                async move {
                    info!("⏮️ Received Prev command from HTTP client - sending to Spotify clients");
                    send_to_spotify("Prev");
                }
            }
        });

        socket.on("Shuffle", {
            let send_to_spotify = send_to_spotify_clients.clone();
            move |_socket: SocketRef| {
                let send_to_spotify = send_to_spotify.clone();
                async move {
                    info!("🔀 Received Shuffle command from HTTP client - sending to Spotify clients");
                    send_to_spotify("Shuffle");
                }
            }
        });

        socket.on("Repeat", {
            let send_to_spotify = send_to_spotify_clients.clone();
            move |_socket: SocketRef| {
                let send_to_spotify = send_to_spotify.clone();
                async move {
                    info!("🔁 Received Repeat command from HTTP client - sending to Spotify clients");
                    send_to_spotify("Repeat");
                }
            }
        });

        socket.on("getdata", {
            let player_state = player_state.clone();
            let send_to_spotify = send_to_spotify_clients.clone();
            move |socket: SocketRef| {
                let player_state = player_state.clone();
                let send_to_spotify = send_to_spotify.clone();
                async move {
                    // Send getdata command to Spotify clients to refresh data (silently)
                    send_to_spotify("getdata");
                    
                    // Also return current player data to the requesting client
                    let current_data = {
                        let data = player_state.read().await;
                        data.clone()
                    };
                    
                    // Format data to match Python client expectations
                    let python_compatible_data = serde_json::json!({
                        "track": {
                            "name": current_data.track_title,
                            "artist": current_data.artist_name,
                            "album": current_data.album_title,
                            "image_url": current_data.image_url,
                            "duration_ms": current_data.duration.parse::<u64>().unwrap_or(0),
                            "progress_ms": 0,
                            "id": current_data.uri
                        }
                    });
                    
                    // Send current player data back to the requesting client (silently)
                    let _ = socket.emit("player_data", &python_compatible_data);
                }
            }
        });

        socket.on("request", {
            let send_to_spotify = send_to_spotify_clients.clone();
            move |_socket: SocketRef, Data::<String>(url)| {
                let send_to_spotify = send_to_spotify.clone();
                async move {
                    info!("🎵 Received request command from HTTP client: {} - sending to Spotify clients", url);
                    let request_command = format!("request {}", url);
                    send_to_spotify(&request_command);
                }
            }
        });

        // Handle disconnect
        socket.on_disconnect({
            let client_registry = client_registry.clone();
            let socket_id = socket.id.to_string();
            move |socket: SocketRef| {
                let client_registry = client_registry.clone();
                let socket_id = socket_id.clone();
                async move {
                    // Remove client from registry
                    {
                        let mut registry = client_registry.write().await;
                        if let Some(client_type) = registry.remove(&socket_id) {
                            info!("🔌 Client {} ({:?}) disconnected", socket.id, client_type);
                        } else {
                            info!("🔌 Client {} disconnected", socket.id);
                        }
                    }
                }
            }
        });
    });

    // Start the server in a tokio task
    let _server_task = tokio::spawn(async move {
        // Get host and port from environment variables
        let host = env::var("IP").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port: u16 = env::var("PORT")
            .unwrap_or_else(|_| "8443".to_string())
            .parse()
            .unwrap_or(8443);
        let addr: String = format!("{}:{}", host, port);
        info!("Starting Socket.IO server on {}", addr);
        let cors = CorsLayer::new()
            .allow_methods(vec![Method::GET, Method::POST])
            .allow_origin(Any);
        let app: axum::Router = axum::Router::new()
            .route("/", axum::routing::get(|| async { "Socket.IO server running" }))
            .layer(layer)
            .layer(cors);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
        Ok::<(), anyhow::Error>(())
    });

    // Wait for shutdown signal from tray
    let _ = shutdown_rx.recv();
    info!("Received exit from system tray, shutting down.");
    // Optionally: gracefully shutdown the server (not strictly needed for axum)
    std::process::exit(0);
}
