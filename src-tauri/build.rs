fn main() {
    // Tauri embeds the frontend from ../src at build time.
    for file in [
        "../src/index.html",
        "../src/styles.css",
        "../src/app.js",
        "../src/bridge.js",
    ] {
        println!("cargo:rerun-if-changed={file}");
    }
    tauri_build::build()
}
