use std::sync::mpsc;
use std::thread;
use image::GenericImageView;
use rust_embed::RustEmbed;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder,
};
use crate::console::show_error_dialog;

#[derive(RustEmbed)]
#[folder = "assets/"]
pub struct Assets;

pub fn create_tray_icon(dev_mode: bool) -> mpsc::Receiver<()> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

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
        }
        
        let icon = icon.unwrap();
        
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
            .build()
        {
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
                        let event_id = event.id.0.as_str();
                        match event_id {
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
            use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG};
            
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

    shutdown_rx
}
