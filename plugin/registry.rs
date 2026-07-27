use std::collections::HashMap;
use anyhow::Result;
use tracing::info;

use super::{PluginManifest, PluginType, PluginState};

/// Plugin Registry - quản lý plugin discovery và lifecycle
/// Inspired by Hermes Agent's PluginManager và OpenClaw's extension system
pub struct PluginRegistry {
    plugins: HashMap<String, PluginManifest>,
    states: HashMap<String, PluginState>,
    categories: HashMap<PluginType, Vec<String>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            states: HashMap::new(),
            categories: HashMap::new(),
        }
    }

    /// Register a plugin manifest
    pub fn register(&mut self, manifest: PluginManifest) -> Result<()> {
        let id = manifest.id.clone();
        let plugin_type = manifest.plugin_type.clone();
        
        info!("Registering plugin: {} ({:?})", manifest.name, plugin_type);
        self.plugins.insert(id.clone(), manifest);
        self.states.insert(id.clone(), PluginState::Discovered);
        self.categories.entry(plugin_type).or_default().push(id);
        Ok(())
    }

    /// Get plugin manifest by ID
    pub fn get(&self, id: &str) -> Option<&PluginManifest> {
        self.plugins.get(id)
    }

    /// Get plugin state
    pub fn get_state(&self, id: &str) -> Option<&PluginState> {
        self.states.get(id)
    }

    /// Update plugin state
    pub fn set_state(&mut self, id: &str, state: PluginState) -> Result<()> {
        if self.plugins.contains_key(id) {
            self.states.insert(id.to_string(), state);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Plugin not found: {}", id))
        }
    }

    /// Get all plugins of a specific type
    pub fn by_type(&self, plugin_type: &PluginType) -> Vec<&PluginManifest> {
        self.categories.get(plugin_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.plugins.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all registered plugins
    pub fn all(&self) -> Vec<&PluginManifest> {
        self.plugins.values().collect()
    }

    /// Get plugins by state
    pub fn by_state(&self, state: &PluginState) -> Vec<&PluginManifest> {
        self.states.iter()
            .filter(|(_, s)| **s == *state)
            .filter_map(|(id, _)| self.plugins.get(id))
            .collect()
    }

    /// Check if plugin exists
    pub fn has(&self, id: &str) -> bool {
        self.plugins.contains_key(id)
    }

    /// Remove a plugin
    pub fn remove(&mut self, id: &str) -> Result<()> {
        if let Some(manifest) = self.plugins.remove(id) {
            self.states.remove(id);
            if let Some(ids) = self.categories.get_mut(&manifest.plugin_type) {
                ids.retain(|i| i != id);
            }
            info!("Removed plugin: {}", id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Plugin not found: {}", id))
        }
    }
}
