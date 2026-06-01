fn main() {
    let profile = std::env::var("PROFILE").unwrap_or_default();
    if profile == "release" {
        let url = std::env::var("STG_API_URL")
            .unwrap_or_else(|_| "https://smacktalkgaming.com".to_string());
        println!("cargo:rustc-env=STG_API_URL={url}");
    }
    tauri_build::build()
}
