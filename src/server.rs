use std::env;
use http::Method;
use socketioxide::{
    extract::{Data, SocketRef},
    SocketIo,
};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::models::{PlayerState, ClientRegistry, ClientType, SpotifyTrackData};

pub async fn create_server(
    player_state: PlayerState,
    client_registry: ClientRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create Socket.IO layer
    let (layer, io) = SocketIo::new_layer();

    // Clone for use in Socket.IO handlers
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
        socket.on("command", {
            let player_state = player_state.clone();
            let client_registry = client_registry.clone();
            let socket_id = socket.id.to_string();
            move |_socket: SocketRef, Data::<serde_json::Value>(data)| {
                let player_state = player_state.clone();
                let client_registry = client_registry.clone();
                let socket_id = socket_id.clone();
                async move {
                    // Register this client as a Spotify client
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

        // Handle 'input' events from external clients (like web interface)
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

        // Handle direct command events from HTTP clients
        setup_command_handlers(&socket, &send_to_spotify_clients);

        // Handle getdata event
        socket.on("getdata", {
            let player_state = player_state.clone();
            let send_to_spotify = send_to_spotify_clients.clone();
            move |socket: SocketRef| {
                let player_state = player_state.clone();
                let send_to_spotify = send_to_spotify.clone();
                async move {
                    // Send getdata command to Spotify clients to refresh data
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
                    
                    // Send current player data back to the requesting client
                    let _ = socket.emit("player_data", &python_compatible_data);
                }
            }
        });

        // Handle request event
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

    // Start the server
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
    
    Ok(())
}

fn setup_command_handlers<F>(socket: &SocketRef, send_to_spotify_clients: &F)
where
    F: Fn(&str) + Clone + Send + Sync + 'static,
{
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
}
