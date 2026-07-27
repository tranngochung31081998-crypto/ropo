// 4-Tier Composite Routing System
// Inspired by OmniRoute's CompositeTiers pattern
// Tier 1: Subscription → Tier 2: API Key → Tier 3: Cheap → Tier 4: Free

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use tracing::{info, debug};

/// CompositeTiers configuration - defines 4-tier fallback chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeTiers {
    pub default_tier: String,
    pub tiers: HashMap<String, TierConfig>,
}

/// Configuration for a single tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierConfig {
    pub provider_id: String,
    pub model: String,
    pub fallback_tier: Option<String>,
    #[serde(default)]
    pub quota_limit: Option<u64>,
    #[serde(default)]
    pub cost_limit: Option<f64>,
    #[serde(default)]
    pub weight: u32,
}

impl CompositeTiers {
    /// Create default 4-tier configuration
    pub fn default_four_tier() -> Self {
        let mut tiers = HashMap::new();

        // Tier 1: Subscription (highest quality)
        tiers.insert("subscription".to_string(), TierConfig {
            provider_id: "openai".to_string(),
            model: "gpt-4o".to_string(),
            fallback_tier: Some("api-key".to_string()),
            quota_limit: None,
            cost_limit: Some(10.0), // Max $10 for premium
            weight: 100,
        });

        // Tier 2: API Key (pay-per-use)
        tiers.insert("api-key".to_string(), TierConfig {
            provider_id: "anthropic".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            fallback_tier: Some("cheap".to_string()),
            quota_limit: None,
            cost_limit: Some(5.0), // Max $5 for pay-per-use
            weight: 75,
        });

        // Tier 3: Cheap (budget-friendly)
        tiers.insert("cheap".to_string(), TierConfig {
            provider_id: "openai".to_string(), // Using gpt-4o-mini as cheap option
            model: "gpt-4o-mini".to_string(),
            fallback_tier: Some("free".to_string()),
            quota_limit: None,
            cost_limit: Some(1.0), // Max $1 for cheap tier
            weight: 50,
        });

        // Tier 4: Free (always available)
        tiers.insert("free".to_string(), TierConfig {
            provider_id: "ollama".to_string(),
            model: "llama3.3".to_string(),
            fallback_tier: None, // No further fallback
            quota_limit: None,
            cost_limit: Some(0.0), // Free tier
            weight: 25,
        });

        Self {
            default_tier: "subscription".to_string(),
            tiers,
        }
    }

    /// Validate tier configuration
    pub fn validate(&self) -> Result<()> {
        // 1. Check default_tier exists
        if !self.tiers.contains_key(&self.default_tier) {
            return Err(anyhow!("default_tier '{}' not found in tiers", self.default_tier));
        }

        // 2. Check all fallback_tier references exist and no self-references
        for (name, config) in &self.tiers {
            if let Some(ref fallback) = config.fallback_tier {
                if fallback == name {
                    return Err(anyhow!("Tier '{}' cannot fallback to itself", name));
                }
                if !self.tiers.contains_key(fallback) {
                    return Err(anyhow!("fallback_tier '{}' not found in tiers", fallback));
                }
            }
        }

        // 3. Detect cycles in fallback chain
        self.detect_cycles()?;

        debug!("✅ CompositeTiers validation passed");
        Ok(())
    }

    /// Detect circular dependencies in fallback chain
    fn detect_cycles(&self) -> Result<()> {
        for tier_name in self.tiers.keys() {
            let mut visited = HashSet::new();
            let mut current = Some(tier_name.clone());

            while let Some(name) = current {
                if !visited.insert(name.clone()) {
                    return Err(anyhow!("Circular fallback detected at tier '{}'", name));
                }

                current = self.tiers.get(&name)
                    .and_then(|cfg| cfg.fallback_tier.clone());
            }
        }

        Ok(())
    }

    /// Get complete fallback chain starting from default tier
    pub fn get_fallback_chain(&self) -> Vec<String> {
        let mut chain = vec![self.default_tier.clone()];
        let mut current = self.default_tier.clone();

        while let Some(config) = self.tiers.get(&current) {
            if let Some(ref fallback) = config.fallback_tier {
                chain.push(fallback.clone());
                current = fallback.clone();
            } else {
                break;
            }
        }

        info!("Fallback chain: {:?}", chain);
        chain
    }

    /// Get tier config by name
    pub fn get_tier(&self, name: &str) -> Option<&TierConfig> {
        self.tiers.get(name)
    }

    /// Get next tier in fallback chain
    pub fn get_next_tier(&self, current_tier: &str) -> Option<String> {
        self.tiers.get(current_tier)
            .and_then(|cfg| cfg.fallback_tier.clone())
    }

    /// Create from a map of tiers with a default tier name
    pub fn from_map(default_tier: &str, tiers: HashMap<String, TierConfig>) -> Self {
        Self {
            default_tier: default_tier.to_string(),
            tiers,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_four_tier_valid() {
        let tiers = CompositeTiers::default_four_tier();
        assert!(tiers.validate().is_ok());
    }

    #[test]
    fn test_fallback_chain() {
        let tiers = CompositeTiers::default_four_tier();
        let chain = tiers.get_fallback_chain();
        assert_eq!(chain, vec!["subscription", "api-key", "cheap", "free"]);
    }

    #[test]
    fn test_cycle_detection() {
        let mut tiers = HashMap::new();
        tiers.insert("tier1".to_string(), TierConfig {
            provider_id: "p1".to_string(),
            model: "m1".to_string(),
            fallback_tier: Some("tier2".to_string()),
            quota_limit: None,
            cost_limit: None,
            weight: 100,
        });
        tiers.insert("tier2".to_string(), TierConfig {
            provider_id: "p2".to_string(),
            model: "m2".to_string(),
            fallback_tier: Some("tier1".to_string()), // Cycle!
            quota_limit: None,
            cost_limit: None,
            weight: 50,
        });

        let composite = CompositeTiers {
            default_tier: "tier1".to_string(),
            tiers,
        };

        assert!(composite.validate().is_err());
    }

    #[test]
    fn test_self_reference() {
        let mut tiers = HashMap::new();
        tiers.insert("tier1".to_string(), TierConfig {
            provider_id: "p1".to_string(),
            model: "m1".to_string(),
            fallback_tier: Some("tier1".to_string()), // Self-reference!
            quota_limit: None,
            cost_limit: None,
            weight: 100,
        });

        let composite = CompositeTiers {
            default_tier: "tier1".to_string(),
            tiers,
        };

        assert!(composite.validate().is_err());
    }

    #[test]
    fn test_missing_fallback_tier() {
        let mut tiers = HashMap::new();
        tiers.insert("tier1".to_string(), TierConfig {
            provider_id: "p1".to_string(),
            model: "m1".to_string(),
            fallback_tier: Some("nonexistent".to_string()), // Missing!
            quota_limit: None,
            cost_limit: None,
            weight: 100,
        });

        let composite = CompositeTiers {
            default_tier: "tier1".to_string(),
            tiers,
        };

        assert!(composite.validate().is_err());
    }
}
