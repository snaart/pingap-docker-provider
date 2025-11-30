use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};

const LABEL_ENABLE: &str = "pingap.enable";
const LABEL_SERVICE_NAME: &str = "pingap.service.name";
const LABEL_SERVICE_ADDRESS: &str = "pingap.service.address";
const LABEL_SERVICE_PORT: &str = "pingap.service.port";
const LABEL_DOCKER_NETWORK: &str = "pingap.docker.network";
const LABEL_HTTP_RULE: &str = "pingap.http.rule";
const LABEL_HTTP_PRIORITY: &str = "pingap.http.priority";
const LABEL_HTTP_HOST: &str = "pingap.http.host";
const LABEL_HTTP_PATHS: &str = "pingap.http.paths";
const LABEL_MIDDLEWARES: &str = "pingap.http.middlewares";
const LABEL_TLS_ENABLED: &str = "pingap.http.tls.enabled";

// Phase 2: Load Balancing & Health Checks
const LABEL_UPSTREAM_WEIGHT: &str = "pingap.upstream.weight";
const LABEL_UPSTREAM_STRATEGY: &str = "pingap.upstream.strategy";
const LABEL_HEALTH_CHECK_PATH: &str = "pingap.health_check.path";
const LABEL_HEALTH_CHECK_INTERVAL: &str = "pingap.health_check.interval";
const LABEL_HEALTH_CHECK_TIMEOUT: &str = "pingap.health_check.timeout";

// Phase 3: Essential Middlewares
const LABEL_MIDDLEWARE_STRIP_PREFIX: &str = "pingap.middleware.strip_prefix";
const LABEL_MIDDLEWARE_ADD_PREFIX: &str = "pingap.middleware.add_prefix";
const LABEL_HEADERS_CUSTOM_REQUEST: &str = "pingap.headers.custom_request";
const LABEL_HEADERS_CUSTOM_RESPONSE: &str = "pingap.headers.custom_response";
const LABEL_HEADERS_CORS_ENABLE: &str = "pingap.headers.cors.enable";
const LABEL_MIDDLEWARE_COMPRESS: &str = "pingap.middleware.compress";

// Phase 4: Security & Advanced
const LABEL_MIDDLEWARE_RATELIMIT_AVERAGE: &str = "pingap.middleware.ratelimit.average";
const LABEL_MIDDLEWARE_RATELIMIT_BURST: &str = "pingap.middleware.ratelimit.burst";
const LABEL_MIDDLEWARE_BASIC_AUTH: &str = "pingap.middleware.basic_auth";
const LABEL_MIDDLEWARE_REDIRECT_SCHEME: &str = "pingap.middleware.redirect_scheme";
const LABEL_MIDDLEWARE_REDIRECT_REGEX: &str = "pingap.middleware.redirect_regex";
const LABEL_TLS_REDIRECT: &str = "pingap.tls.redirect";
const LABEL_TLS_DOMAINS: &str = "pingap.tls.domains";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingapServiceConfig {
    pub name: String,
    pub upstreams: Vec<String>,
    pub location: PingapLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_config: Option<UpstreamConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check: Option<HealthCheckConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub middleware_config: Option<MiddlewareConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_config: Option<TlsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>, // "round_robin", "hash", "random"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>, // e.g. "10s"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,  // e.g. "5s"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiddlewareConfig {
    // Phase 3: Path Manipulation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strip_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_prefix: Option<String>,
    
    // Phase 3: Headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_request_headers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_response_headers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cors_enabled: Option<bool>,
    
    // Phase 3: Performance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compress: Option<bool>,
    
    // Phase 4: Rate Limiting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ratelimit_average: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ratelimit_burst: Option<u32>,
    
    // Phase 4: Authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basic_auth: Option<String>,
    
    // Phase 4: Redirects
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_scheme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_regex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domains: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingapLocation {
    pub rule: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub middlewares: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<bool>,
}

pub struct ContainerInfo {
    #[allow(dead_code)]
    pub id: String,
    pub name: String,
    pub labels: HashMap<String, String>,
    pub ip_address: Option<String>,
    pub ports: Vec<u16>,
    pub networks: HashMap<String, String>, // network name -> IP address
}

impl ContainerInfo {
    pub fn parse_pingap_config(&self) -> Result<Option<PingapServiceConfig>> {
        // Check if enabled
        if self.labels.get(LABEL_ENABLE).map(|v| v.as_str()) != Some("true") {
            return Ok(None);
        }

        // Get Service Name
        let name = self.labels.get(LABEL_SERVICE_NAME)
            .cloned()
            .unwrap_or_else(|| self.name.trim_start_matches('/').to_string());

        // Get IP Address (with network override support)
        let ip = if let Some(network_name) = self.labels.get(LABEL_DOCKER_NETWORK) {
            // User specified a specific network
            self.networks.get(network_name)
                .ok_or_else(|| anyhow!("Container {} is not connected to network '{}'. Available networks: {:?}", 
                    self.name, network_name, self.networks.keys().collect::<Vec<_>>()))?
                .clone()
        } else {
            // Use default IP (first network or primary IP)
            self.ip_address.clone()
                .or_else(|| self.networks.values().next().cloned())
                .ok_or_else(|| anyhow!("No IP address found for container {}", self.name))?
        };

        // Get Port (with explicit override support)
        let port = if let Some(port_str) = self.labels.get(LABEL_SERVICE_PORT) {
            port_str.parse::<u16>()
                .map_err(|e| anyhow!("Invalid port '{}': {}", port_str, e))?
        } else {
            // Auto-detect first exposed port
            *self.ports.first()
                .ok_or_else(|| anyhow!("No exposed ports found for container {}. Use {} label to specify port explicitly.", 
                    self.name, LABEL_SERVICE_PORT))?
        };

        // Build upstream address (override if LABEL_SERVICE_ADDRESS is set)
        let address = self.labels.get(LABEL_SERVICE_ADDRESS)
            .cloned()
            .unwrap_or_else(|| format!("{}:{}", ip, port));

        // Build routing rule (supports explicit rule, or simplified host/paths)
        let rule = if let Some(explicit_rule) = self.labels.get(LABEL_HTTP_RULE) {
            // User provided explicit rule like "Host(`example.com`) && PathPrefix(`/api`)"
            explicit_rule.clone()
        } else {
            // Try simplified aliases
            let host_rule = self.labels.get(LABEL_HTTP_HOST)
                .map(|h| format!("Host(`{}`)", h));
            
            let path_rules = self.labels.get(LABEL_HTTP_PATHS)
                .map(|paths| {
                    paths.split(',')
                        .map(|p| format!("PathPrefix(`{}`)", p.trim()))
                        .collect::<Vec<_>>()
                        .join(" || ")
                });

            match (host_rule, path_rules) {
                (Some(h), Some(p)) => format!("{} && ({})", h, p),
                (Some(h), None) => h,
                (None, Some(p)) => p,
                (None, None) => {
                    return Err(anyhow!(
                        "Container {} has pingap.enable=true but no routing rule. \
                        Provide one of: {}, {}, or {}",
                        self.name, LABEL_HTTP_RULE, LABEL_HTTP_HOST, LABEL_HTTP_PATHS
                    ));
                }
            }
        };

        // Get Priority
        let priority = self.labels.get(LABEL_HTTP_PRIORITY)
            .and_then(|p| p.parse::<i32>().ok());

        // Get Middlewares
        let middlewares = self.labels.get(LABEL_MIDDLEWARES)
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect());

        // Get TLS
        let tls = self.labels.get(LABEL_TLS_ENABLED)
            .map(|v| v == "true");

        // Phase 2: Upstream Configuration
        let upstream_config = {
            let weight = self.labels.get(LABEL_UPSTREAM_WEIGHT)
                .and_then(|w| w.parse::<u32>().ok());
            
            let strategy = self.labels.get(LABEL_UPSTREAM_STRATEGY)
                .map(|s| s.clone());

            if weight.is_some() || strategy.is_some() {
                Some(UpstreamConfig { weight, strategy })
            } else {
                None
            }
        };

        // Phase 2: Health Check Configuration
        let health_check = self.labels.get(LABEL_HEALTH_CHECK_PATH)
            .map(|path| HealthCheckConfig {
                path: path.clone(),
                interval: self.labels.get(LABEL_HEALTH_CHECK_INTERVAL).cloned(),
                timeout: self.labels.get(LABEL_HEALTH_CHECK_TIMEOUT).cloned(),
            });

        // Phase 3 & 4: Middleware Configuration
        let middleware_config = {
            let strip_prefix = self.labels.get(LABEL_MIDDLEWARE_STRIP_PREFIX).cloned();
            let add_prefix = self.labels.get(LABEL_MIDDLEWARE_ADD_PREFIX).cloned();
            
            let custom_request_headers = self.labels.get(LABEL_HEADERS_CUSTOM_REQUEST)
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect());
            
            let custom_response_headers = self.labels.get(LABEL_HEADERS_CUSTOM_RESPONSE)
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect());
            
            let cors_enabled = self.labels.get(LABEL_HEADERS_CORS_ENABLE)
                .map(|v| v == "true");
            
            let compress = self.labels.get(LABEL_MIDDLEWARE_COMPRESS)
                .map(|v| v == "true");
            
            let ratelimit_average = self.labels.get(LABEL_MIDDLEWARE_RATELIMIT_AVERAGE)
                .and_then(|v| v.parse::<u32>().ok());
            
            let ratelimit_burst = self.labels.get(LABEL_MIDDLEWARE_RATELIMIT_BURST)
                .and_then(|v| v.parse::<u32>().ok());
            
            let basic_auth = self.labels.get(LABEL_MIDDLEWARE_BASIC_AUTH).cloned();
            
            let redirect_scheme = self.labels.get(LABEL_MIDDLEWARE_REDIRECT_SCHEME).cloned();
            
            let redirect_regex = self.labels.get(LABEL_MIDDLEWARE_REDIRECT_REGEX).cloned();
            
            // Only create MiddlewareConfig if at least one middleware is configured
            if strip_prefix.is_some() || add_prefix.is_some() || custom_request_headers.is_some() ||
               custom_response_headers.is_some() || cors_enabled.is_some() || compress.is_some() ||
               ratelimit_average.is_some() || ratelimit_burst.is_some() || basic_auth.is_some() ||
               redirect_scheme.is_some() || redirect_regex.is_some() {
                Some(MiddlewareConfig {
                    strip_prefix,
                    add_prefix,
                    custom_request_headers,
                    custom_response_headers,
                    cors_enabled,
                    compress,
                    ratelimit_average,
                    ratelimit_burst,
                    basic_auth,
                    redirect_scheme,
                    redirect_regex,
                })
            } else {
                None
            }
        };

        // Phase 4: TLS Advanced Configuration
        let tls_config = if tls == Some(true) {
            let redirect = self.labels.get(LABEL_TLS_REDIRECT)
                .map(|v| v == "true");
            
            let domains = self.labels.get(LABEL_TLS_DOMAINS)
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect());
            
            Some(TlsConfig {
                enabled: true,
                redirect,
                domains,
            })
        } else {
            None
        };

        Ok(Some(PingapServiceConfig {
            name,
            upstreams: vec![address],
            location: PingapLocation {
                rule,
                priority,
                middlewares,
                tls,
            },
            upstream_config,
            health_check,
            middleware_config,
            tls_config,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_container(labels: HashMap<String, String>) -> ContainerInfo {
        ContainerInfo {
            id: "test123".to_string(),
            name: "/test-container".to_string(),
            labels,
            ip_address: Some("192.168.1.100".to_string()),
            ports: vec![8080],
            networks: HashMap::from([
                ("bridge".to_string(), "172.17.0.2".to_string()),
                ("custom".to_string(), "192.168.1.100".to_string()),
            ]),
        }
    }

    #[test]
    fn test_disabled_container() {
        let container = create_test_container(HashMap::new());
        assert!(container.parse_pingap_config().unwrap().is_none());
    }

    #[test]
    fn test_basic_host_alias() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_HTTP_HOST.to_string(), "example.com".to_string());
        
        let config = create_test_container(labels).parse_pingap_config().unwrap().unwrap();
        assert_eq!(config.location.rule, "Host(`example.com`)");
    }

    #[test]
    fn test_path_alias() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_HTTP_PATHS.to_string(), "/api,/v1".to_string());
        
        let config = create_test_container(labels).parse_pingap_config().unwrap().unwrap();
        assert!(config.location.rule.contains("PathPrefix(`/api`)"));
        assert!(config.location.rule.contains("PathPrefix(`/v1`)"));
    }

    #[test]
    fn test_explicit_port() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_SERVICE_PORT.to_string(), "3000".to_string());
        labels.insert(LABEL_HTTP_HOST.to_string(), "app.local".to_string());
        
        let config = create_test_container(labels).parse_pingap_config().unwrap().unwrap();
        assert_eq!(config.upstreams[0], "192.168.1.100:3000");
    }

    #[test]
    fn test_network_selection() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_DOCKER_NETWORK.to_string(), "bridge".to_string());
        labels.insert(LABEL_HTTP_HOST.to_string(), "app.local".to_string());
        
        let config = create_test_container(labels).parse_pingap_config().unwrap().unwrap();
        assert_eq!(config.upstreams[0], "172.17.0.2:8080");
    }

    #[test]
    fn test_priority() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_HTTP_HOST.to_string(), "app.local".to_string());
        labels.insert(LABEL_HTTP_PRIORITY.to_string(), "10".to_string());
        
        let config = create_test_container(labels).parse_pingap_config().unwrap().unwrap();
        assert_eq!(config.location.priority, Some(10));
    }

    #[test]
    fn test_upstream_config() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_HTTP_HOST.to_string(), "app.local".to_string());
        labels.insert(LABEL_UPSTREAM_WEIGHT.to_string(), "50".to_string());
        labels.insert(LABEL_UPSTREAM_STRATEGY.to_string(), "hash".to_string());
        
        let config = create_test_container(labels).parse_pingap_config().unwrap().unwrap();
        assert!(config.upstream_config.is_some());
        let uc = config.upstream_config.unwrap();
        assert_eq!(uc.weight, Some(50));
        assert_eq!(uc.strategy, Some("hash".to_string()));
    }

    #[test]
    fn test_health_check() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_HTTP_HOST.to_string(), "app.local".to_string());
        labels.insert(LABEL_HEALTH_CHECK_PATH.to_string(), "/health".to_string());
        labels.insert(LABEL_HEALTH_CHECK_INTERVAL.to_string(), "10s".to_string());
        
        let config = create_test_container(labels).parse_pingap_config().unwrap().unwrap();
        let hc = config.health_check.unwrap();
        assert_eq!(hc.path, "/health");
        assert_eq!(hc.interval, Some("10s".to_string()));
    }

    #[test]
    fn test_middlewares() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_HTTP_HOST.to_string(), "app.local".to_string());
        labels.insert(LABEL_MIDDLEWARE_STRIP_PREFIX.to_string(), "/api".to_string());
        labels.insert(LABEL_MIDDLEWARE_COMPRESS.to_string(), "true".to_string());
        labels.insert(LABEL_MIDDLEWARE_RATELIMIT_AVERAGE.to_string(), "100".to_string());
        
        let config = create_test_container(labels).parse_pingap_config().unwrap().unwrap();
        let mw = config.middleware_config.unwrap();
        assert_eq!(mw.strip_prefix, Some("/api".to_string()));
        assert_eq!(mw.compress, Some(true));
        assert_eq!(mw.ratelimit_average, Some(100));
    }

    #[test]
    fn test_tls_config() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_HTTP_HOST.to_string(), "app.local".to_string());
        labels.insert(LABEL_TLS_ENABLED.to_string(), "true".to_string());
        labels.insert(LABEL_TLS_REDIRECT.to_string(), "true".to_string());
        labels.insert(LABEL_TLS_DOMAINS.to_string(), "example.com".to_string());
        
        let config = create_test_container(labels).parse_pingap_config().unwrap().unwrap();
        assert_eq!(config.location.tls, Some(true));
        let tls = config.tls_config.unwrap();
        assert!(tls.enabled);
        assert_eq!(tls.redirect, Some(true));
    }

    #[test]
    fn test_missing_routing_rule_error() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        
        let result = create_test_container(labels).parse_pingap_config();
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_network_error() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_DOCKER_NETWORK.to_string(), "nonexistent".to_string());
        labels.insert(LABEL_HTTP_HOST.to_string(), "app.local".to_string());
        
        assert!(create_test_container(labels).parse_pingap_config().is_err());
    }

    #[test]
    fn test_explicit_service_address() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_SERVICE_ADDRESS.to_string(), "10.0.0.5:9000".to_string());
        labels.insert(LABEL_HTTP_HOST.to_string(), "app.local".to_string());
        
        let config = create_test_container(labels).parse_pingap_config().unwrap().unwrap();
        assert_eq!(config.upstreams[0], "10.0.0.5:9000");
    }

    #[test]
    fn test_explicit_rule() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_HTTP_RULE.to_string(), "Host(`custom.com`) && Path(`/special`)".to_string());
        
        let config = create_test_container(labels).parse_pingap_config().unwrap().unwrap();
        assert_eq!(config.location.rule, "Host(`custom.com`) && Path(`/special`)");
    }

    #[test]
    fn test_host_and_paths_combined() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_HTTP_HOST.to_string(), "api.example.com".to_string());
        labels.insert(LABEL_HTTP_PATHS.to_string(), "/v1,/v2".to_string());
        
        let config = create_test_container(labels).parse_pingap_config().unwrap().unwrap();
        assert!(config.location.rule.contains("Host(`api.example.com`)"));
        assert!(config.location.rule.contains("PathPrefix(`/v1`)"));
    }

    #[test]
    fn test_legacy_middlewares_label() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_HTTP_HOST.to_string(), "app.local".to_string());
        labels.insert(LABEL_MIDDLEWARES.to_string(), "compress,auth".to_string());
        
        let config = create_test_container(labels).parse_pingap_config().unwrap().unwrap();
        assert_eq!(config.location.middlewares, Some(vec!["compress".to_string(), "auth".to_string()]));
    }

    #[test]
    fn test_tls_without_advanced_config() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_HTTP_HOST.to_string(), "app.local".to_string());
        labels.insert(LABEL_TLS_ENABLED.to_string(), "false".to_string());
        
        let config = create_test_container(labels).parse_pingap_config().unwrap().unwrap();
        // TLS enabled is false, not None
        assert_eq!(config.location.tls, Some(false));
        assert!(config.tls_config.is_none());
    }

    #[test]
    fn test_all_middlewares_at_once() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_HTTP_HOST.to_string(), "app.local".to_string());
        labels.insert(LABEL_MIDDLEWARE_STRIP_PREFIX.to_string(), "/old".to_string());
        labels.insert(LABEL_MIDDLEWARE_ADD_PREFIX.to_string(), "/new".to_string());
        labels.insert(LABEL_HEADERS_CUSTOM_REQUEST.to_string(), "X-Req:val".to_string());
        labels.insert(LABEL_HEADERS_CUSTOM_RESPONSE.to_string(), "X-Resp:val".to_string());
        labels.insert(LABEL_HEADERS_CORS_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_MIDDLEWARE_COMPRESS.to_string(), "true".to_string());
        labels.insert(LABEL_MIDDLEWARE_RATELIMIT_AVERAGE.to_string(), "50".to_string());
        labels.insert(LABEL_MIDDLEWARE_RATELIMIT_BURST.to_string(), "25".to_string());
        labels.insert(LABEL_MIDDLEWARE_BASIC_AUTH.to_string(), "user:pass".to_string());
        labels.insert(LABEL_MIDDLEWARE_REDIRECT_SCHEME.to_string(), "https".to_string());
        labels.insert(LABEL_MIDDLEWARE_REDIRECT_REGEX.to_string(), "^old->new".to_string());
        
        let config = create_test_container(labels).parse_pingap_config().unwrap().unwrap();
        let mw = config.middleware_config.unwrap();
        assert_eq!(mw.strip_prefix, Some("/old".to_string()));
        assert_eq!(mw.add_prefix, Some("/new".to_string()));
        assert_eq!(mw.compress, Some(true));
        assert_eq!(mw.ratelimit_average, Some(50));
        assert_eq!(mw.ratelimit_burst, Some(25));
        assert_eq!(mw.basic_auth, Some("user:pass".to_string()));
    }

    #[test]
    fn test_service_name_from_container_name() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_HTTP_HOST.to_string(), "test.local".to_string());
        
        let config = create_test_container(labels).parse_pingap_config().unwrap().unwrap();
        // Container name is "/test-container", service name should be "test-container" (strip leading /)
        assert_eq!(config.name, "test-container");
    }

    #[test]
    fn test_invalid_priority() {
        let mut labels = HashMap::new();
        labels.insert(LABEL_ENABLE.to_string(), "true".to_string());
        labels.insert(LABEL_HTTP_HOST.to_string(), "app.local".to_string());
        labels.insert(LABEL_HTTP_PRIORITY.to_string(), "invalid".to_string());
        
        let config = create_test_container(labels).parse_pingap_config().unwrap().unwrap();
        // Invalid priority should be None
        assert_eq!(config.location.priority, None);
    }
}
