mod config;
mod models;
mod docker;
mod pingap;

use crate::config::Config;
use crate::docker::DockerClient;
use crate::pingap::PingapClient;
use anyhow::Result;
use futures::StreamExt;
use tracing::{info, error, warn, Level};
use tracing_subscriber::FmtSubscriber;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Setup Logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO) // Default, will be overridden by env var if we parse it
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // 2. Load Config
    let config = Config::from_env()?;
    
    // Adjust log level based on config
    // Note: tracing_subscriber env filter is better for this, but for simplicity:
    if config.log_level.to_lowercase() == "debug" {
        // Re-init or just rely on RUST_LOG env var which tracing-subscriber uses by default if configured with env_filter
        // For now, let's just log startup
    }

    info!("Starting pingap-docker-provider");
    info!("Pingap Admin URL: {}", config.pingap_admin_url);

    // 3. Initialize Clients
    let docker = DockerClient::new(config.docker_host.clone())?;
    let pingap = PingapClient::new(config.pingap_admin_url.clone());

    // State tracking: ContainerID -> ServiceName
    // This ensures we know which service to remove even if 'die' event lacks attributes or container is gone.
    let mut container_services: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    // 4. Initial Synchronization
    info!("Performing initial synchronization...");
    let containers = docker.get_running_containers().await?;
    for container in containers {
        match container.parse_pingap_config() {
            Ok(Some(service_config)) => {
                info!("Found enabled container: {} -> Service: {}", container.name, service_config.name);
                if let Err(e) = pingap.apply_config(&service_config).await {
                    error!("Failed to apply config for {}: {:?}", container.name, e);
                } else {
                    container_services.insert(container.id.clone(), service_config.name.clone());
                }
            },
            Ok(None) => {
                // Not enabled, ignore
            },
            Err(e) => {
                warn!("Failed to parse labels for container {}: {:?}", container.name, e);
            }
        }
    }
    info!("Initial synchronization complete. Tracking {} services.", container_services.len());

    // 5. Event Loop
    let mut events = docker.subscribe_to_events().await;
    
    info!("Listening for Docker events...");
    
    loop {
        tokio::select! {
            event = events.next() => {
                match event {
                    Some(Ok(msg)) => {
                        let action = msg.action.unwrap_or_default();
                        let actor = msg.actor.unwrap_or_default();
                        let attributes = actor.attributes.unwrap_or_default();
                        let container_id = actor.id.unwrap_or_default();
                        
                        match action.as_str() {
                            "start" => {
                                info!("Container started: {}", container_id);
                                // Inspect to get fresh details
                                match docker.inspect_container(&container_id).await {
                                    Ok(container) => {
                                        match container.parse_pingap_config() {
                                            Ok(Some(service_config)) => {
                                                info!("Applying config for new container: {}", container.name);
                                                if let Err(e) = pingap.apply_config(&service_config).await {
                                                    error!("Failed to apply config for {}: {:?}", container.name, e);
                                                } else {
                                                    container_services.insert(container.id.clone(), service_config.name.clone());
                                                }
                                            },
                                            Ok(None) => {}, // Ignore
                                            Err(e) => warn!("Invalid labels on {}: {:?}", container.name, e),
                                        }
                                    },
                                    Err(e) => error!("Failed to inspect started container {}: {:?}", container_id, e),
                                }
                            },
                            "die" | "stop" => {
                                info!("Container stopped/died: {}", container_id);
                                
                                // Try to get service name from state first
                                let service_name_opt = container_services.remove(&container_id);
                                
                                let service_name = if let Some(name) = service_name_opt {
                                    info!("Found service {} in state for container {}", name, container_id);
                                    Some(name)
                                } else {
                                    // Fallback to attributes if not in state (e.g. started before we started listening and failed sync?)
                                    let name = attributes.get("name").cloned().unwrap_or_default();
                                    let s_name = attributes.get("pingap.service.name")
                                        .cloned()
                                        .unwrap_or_else(|| name.trim_start_matches('/').to_string());
                                        
                                    let enabled = attributes.get("pingap.enable").map(|v| v.as_str()) == Some("true");
                                    if enabled {
                                        Some(s_name)
                                    } else {
                                        None
                                    }
                                };
                                
                                if let Some(service_name) = service_name {
                                    info!("Removing config for service: {}", service_name);
                                    if let Err(e) = pingap.delete_config(&service_name).await {
                                        error!("Failed to delete config for {}: {:?}", service_name, e);
                                    }
                                }
                            },
                            _ => {}
                        }
                    },
                    Some(Err(e)) => {
                        error!("Docker event stream error: {:?}", e);
                    },
                    None => {
                        warn!("Docker event stream ended.");
                        break;
                    }
                }
            },
            _ = signal::ctrl_c() => {
                info!("Received shutdown signal");
                break;
            }
        }
    }

    info!("Shutting down.");
    Ok(())
}
