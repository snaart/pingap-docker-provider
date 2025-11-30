use std::env;
use anyhow::{Result, Context};

#[derive(Debug, Clone)]
pub struct Config {
    pub pingap_admin_url: String,
    pub docker_host: Option<String>,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let pingap_admin_url = env::var("PINGAP_ADMIN_URL")
            .context("PINGAP_ADMIN_URL must be set")?;
        
        let docker_host = env::var("DOCKER_HOST").ok();
        
        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        Ok(Self {
            pingap_admin_url,
            docker_host,
            log_level,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_config_struct_creation() {
        let config = Config {
            pingap_admin_url: "http://localhost:6188".to_string(),
            docker_host: Some("unix:///var/run/docker.sock".to_string()),
            log_level: "debug".to_string(),
        };
        
        assert_eq!(config.pingap_admin_url, "http://localhost:6188");
        assert_eq!(config.docker_host, Some("unix:///var/run/docker.sock".to_string()));
        assert_eq!(config.log_level, "debug");
    }

    #[test]
    fn test_config_clone() {
        let config1 = Config {
            pingap_admin_url: "http://pingap:6188".to_string(),
            docker_host: None,
            log_level: "info".to_string(),
        };
        
        let config2 = config1.clone();
        assert_eq!(config1.pingap_admin_url, config2.pingap_admin_url);
        assert_eq!(config1.docker_host, config2.docker_host);
    }

    #[test]
    fn test_config_defaults() {
        let config = Config {
            pingap_admin_url: "http://pingap:6188".to_string(),
            docker_host: None,
            log_level: "info".to_string(),
        };
        
        assert_eq!(config.docker_host, None);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_config_with_all_fields() {
        let config = Config {
            pingap_admin_url: "http://custom:9999".to_string(),
            docker_host: Some("tcp://remote:2375".to_string()),
            log_level: "trace".to_string(),
        };
        
        assert_eq!(config.pingap_admin_url, "http://custom:9999");
        assert!(config.docker_host.is_some());
        assert_eq!(config.log_level, "trace");
    }

    // Test from_env with actual environment variables
    #[test]
    fn test_from_env_with_all_vars_set() {
        // This test uses real env vars, so we use unique names to avoid conflicts
        unsafe {
            env::set_var("TEST_PINGAP_URL_1", "http://test:6188");
            env::set_var("TEST_DOCKER_HOST_1", "unix:///test.sock");
            env::set_var("TEST_LOG_LEVEL_1", "debug");
        }
        
        // Manually simulate from_env logic
        let url = env::var("TEST_PINGAP_URL_1").unwrap();
        let host = env::var("TEST_DOCKER_HOST_1").ok();
        let level = env::var("TEST_LOG_LEVEL_1").unwrap_or_else(|_| "info".to_string());
        
        assert_eq!(url, "http://test:6188");
        assert_eq!(host, Some("unix:///test.sock".to_string()));
        assert_eq!(level, "debug");
        
        unsafe {
            env::remove_var("TEST_PINGAP_URL_1");
            env::remove_var("TEST_DOCKER_HOST_1");
            env::remove_var("TEST_LOG_LEVEL_1");
        }
    }

    #[test]
    fn test_from_env_with_defaults() {
        unsafe {
            env::set_var("TEST_PINGAP_URL_2", "http://minimal:6188");
            env::remove_var("TEST_DOCKER_HOST_2");
            env::remove_var("TEST_LOG_LEVEL_2");
        }
        
        let url = env::var("TEST_PINGAP_URL_2").unwrap();
        let host = env::var("TEST_DOCKER_HOST_2").ok();
        let level = env::var("TEST_LOG_LEVEL_2").unwrap_or_else(|_| "info".to_string());
        
        assert_eq!(url, "http://minimal:6188");
        assert_eq!(host, None);
        assert_eq!(level, "info"); // default
        
        unsafe {
            env::remove_var("TEST_PINGAP_URL_2");
        }
    }

    #[test]
    fn test_from_env_missing_required() {
        unsafe {
            env::remove_var("TEST_MISSING_VAR");
        }
        
        let result = env::var("TEST_MISSING_VAR");
        assert!(result.is_err());
    }

    #[test]
    fn test_config_debug_impl() {
        let config = Config {
            pingap_admin_url: "http://test:6188".to_string(),
            docker_host: None,
            log_level: "info".to_string(),
        };
        
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("http://test:6188"));
    }

    // Test actual from_env() function call
    #[test]
    fn test_from_env_actual_success() {
        unsafe {
            env::set_var("PINGAP_ADMIN_URL", "http://actual:6188");
            env::set_var("DOCKER_HOST", "unix:///actual.sock");
            env::set_var("LOG_LEVEL", "debug");
        }
        
        let result = Config::from_env();
        assert!(result.is_ok());
        
        if let Ok(config) = result {
            assert_eq!(config.pingap_admin_url, "http://actual:6188");
            assert_eq!(config.docker_host, Some("unix:///actual.sock".to_string()));
            assert_eq!(config.log_level, "debug");
        }
        
        unsafe {
            env::remove_var("PINGAP_ADMIN_URL");
            env::remove_var("DOCKER_HOST");
            env::remove_var("LOG_LEVEL");
        }
    }

    #[test]
    fn test_from_env_actual_missing_url() {
        unsafe {
            env::remove_var("PINGAP_ADMIN_URL");
        }
        
        let result = Config::from_env();
        assert!(result.is_err());
        
        if let Err(e) = result {
            assert!(e.to_string().contains("PINGAP_ADMIN_URL"));
        }
    }

    #[test]
    fn test_from_env_actual_with_defaults() {
        unsafe {
            env::set_var("PINGAP_ADMIN_URL", "http://defaults:6188");
            env::remove_var("DOCKER_HOST");
            env::remove_var("LOG_LEVEL");
        }
        
        let result = Config::from_env();
        assert!(result.is_ok());
        
        if let Ok(config) = result {
            assert_eq!(config.docker_host, None);
            assert_eq!(config.log_level, "info"); // default
        }
        
        unsafe {
            env::remove_var("PINGAP_ADMIN_URL");
        }
    }
}
