pub struct Config;

impl Config {
    pub fn api_base_url() -> String {
        // Check if we're running in a development environment
        // In development, Trunk serves the frontend and proxies /api/ to backend (port from .env.development)
        // In production, nginx serves the frontend and proxies /api/ to the backend container

        // For now, we'll use a simple approach: always use relative URLs
        // This works for both development (Trunk proxy) and production (nginx proxy)
        "".to_string()
    }
}
