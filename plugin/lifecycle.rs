use anyhow::Result;
use tracing::info;
use std::collections::HashMap;

use super::{PluginManifest, PluginState};

/// Plugin Lifecycle Manager - quản lý init → start → stop cycle
/// Inspired by Hermes Agent's plugin hooks system
pub struct LifecycleManager;

impl LifecycleManager {
    pub fn new() -> Self {
        Self
    }

    /// Initialize a plugin with config
    pub fn init(&self, manifest: &PluginManifest, config: HashMap<String, serde_json::Value>) -> Result<(String, PluginState)> {
        info!("Initializing plugin: {} v{}", manifest.name, manifest.version);
        
        // Validate required config fields
        if let Some(schema) = &manifest.config_schema {
            self.validate_config(schema, &config)?;
        }

        // Validate dependencies
        for dep in &manifest.dependencies {
            info!("  Dependency required: {}", dep);
        }

        Ok((manifest.id.clone(), PluginState::Initialized))
    }

    /// Start a plugin
    pub fn start(&self, manifest: &PluginManifest) -> Result<(String, PluginState)> {
        info!("Starting plugin: {}", manifest.name);
        info!("  Entry point: {}", manifest.entry_point);
        info!("  Permissions: {:?}", manifest.permissions);
        Ok((manifest.id.clone(), PluginState::Running))
    }

    /// Stop a plugin
    pub fn stop(&self, manifest: &PluginManifest) -> Result<(String, PluginState)> {
        info!("Stopping plugin: {}", manifest.name);
        Ok((manifest.id.clone(), PluginState::Stopped))
    }

    /// Restart a plugin
    pub fn restart(&self, manifest: &PluginManifest, 
                   config: HashMap<String, serde_json::Value>) -> Result<(String, PluginState)> {
        self.stop(manifest)?;
        self.init(manifest, config)?;
        self.start(manifest)
    }

    /// Validate config against schema
    fn validate_config(&self, schema: &HashMap<String, serde_json::Value>, 
                       config: &HashMap<String, serde_json::Value>) -> Result<()> {
        for (key, _schema_val) in schema {
            if !config.contains_key(key) {
                return Err(anyhow::anyhow!("Missing required config field: {}", key));
            }
        }
        Ok(())
    }
}
