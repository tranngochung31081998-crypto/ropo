use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::info;

/// Plugin Isolation - security sandbox cho plugins
/// Inspired by OpenClaw's security model và Hermes Agent's approval system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationPolicy {
    pub allowed_paths: Vec<String>,
    pub allowed_commands: Vec<String>,
    pub allowed_network: Vec<String>,
    pub max_memory_mb: u64,
    pub max_cpu_percent: u8,
    pub sandbox_level: SandboxLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SandboxLevel {
    None,       // No isolation
    Low,        // Basic path/command restrictions
    Medium,     // Network + resource limits
    High,       // Full sandbox (container)
    Custom,     // Custom policy
}

/// Plugin Sandbox - thực thi plugin trong môi trường cách ly
pub struct PluginSandbox {
    policies: Vec<(String, IsolationPolicy)>,
    revoked_permissions: HashSet<String>,
}

impl PluginSandbox {
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
            revoked_permissions: HashSet::new(),
        }
    }

    /// Assign isolation policy to a plugin
    pub fn assign_policy(&mut self, plugin_id: &str, policy: IsolationPolicy) -> Result<()> {
        info!("Assigning {:?} sandbox to plugin: {}", policy.sandbox_level, plugin_id);
        self.policies.push((plugin_id.to_string(), policy));
        Ok(())
    }

    /// Check if a path is allowed for a plugin
    pub fn check_path(&self, plugin_id: &str, path: &str) -> bool {
        if let Some((_, policy)) = self.policies.iter().find(|(id, _)| id == plugin_id) {
            policy.allowed_paths.iter().any(|p| path.starts_with(p))
        } else {
            false
        }
    }

    /// Check if a command is allowed
    pub fn check_command(&self, plugin_id: &str, command: &str) -> bool {
        if let Some((_, policy)) = self.policies.iter().find(|(id, _)| id == plugin_id) {
            policy.allowed_commands.iter().any(|c| command.starts_with(c))
        } else {
            false
        }
    }

    /// Revoke a permission from a plugin
    pub fn revoke_permission(&mut self, permission: &str) {
        self.revoked_permissions.insert(permission.to_string());
        info!("Revoked permission: {}", permission);
    }

    /// Get policy for a plugin
    pub fn get_policy(&self, plugin_id: &str) -> Option<&IsolationPolicy> {
        self.policies.iter()
            .find(|(id, _)| id == plugin_id)
            .map(|(_, policy)| policy)
    }
}
