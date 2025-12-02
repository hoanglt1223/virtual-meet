fn main() {
    // Try to help pkg-config find FFmpeg in CI environment
    if std::env::var("CI").is_ok() {
        // In CI, try to set up FFmpeg paths
        if let Ok(ffmpeg_dir) = std::env::var("FFMPEG_DIR") {
            println!("cargo:rustc-env=FFMPEG_DIR={}", ffmpeg_dir);

            // Add library search path
            let lib_path = std::path::Path::new(&ffmpeg_dir)
                .parent()
                .unwrap_or_else(|| std::path::Path::new("/ffmpeg"))
                .join("lib");

            if lib_path.exists() {
                println!("cargo:rustc-link-search=native={}", lib_path.display());
            }
        }
    }

    tauri_build::build()
}
