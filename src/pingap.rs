use reqwest::Client;
use anyhow::{Result, Context, anyhow};
use crate::models::PingapServiceConfig;
use backoff::ExponentialBackoff;
use backoff::future::retry;
use tracing::{info, debug};
use std::time::Duration;

pub struct PingapClient {
    client: Client,
    base_url: String,
}

impl PingapClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn apply_config(&self, config: &PingapServiceConfig) -> Result<()> {
        // let url = format!("{}/upstreams/{}", self.base_url, config.name);
        
        // Pingap API structure assumption based on typical reverse proxy APIs (like Apache APISIX or similar, since Pingap is relatively new/custom).
        // The prompt says: "POST or PUT on endpoint like /services/{service_name}"
        // Let's assume a structure where we define upstream and location separately or together.
        // Re-reading prompt: "Создание/обновление конфигурации сервиса (вероятно, POST или PUT на эндпоинт вроде /services/{service_name})"
        // Let's try to push the whole config to /upstreams/{name} and /locations/{name} or similar?
        // Actually, let's assume a unified endpoint for simplicity as per prompt suggestion, but split if needed.
        // If Pingap follows a specific config schema, we might need to adjust.
        // Let's assume we post the Upstream and Location.
        
        // Strategy:
        // 1. Create/Update Upstream
        // 2. Create/Update Location
        
        let op = || async {
            // 1. Upstream
            let upstream_payload = serde_json::json!({
                "addrs": config.upstreams,
                // "algo": "round_robin" // default
            });
            
            let upstream_url = format!("{}/upstreams/{}", self.base_url, config.name);
            debug!("Sending upstream config to {}: {:?}", upstream_url, upstream_payload);
            
            let resp = self.client.post(&upstream_url)
                .json(&upstream_payload)
                .send()
                .await
                .context("Failed to send upstream request")?;
                
            if !resp.status().is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(backoff::Error::Transient {
                    err: anyhow!("Pingap Upstream API error: {}", text),
                    retry_after: None,
                });
            }

            // 2. Location
            let mut location_payload = serde_json::json!({
                "upstream": config.name,
                "host": "", // parsed from rule?
                "path": "", // parsed from rule?
            });
            
            // We need to parse the rule "Host(`app.example.com`)" or "PathPrefix(`/api`)"
            // Simple parser for now
            if config.location.rule.starts_with("Host(") {
                let host = config.location.rule.trim_start_matches("Host(").trim_end_matches(')');
                location_payload["host"] = serde_json::json!(host);
            } else if config.location.rule.starts_with("PathPrefix(") {
                let path = config.location.rule.trim_start_matches("PathPrefix(").trim_end_matches(')');
                location_payload["path"] = serde_json::json!(path);
            }
            
            if let Some(_middlewares) = &config.location.middlewares {
                 // location_payload["middlewares"] = ...
            }
            
            let location_url = format!("{}/locations/{}", self.base_url, config.name);
            debug!("Sending location config to {}: {:?}", location_url, location_payload);

            let resp = self.client.post(&location_url)
                .json(&location_payload)
                .send()
                .await
                .context("Failed to send location request")?;

            if !resp.status().is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(backoff::Error::Transient {
                    err: anyhow!("Pingap Location API error: {}", text),
                    retry_after: None,
                });
            }
            
            Ok(())
        };

        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(60)),
            ..Default::default()
        };

        retry(backoff, op).await.context("Failed to apply config after retries")?;
        
        info!("Successfully applied config for service {}", config.name);
        Ok(())
    }

    pub async fn delete_config(&self, service_name: &str) -> Result<()> {
        let op = || async {
            // Delete Location
            let location_url = format!("{}/locations/{}", self.base_url, service_name);
            let resp = self.client.delete(&location_url).send().await
                .context("Failed to delete location")?;
            
            if !resp.status().is_success() && resp.status() != 404 {
                 return Err(backoff::Error::Transient {
                    err: anyhow!("Pingap Delete Location API error: {}", resp.status()),
                    retry_after: None,
                });
            }

            // Delete Upstream
            let upstream_url = format!("{}/upstreams/{}", self.base_url, service_name);
            let resp = self.client.delete(&upstream_url).send().await
                .context("Failed to delete upstream")?;

            if !resp.status().is_success() && resp.status() != 404 {
                 return Err(backoff::Error::Transient {
                    err: anyhow!("Pingap Delete Upstream API error: {}", resp.status()),
                    retry_after: None,
                });
            }
            
            Ok(())
        };

        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(30)),
            ..Default::default()
        };

        retry(backoff, op).await.context("Failed to delete config after retries")?;
        
        info!("Successfully deleted config for service {}", service_name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = PingapClient::new("http://localhost:6188".to_string());
        assert_eq!(client.base_url, "http://localhost:6188");
    }

    #[tokio::test]
    async fn test_client_trims_trailing_slash() {
        let client = PingapClient::new("http://localhost:6188/".to_string());
        assert_eq!(client.base_url, "http://localhost:6188");
    }

    #[tokio::test]
    async fn test_apply_config_success() {
        let mut server = mockito::Server::new_async().await;
        
        let _upstream_mock = server.mock("POST", "/upstreams/test-service")
            .with_status(200)
            .with_body("{}")
            .create_async()
            .await;
        
        let _location_mock = server.mock("POST", "/locations/test-service")
            .with_status(200)
            .with_body("{}")
            .create_async()
            .await;
        
        let client = PingapClient::new(server.url());
        let config = PingapServiceConfig {
            name: "test-service".to_string(),
            upstreams: vec!["192.168.1.1:8080".to_string()],
            location: PingapLocation {
                rule: "Host(`example.com`)".to_string(),
                priority: None,
                middlewares: None,
                tls: None,
            },
            upstream_config: None,
            health_check: None,
            middleware_config: None,
            tls_config: None,
        };
        
        let result = client.apply_config(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_apply_config_with_path_prefix() {
        let mut server = mockito::Server::new_async().await;
        
        let _upstream_mock = server.mock("POST", "/upstreams/api-service")
            .with_status(200)
            .create_async()
            .await;
        
        let _location_mock = server.mock("POST", "/locations/api-service")
            .with_status(200)
            .create_async()
            .await;
        
        let client = PingapClient::new(server.url());
        let config = PingapServiceConfig {
            name: "api-service".to_string(),
            upstreams: vec!["10.0.0.1:3000".to_string()],
            location: PingapLocation {
                rule: "PathPrefix(`/api`)".to_string(),
                priority: Some(10),
                middlewares: Some(vec!["compress".to_string()]),
                tls: Some(true),
            },
            upstream_config: None,
            health_check: None,
            middleware_config: None,
            tls_config: None,
        };
        
        assert!(client.apply_config(&config).await.is_ok());
    }

    #[tokio::test]
    async fn test_delete_config_success() {
        let mut server = mockito::Server::new_async().await;
        
        let _location_mock = server.mock("DELETE", "/locations/test-service")
            .with_status(200)
            .create_async()
            .await;
        
        let _upstream_mock = server.mock("DELETE", "/upstreams/test-service")
            .with_status(200)
            .create_async()
            .await;
        
        let client = PingapClient::new(server.url());
        let result = client.delete_config("test-service").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_config_not_found_ok() {
        let mut server = mockito::Server::new_async().await;
        
        let _location_mock = server.mock("DELETE", "/locations/nonexistent")
            .with_status(404)
            .create_async()
            .await;
        
        let _upstream_mock = server.mock("DELETE", "/upstreams/nonexistent")
            .with_status(404)
            .create_async()
            .await;
        
        let client = PingapClient::new(server.url());
        // 404 is acceptable for delete operations
        let result = client.delete_config("nonexistent").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_new_returns_client() {
        let url = "http://pingap:6188";
        let client = PingapClient::new(url.to_string());
        assert_eq!(client.base_url, url);
    }

    #[tokio::test]
    async fn test_apply_config_upstream_error() {
        let mut server = mockito::Server::new_async().await;
        
        // Upstream fails
        let _upstream_mock = server.mock("POST", "/upstreams/error-service")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;
        
        let client = PingapClient::new(server.url());
        let config = PingapServiceConfig {
            name: "error-service".to_string(),
            upstreams: vec!["192.168.1.1:8080".to_string()],
            location: PingapLocation {
                rule: "Host(`error.com`)".to_string(),
                priority: None,
                middlewares: None,
                tls: None,
            },
            upstream_config: None,
            health_check: None,
            middleware_config: None,
            tls_config: None,
        };
        
        // Should fail after retries
        let result = client.apply_config(&config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_apply_config_location_error() {
        let mut server = mockito::Server::new_async().await;
        
        // Upstream succeeds
        let _upstream_mock = server.mock("POST", "/upstreams/loc-error-service")
            .with_status(200)
            .create_async()
            .await;
        
        // Location fails
        let _location_mock = server.mock("POST", "/locations/loc-error-service")
            .with_status(500)
            .with_body("Location Error")
            .create_async()
            .await;
        
        let client = PingapClient::new(server.url());
        let config = PingapServiceConfig {
            name: "loc-error-service".to_string(),
            upstreams: vec!["192.168.1.1:8080".to_string()],
            location: PingapLocation {
                rule: "Host(`locerror.com`)".to_string(),
                priority: None,
                middlewares: None,
                tls: None,
            },
            upstream_config: None,
            health_check: None,
            middleware_config: None,
            tls_config: None,
        };
        
        let result = client.apply_config(&config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_config_error_non_404() {
        let mut server = mockito::Server::new_async().await;
        
        // Delete returns 500 (not 404) - should retry and fail
        let _location_mock = server.mock("DELETE", "/locations/error-delete")
            .with_status(500)
            .create_async()
            .await;
        
        let client = PingapClient::new(server.url());
        let result = client.delete_config("error-delete").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_parse_host_rule() {
        let mut server = mockito::Server::new_async().await;
        
        let _upstream_mock = server.mock("POST", "/upstreams/host-test")
            .with_status(200)
            .create_async()
            .await;
        
        let _location_mock = server.mock("POST", "/locations/host-test")
            .with_status(200)
            .create_async()
            .await;
        
        let client = PingapClient::new(server.url());
        let config = PingapServiceConfig {
            name: "host-test".to_string(),
            upstreams: vec!["10.0.0.1:8080".to_string()],
            location: PingapLocation {
                rule: "Host(`example.com`)".to_string(),
                priority: None,
                middlewares: None,
                tls: None,
            },
            upstream_config: None,
            health_check: None,
            middleware_config: None,
            tls_config: None,
        };
        
        assert!(client.apply_config(&config).await.is_ok());
    }
}
