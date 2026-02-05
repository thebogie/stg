use wasm_bindgen::prelude::*;

/// Version information for the frontend application
pub struct Version;

impl Version {
    /// Returns the current version of the application
    pub fn current() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// Returns the application name
    pub fn name() -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    /// Returns the full version string with name
    pub fn full() -> String {
        format!("{} v{}", Self::name(), Self::current())
    }

    /// Returns a short version string
    pub fn short() -> String {
        format!("v{}", Self::current())
    }

    /// Returns build information
    pub fn build_info() -> String {
        let version = Self::current();
        let name = Self::name();

        // Try to get build date from environment
        let build_date = option_env!("BUILD_DATE").unwrap_or("unknown");
        let git_commit = option_env!("GIT_COMMIT").unwrap_or("unknown");

        format!(
            "{} v{} (build: {}, commit: {})",
            name, version, build_date, git_commit
        )
    }
}

#[wasm_bindgen]
pub fn get_version() -> String {
    Version::current().to_string()
}

#[wasm_bindgen]
pub fn get_full_version() -> String {
    Version::full()
}

#[wasm_bindgen]
pub fn get_build_info() -> String {
    Version::build_info()
}

/// Returns build metadata as JSON string (industry standard)
#[wasm_bindgen]
pub fn get_build_metadata() -> String {
    let version = Self::current();
    let name = Self::name();
    let build_date = option_env!("BUILD_DATE").unwrap_or("unknown");
    let git_commit = option_env!("GIT_COMMIT").unwrap_or("unknown");
    
    // Format as JSON (simple, no serde_json dependency needed)
    format!(
        r#"{{"name":"{}","version":"{}","build_date":"{}","git_commit":"{}","build_timestamp":{}}}"#,
        name,
        version,
        build_date,
        git_commit,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_current() {
        let version = Version::current();
        assert!(!version.is_empty());
        assert!(version.contains('.'));
    }

    #[test]
    fn test_version_name() {
        let name = Version::name();
        assert!(!name.is_empty());
        assert_eq!(name, "frontend");
    }

    #[test]
    fn test_version_full() {
        let full = Version::full();
        assert!(full.contains("frontend"));
        assert!(full.contains("v"));
    }

    #[test]
    fn test_version_short() {
        let short = Version::short();
        assert!(short.starts_with("v"));
        assert!(short.contains('.'));
    }

    #[test]
    fn test_build_info() {
        let build_info = Version::build_info();
        assert!(build_info.contains("frontend"));
        assert!(build_info.contains("v"));
    }
}
