use anyhow::Result;
use std::collections::HashMap;
use tracing::{info, warn, debug};

use super::{
    LLMProvider, Message, LLMResponse, ToolDefinition, TokenUsage,
    OpenAIProvider, AnthropicProvider, OllamaProvider,
};
use super::composite_tiers::{CompositeTiers, TierConfig};

/// Provider Router - quản lý multi-provider với 4-tier fallback system
/// Inspired by OmniRoute's CompositeTiers pattern
/// Tier 1: Subscription → Tier 2: API Key → Tier 3: Cheap → Tier 4: Free
pub struct ProviderRouter {
    providers: HashMap<String, Box<dyn LLMProvider>>,
    primary: String,
    fallback_chain: Vec<String>,
    token_tracker: TokenTracker,
    config: RouterConfig,
    /// Optional 4-tier routing configuration
    composite_tiers: Option<CompositeTiers>,
}

#[derive(Debug, Clone)]
pub struct RouterConfig {
    pub max_retries: u32,
    pub timeout_seconds: u64,
    pub enable_fallback: bool,
    pub enable_token_tracking: bool,
    pub use_composite_tiers: bool,  // Enable 4-tier routing
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            timeout_seconds: 120,
            enable_fallback: true,
            enable_token_tracking: true,
            use_composite_tiers: false,  // Disabled by default for backward compatibility
        }
    }
}

impl ProviderRouter {
    pub fn new(config: RouterConfig) -> Self {
        let mut providers: HashMap<String, Box<dyn LLMProvider>> = HashMap::new();
        
        // Đăng ký providers mặc định
        providers.insert("openai".into(), Box::new(OpenAIProvider::new()));
        providers.insert("anthropic".into(), Box::new(AnthropicProvider::new()));
        providers.insert("ollama".into(), Box::new(OllamaProvider::new()));

        Self {
            providers,
            primary: "openai".into(),
            fallback_chain: vec!["anthropic".into(), "ollama".into()],
            token_tracker: TokenTracker::new(),
            config,
            composite_tiers: None,
        }
    }

    /// Enable 4-tier routing with default configuration
    pub fn enable_four_tier_routing(&mut self) -> Result<()> {
        let tiers = CompositeTiers::default_four_tier();
        tiers.validate()?;
        
        self.composite_tiers = Some(tiers);
        self.config.use_composite_tiers = true;
        
        info!("✅ 4-tier routing enabled: Subscription → API Key → Cheap → Free");
        Ok(())
    }

    /// Set custom composite tiers configuration
    pub fn set_composite_tiers(&mut self, tiers: CompositeTiers) -> Result<()> {
        tiers.validate()?;
        self.composite_tiers = Some(tiers);
        self.config.use_composite_tiers = true;
        info!("✅ Custom composite tiers configured");
        Ok(())
    }

    /// Đăng ký provider tùy chỉnh
    pub fn register_provider(&mut self, name: &str, provider: Box<dyn LLMProvider>) {
        self.providers.insert(name.to_string(), provider);
    }

    /// Set primary provider
    pub fn set_primary(&mut self, name: &str) -> Result<()> {
        if self.providers.contains_key(name) {
            self.primary = name.to_string();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Provider '{}' not found", name))
        }
    }

    /// Set fallback chain
    pub fn set_fallback_chain(&mut self, chain: Vec<String>) {
        self.fallback_chain = chain;
    }

    /// Main chat method - với 4-tier fallback or traditional fallback chain
    /// Pattern: Thử primary → fallback 1 → fallback 2 → error
    pub async fn chat(
        &mut self,
        messages: &[Message],
        model: &str,
        tools: &[ToolDefinition],
    ) -> Result<LLMResponse> {
        // Use 4-tier routing if enabled
        if self.config.use_composite_tiers && self.composite_tiers.is_some() {
            // Clone tiers to avoid borrow issues
            let tiers = self.composite_tiers.as_ref().unwrap().clone();
            return self.route_with_tiers(messages, model, tools, &tiers).await;
        }

        // Traditional fallback routing
        self.route_with_fallback_chain(messages, model, tools).await
    }

    /// Route using 4-tier composite system
    async fn route_with_tiers(
        &mut self,
        messages: &[Message],
        _model: &str,
        tools: &[ToolDefinition],
        tiers: &CompositeTiers,
    ) -> Result<LLMResponse> {
        let chain = tiers.get_fallback_chain();
        info!("🔄 Starting 4-tier routing: {:?}", chain);

        for (idx, tier_name) in chain.iter().enumerate() {
            let tier_config = tiers.get_tier(tier_name)
                .ok_or_else(|| anyhow::anyhow!("Tier '{}' not found", tier_name))?;

            info!("Tier {}/{}: Trying '{}' (provider: {}, model: {})",
                idx + 1, chain.len(), tier_name, tier_config.provider_id, tier_config.model);

            match self.try_tier(tier_name, tier_config, messages, tools).await {
                Ok(response) => {
                    info!("✅ Tier '{}' succeeded", tier_name);
                    self.token_tracker.track(&tier_config.provider_id, &response.usage);
                    return Ok(response);
                }
                Err(e) => {
                    warn!("⚠️ Tier '{}' failed: {}. Falling back...", tier_name, e);
                    continue;
                }
            }
        }

        Err(anyhow::anyhow!("All 4 tiers exhausted"))
    }

    /// Try a specific tier
    async fn try_tier(
        &self,
        tier_name: &str,
        tier_config: &TierConfig,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<LLMResponse> {
        debug!("Attempting tier '{}': provider={}, model={}",
            tier_name, tier_config.provider_id, tier_config.model);

        self.try_provider(
            &tier_config.provider_id,
            messages,
            &tier_config.model,
            tools
        ).await
    }

    /// Traditional fallback chain routing (backward compatibility)
    async fn route_with_fallback_chain(
        &mut self,
        messages: &[Message],
        model: &str,
        tools: &[ToolDefinition],
    ) -> Result<LLMResponse> {
        // Thử primary provider
        info!("Trying primary provider: {}", self.primary);
        match self.try_provider(&self.primary, messages, model, tools).await {
            Ok(response) => {
                info!("Primary provider {} succeeded", self.primary);
                self.token_tracker.track(&self.primary, &response.usage);
                return Ok(response);
            }
            Err(e) => {
                warn!("Primary provider failed: {}. Trying fallbacks...", e);
            }
        }

        // Fallback chain
        if self.config.enable_fallback {
            for provider_name in &self.fallback_chain {
                if provider_name == &self.primary {
                    continue; // Bỏ qua primary đã thử
                }
                info!("Trying fallback provider: {}", provider_name);
                match self.try_provider(provider_name, messages, model, tools).await {
                    Ok(response) => {
                        info!("Fallback provider {} succeeded", provider_name);
                        self.token_tracker.track(provider_name, &response.usage);
                        return Ok(response);
                    }
                    Err(e) => {
                        warn!("Fallback {} failed: {}", provider_name, e);
                        continue;
                    }
                }
            }
        }

        Err(anyhow::anyhow!("All providers failed"))
    }

    /// Try a specific provider with retries
    pub async fn chat_with_provider(
        &self,
        name: &str,
        messages: &[Message],
        model: &str,
        tools: &[ToolDefinition],
    ) -> Result<LLMResponse> {
        self.try_provider(name, messages, model, tools).await
    }

    /// Try a specific provider with retries (internal)
    async fn try_provider(
        &self,
        name: &str,
        messages: &[Message],
        model: &str,
        tools: &[ToolDefinition],
    ) -> Result<LLMResponse> {
        let provider = self.providers.get(name)
            .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found", name))?;

        let mut last_error = None;
        for attempt in 1..=self.config.max_retries {
            match provider.chat(messages, model, tools).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    warn!("Provider {} attempt {}/{} failed: {}", name, attempt, self.config.max_retries, e);
                    last_error = Some(e);
                    if attempt < self.config.max_retries {
                        tokio::time::sleep(tokio::time::Duration::from_secs(
                            2u64.pow(attempt) // Exponential backoff
                        )).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Provider '{}' failed", name)))
    }

    pub fn get_token_usage(&self) -> &TokenTracker {
        &self.token_tracker
    }

    pub fn track_token_usage(&mut self, provider: &str, usage: &TokenUsage) {
        self.token_tracker.track(provider, usage);
    }

    pub fn get_provider_names(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}

/// Token usage tracker - inspired by 9router
#[derive(Debug, Clone)]
pub struct TokenTracker {
    total_prompt: u64,
    total_completion: u64,
    total_cost: f64,
    provider_usage: HashMap<String, ProviderUsage>,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderUsage {
    pub calls: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cost: f64,
}

impl TokenTracker {
    pub fn new() -> Self {
        Self {
            total_prompt: 0,
            total_completion: 0,
            total_cost: 0.0,
            provider_usage: HashMap::new(),
        }
    }

    pub fn track(&mut self, provider: &str, usage: &TokenUsage) {
        self.total_prompt += usage.prompt_tokens as u64;
        self.total_completion += usage.completion_tokens as u64;
        
        let entry = self.provider_usage.entry(provider.to_string())
            .or_insert_with(ProviderUsage::default);
        entry.calls += 1;
        entry.prompt_tokens += usage.prompt_tokens as u64;
        entry.completion_tokens += usage.completion_tokens as u64;
    }

    pub fn summary(&self) -> String {
        format!(
            "Tokens: {} prompt + {} completion = {} total | Cost: ${:.4}",
            self.total_prompt,
            self.total_completion,
            self.total_prompt + self.total_completion,
            self.total_cost
        )
    }

    pub fn provider_breakdown(&self) -> Vec<String> {
        self.provider_usage.iter().map(|(name, usage)| {
            format!(
                "{}: {} calls, {} tokens, ${:.4}",
                name, usage.calls,
                usage.prompt_tokens + usage.completion_tokens,
                usage.cost
            )
        }).collect()
    }
}
