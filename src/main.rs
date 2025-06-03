// Use Windows subsystem to prevent console window by default
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod server;
mod tray;
mod console;

use std::env;
use tracing::info;
use tracing_subscriber;

use console::{show_error_dialog, show_console, hide_console};
use models::{PlayerState, ClientRegistry};
use server::create_server;
use tray::create_tray_icon;

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
    }

    // Create system tray and get shutdown receiver
    let shutdown_rx = create_tray_icon(dev_mode);

    // Shared state
    let player_state = PlayerState::default();
    let client_registry = ClientRegistry::default();

    // Start the server in a tokio task
    let _server_task = tokio::spawn({
        let player_state = player_state.clone();
        let client_registry = client_registry.clone();
        async move {
            if let Err(e) = create_server(player_state, client_registry).await {
                if dev_mode {
                    eprintln!("Server error: {}", e);
                } else {
                    show_error_dialog(&format!("Server error: {}", e));
                }
            }
        }
    });

    // Wait for shutdown signal from tray
    let _ = shutdown_rx.recv();
    info!("Received exit from system tray, shutting down.");
    std::process::exit(0);
}