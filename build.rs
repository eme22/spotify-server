fn main() {
    // Check if we're building for dev mode (debug profile)
    let profile = std::env::var("PROFILE").unwrap_or_default();
    
    // Only use Windows subsystem for release builds
    if profile == "release" {
        println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS");
    }
    
    // Compile Windows resource file to embed icon in executable
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resource: {}", e);
        }
    }
}
