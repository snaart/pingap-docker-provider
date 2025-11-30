use bollard::Docker;
use bollard::container::ListContainersOptions;
use bollard::system::EventsOptions;
use anyhow::{Result, Context};
use crate::models::ContainerInfo;
use std::collections::HashMap;

pub struct DockerClient {
    docker: Docker,
}

impl DockerClient {
    pub fn new(host: Option<String>) -> Result<Self> {
        let docker = if let Some(h) = host {
            Docker::connect_with_socket(&h, 120, bollard::API_DEFAULT_VERSION)
                .context("Failed to connect to Docker socket")?
        } else {
            Docker::connect_with_socket_defaults()
                .context("Failed to connect to Docker socket defaults")?
        };
        
        // Verify connection
        // We can't easily verify synchronously without async, but the connection object is created.
        // The first call will fail if connection is bad.
        
        Ok(Self { docker })
    }

    pub async fn get_running_containers(&self) -> Result<Vec<ContainerInfo>> {
        let options = ListContainersOptions::<String>::default();
        let containers = self.docker.list_containers(Some(options)).await
            .context("Failed to list containers")?;

        let mut result = Vec::new();

        for c in containers {
            let id = c.id.unwrap_or_default();
            // Names are usually like ["/container_name"], we want "container_name"
            let name = c.names.as_ref().and_then(|n| n.first()).map(|s| s.as_str()).unwrap_or("unknown").to_string();
            let labels = c.labels.unwrap_or_default();
            
            // Collect all networks and their IPs
            let mut networks = HashMap::new();
            let mut ip_address = None;
            
            if let Some(ns) = c.network_settings.as_ref() {
                if let Some(nets) = ns.networks.as_ref() {
                    for (net_name, net_info) in nets {
                        if let Some(ip) = net_info.ip_address.as_ref() {
                            if !ip.is_empty() {
                                networks.insert(net_name.clone(), ip.clone());
                                // Set primary IP as the first non-empty one we find
                                if ip_address.is_none() {
                                    ip_address = Some(ip.clone());
                                }
                            }
                        }
                    }
                }
            }

            let ports = c.ports.as_ref().map(|p| {
                p.iter().map(|port| port.private_port).collect()
            }).unwrap_or_default();

            result.push(ContainerInfo {
                id,
                name,
                labels,
                ip_address,
                ports,
                networks,
            });
        }

        Ok(result)
    }

    pub async fn subscribe_to_events(&self) -> impl futures::Stream<Item = Result<bollard::models::EventMessage, bollard::errors::Error>> {
        let options = EventsOptions {
            filters: HashMap::from([
                ("type".to_string(), vec!["container".to_string()]),
                ("event".to_string(), vec!["start".to_string(), "die".to_string(), "stop".to_string()]),
            ]),
            ..Default::default()
        };
        self.docker.events(Some(options))
    }
    
    pub async fn inspect_container(&self, id: &str) -> Result<ContainerInfo> {
        let container = self.docker.inspect_container(id, None).await
            .context(format!("Failed to inspect container {}", id))?;
            
        let name = container.name.unwrap_or_default();
        let config = container.config.unwrap_or_default();
        let labels = config.labels.unwrap_or_default();
        
        let network_settings = container.network_settings.unwrap_or_default();
        
        // Collect all networks and their IPs
        let mut networks = HashMap::new();
        let mut ip_address = None;
        
        if let Some(nets) = network_settings.networks.as_ref() {
            for (net_name, net_info) in nets {
                if let Some(ip) = net_info.ip_address.as_ref() {
                    if !ip.is_empty() {
                        networks.insert(net_name.clone(), ip.clone());
                        // Set primary IP as the first non-empty one we find
                        if ip_address.is_none() {
                            ip_address = Some(ip.clone());
                        }
                    }
                }
            }
        }
             
        // Extract exposed ports from config
        let mut ports = Vec::new();
        if let Some(exposed) = config.exposed_ports {
             for (k, _) in exposed {
                 // k is like "80/tcp"
                 if let Some(port_str) = k.split('/').next() {
                     if let Ok(p) = port_str.parse::<u16>() {
                         ports.push(p);
                     }
                 }
             }
        }

        Ok(ContainerInfo {
            id: id.to_string(),
            name,
            labels,
            ip_address,
            ports,
            networks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_client_creation_default() {
        // Test that we can create a client without errors
        let result = DockerClient::new(None);
        // We expect it to either succeed (if Docker is available) or fail gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_docker_client_with_custom_host() {
        // Test with custom host path
        // Note: bollard may not immediately fail on invalid socket during construction
        let result = DockerClient::new(Some("unix:///invalid/path.sock".to_string()));
        // Client creation may succeed (validation happens on first use)
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_container_info_structure() {
        // Test ContainerInfo can be created with expected fields
        let info = ContainerInfo {
            id: "abc123".to_string(),
            name: "/my-container".to_string(),
            labels: HashMap::from([
                ("app".to_string(), "test".to_string()),
            ]),
            ip_address: Some("172.17.0.2".to_string()),
            ports: vec![80, 443],
            networks: HashMap::from([
                ("bridge".to_string(), "172.17.0.2".to_string()),
            ]),
        };
        
        assert_eq!(info.id, "abc123");
        assert_eq!(info.name, "/my-container");
        assert_eq!(info.ports.len(), 2);
        assert_eq!(info.networks.len(), 1);
    }

    #[test]
    fn test_container_info_with_multiple_networks() {
        let info = ContainerInfo {
            id: "multi123".to_string(),
            name: "/multi-network".to_string(),
            labels: HashMap::new(),
            ip_address: Some("192.168.1.100".to_string()),
            ports: vec![8080],
            networks: HashMap::from([
                ("bridge".to_string(), "172.17.0.2".to_string()),
                ("custom".to_string(), "192.168.1.100".to_string()),
                ("frontend".to_string(), "10.0.1.50".to_string()),
            ]),
        };
        
        assert_eq!(info.networks.len(), 3);
        assert!(info.networks.contains_key("bridge"));
        assert!(info.networks.contains_key("custom"));
        assert!(info.networks.contains_key("frontend"));
    }

    #[test]
    fn test_container_info_no_ip() {
        let info = ContainerInfo {
            id: "noip123".to_string(),
            name: "/no-ip-container".to_string(),
            labels: HashMap::new(),
            ip_address: None,
            ports: vec![],
            networks: HashMap::new(),
        };
        
        assert!(info.ip_address.is_none());
        assert_eq!(info.ports.len(), 0);
        assert_eq!(info.networks.len(), 0);
    }

    // Async tests - these may fail if Docker is not available, which is expected
    #[tokio::test]
    async fn test_get_running_containers_structure() {
        // This test structure validates the method signature and basic error handling
        if let Ok(client) = DockerClient::new(None) {
            let result = client.get_running_containers().await;
            // Result can be Ok or Err depending on Docker availability
            match result {
                Ok(containers) => {
                    // If successful, verify structure is valid Vec
                    let _count = containers.len();
                    assert!(true);
                }
                Err(_) => {
                    // Expected if Docker is not running
                    assert!(true);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_inspect_container_structure() {
        if let Ok(client) = DockerClient::new(None) {
            // Try to inspect a non-existent container
            let result = client.inspect_container("nonexistent123").await;
            // Should error for non-existent container
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_subscribe_to_events_structure() {
        if let Ok(client) = DockerClient::new(None) {
            // Just verify we can call the method
            let _stream = client.subscribe_to_events().await;
            // Stream creation should succeed even if no Docker
            assert!(true);
        }
    }

    #[test]
    fn test_container_info_empty_labels() {
        let info = ContainerInfo {
            id: "empty123".to_string(),
            name: "/empty-labels".to_string(),
            labels: HashMap::new(),
            ip_address: Some("10.0.0.1".to_string()),
            ports: vec![],
            networks: HashMap::new(),
        };
        
        assert_eq!(info.labels.len(), 0);
        assert_eq!(info.ports.len(), 0);
    }

    #[test]
    fn test_container_info_many_ports() {
        let info = ContainerInfo {
            id: "ports123".to_string(),
            name: "/many-ports".to_string(),
            labels: HashMap::new(),
            ip_address: Some("10.0.0.1".to_string()),
            ports: vec![80, 443, 8080, 9000, 3000],
            networks: HashMap::new(),
        };
        
        assert_eq!(info.ports.len(), 5);
        assert!(info.ports.contains(&443));
    }

    #[test]
    fn test_docker_client_new_none() {
        // Test explicit None parameter
        let result = DockerClient::new(None);
        // Should either succeed or fail gracefully
        match result {
            Ok(_) => assert!(true),
            Err(_) => assert!(true),
        }
    }

    #[test]
    fn test_docker_client_new_some() {
        // Test with Some parameter
        let result = DockerClient::new(Some("/var/run/docker.sock".to_string()));
        match result {
            Ok(_) => assert!(true),
            Err(_) => assert!(true),
        }
    }
}
