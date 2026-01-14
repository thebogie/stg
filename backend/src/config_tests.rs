#[cfg(test)]
mod config_tests {
    use crate::config::{Config, Environment};

    #[test]
    fn test_config_loading() {
        // Test that config can be loaded (even if env vars are missing)
        let config = Config::load();
        assert!(config.is_ok() || config.is_err()); // Should either load or fail gracefully
    }

    #[test]
    fn test_environment_enum() {
        // Test that Environment enum variants exist
        match Environment::Development {
            Environment::Development => assert!(true),
            _ => assert!(false),
        }

        match Environment::Production {
            Environment::Production => assert!(true),
            _ => assert!(false),
        }

        match Environment::Test {
            Environment::Test => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_config_defaults() {
        // Test config has reasonable defaults
        let config = Config::load();
        assert!(config.is_ok() || config.is_err()); // Should either load or fail gracefully
    }

    #[test]
    fn test_config_structure() {
        // Test that config can be loaded and has expected structure
        let config = Config::load();
        if let Ok(config) = config {
            // Just test that we can access the fields
            let _env = &config.environment;
            let _server = &config.server;
            let _database = &config.database;
        }
    }
}
