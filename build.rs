fn main() {
    // Check if we're building for dev mode (debug profile)
    let profile = std::env::var("PROFILE").unwrap_or_default();
    
    // Only use Windows subsystem for release builds
    if profile == "release" {
        println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS");
    }
}
