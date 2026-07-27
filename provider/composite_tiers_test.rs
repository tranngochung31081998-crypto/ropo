// Test file for 4-tier routing system

#[cfg(test)]
mod integration_tests {
    use crate::provider::{ProviderRouter, RouterConfig, CompositeTiers};

    #[tokio::test]
    async fn test_four_tier_routing_enabled() {
        let mut router = ProviderRouter::new(RouterConfig::default());
        
        // Enable 4-tier routing
        let result = router.enable_four_tier_routing();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_four_tier_validation() {
        let tiers = CompositeTiers::default_four_tier();
        assert!(tiers.validate().is_ok());

        let chain = tiers.get_fallback_chain();
        assert_eq!(chain.len(), 4);
        assert_eq!(chain[0], "subscription");
        assert_eq!(chain[1], "api-key");
        assert_eq!(chain[2], "cheap");
        assert_eq!(chain[3], "free");
    }

    #[tokio::test]
    async fn test_tier_navigation() {
        let tiers = CompositeTiers::default_four_tier();
        
        // Test tier retrieval
        assert!(tiers.get_tier("subscription").is_some());
        assert!(tiers.get_tier("api-key").is_some());
        assert!(tiers.get_tier("cheap").is_some());
        assert!(tiers.get_tier("free").is_some());
        assert!(tiers.get_tier("nonexistent").is_none());

        // Test next tier navigation
        assert_eq!(tiers.get_next_tier("subscription"), Some("api-key".to_string()));
        assert_eq!(tiers.get_next_tier("api-key"), Some("cheap".to_string()));
        assert_eq!(tiers.get_next_tier("cheap"), Some("free".to_string()));
        assert_eq!(tiers.get_next_tier("free"), None);
    }
}
