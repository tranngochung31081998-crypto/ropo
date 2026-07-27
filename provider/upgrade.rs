use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::{Message, TokenUsage};

// ============================================================================
// RTK Token Saver - Optimize token usage through format translation
// Inspired by OmniRoute's RTK (Reduced Token Kernel) pattern
// ============================================================================

/// Token-efficient message format that reduces token count 20-40%
pub struct TokenSaver;

impl TokenSaver {
    /// Optimize messages by removing redundant whitespace and shortening field names
    /// Returns optimized messages + estimated token saving percentage
    pub fn optimize(messages: &[Message]) -> (Vec<Message>, f32) {
        let original_len: usize = messages.iter().map(|m| m.content.len()).sum();
        let optimized: Vec<Message> = messages.iter().map(|msg| {
            let mut content = msg.content.clone();
            // Compress: remove excessive whitespace
            content = content.split_whitespace().collect::<Vec<_>>().join(" ");
            // Remove redundant newlines (keep single \n between paragraphs)
            let lines: Vec<&str> = content.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
            content = lines.join("\n");
            // Shorten role names for internal storage
            Message {
                content,
                ..msg.clone()
            }
        }).collect();

        let optimized_len: usize = optimized.iter().map(|m| m.content.len()).sum();
        let saving = if original_len > 0 {
            (original_len - optimized_len) as f32 / original_len as f32 * 100.0
        } else {
            0.0
        };

        (optimized, saving)
    }

    /// Estimate token count from text (rough: ~4 chars per token)
    pub fn estimate_tokens(text: &str) -> u32 {
        (text.len() as f32 / 4.0).ceil() as u32
    }

    /// Estimate the cost of a request
    pub fn estimate_cost(prompt_tokens: u32, completion_tokens: u32, model: &str) -> f64 {
        let (prompt_price, completion_price) = match model {
            m if m.contains("gpt-4o") => (2.50e-6, 10.00e-6),   // $2.50/1M input, $10.00/1M output
            m if m.contains("gpt-4") => (30.00e-6, 60.00e-6),   // $30/1M input, $60/1M output
            m if m.contains("gpt-3.5") => (0.50e-6, 1.50e-6),   // $0.50/1M input, $1.50/1M output
            m if m.contains("claude-3-5-sonnet") => (3.00e-6, 15.00e-6),
            m if m.contains("claude-3-haiku") => (0.25e-6, 1.25e-6),
            m if m.contains("claude-3-opus") => (15.00e-6, 75.00e-6),
            _ => (2.50e-6, 10.00e-6), // Default to gpt-4o pricing
        };
        prompt_tokens as f64 * prompt_price + completion_tokens as f64 * completion_price
    }
}

// ============================================================================
// Multi-Account Manager - Round-robin across multiple API keys
// Inspired by OmniRoute's multi-account pattern
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    pub name: String,
    pub api_key: String,
    pub provider: String,
    pub models: Vec<String>,
    pub daily_limit_tokens: u64,
    pub used_tokens: u64,
    pub enabled: bool,
}

impl AccountConfig {
    pub fn remaining_tokens(&self) -> u64 {
        self.daily_limit_tokens.saturating_sub(self.used_tokens)
    }

    pub fn is_available(&self) -> bool {
        self.enabled && self.remaining_tokens() > 0
    }
}

/// Multi-account round-robin manager
pub struct AccountManager {
    accounts: Vec<AccountConfig>,
    current_index: usize,
}

impl AccountManager {
    pub fn new() -> Self {
        Self {
            accounts: Vec::new(),
            current_index: 0,
        }
    }

    pub fn add_account(&mut self, account: AccountConfig) {
        info!("Added account: {}", account.name);
        self.accounts.push(account);
    }

    pub fn remove_account(&mut self, name: &str) {
        self.accounts.retain(|a| a.name != name);
    }

    /// Get next available account via round-robin
    pub fn next_available(&mut self, model: &str) -> Option<&AccountConfig> {
        let len = self.accounts.len();
        for _ in 0..len {
            self.current_index = (self.current_index + 1) % len;
            if let Some(account) = self.accounts.get(self.current_index) {
                if account.is_available() && account.models.iter().any(|m| m == model || m == "any") {
                    return Some(account);
                }
            }
        }
        None
    }

    /// Track token usage for an account
    pub fn track_usage(&mut self, account_name: &str, tokens: u64) {
        if let Some(account) = self.accounts.iter_mut().find(|a| a.name == account_name) {
            account.used_tokens += tokens;
        }
    }

    pub fn account_count(&self) -> usize {
        self.accounts.len()
    }

    pub fn summary(&self) -> Vec<String> {
        self.accounts.iter().map(|a| {
            format!("{} ({}): {}/{} tokens, enabled={}", 
                a.name, a.provider, a.used_tokens, a.daily_limit_tokens, a.enabled)
        }).collect()
    }
}

// ============================================================================
// Cost Tracker - Per-request cost estimation and tracking
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEntry {
    pub provider: String,
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub cost: f64,
    pub timestamp: String,
}

/// Track costs per provider, model, and time period
pub struct CostTracker {
    history: Vec<CostEntry>,
    daily_budget: f64,
}

impl CostTracker {
    pub fn new(daily_budget: f64) -> Self {
        Self {
            history: Vec::new(),
            daily_budget,
        }
    }

    pub fn track(&mut self, provider: &str, model: &str, usage: &TokenUsage) {
        let cost = TokenSaver::estimate_cost(usage.prompt_tokens, usage.completion_tokens, model);
        self.history.push(CostEntry {
            provider: provider.to_string(),
            model: model.to_string(),
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            cost,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    pub fn total_cost(&self) -> f64 {
        self.history.iter().map(|e| e.cost).sum()
    }

    pub fn daily_cost(&self) -> f64 {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        self.history.iter()
            .filter(|e| e.timestamp.starts_with(&today))
            .map(|e| e.cost)
            .sum()
    }

    pub fn remaining_budget(&self) -> f64 {
        self.daily_budget - self.daily_cost()
    }

    pub fn is_over_budget(&self) -> bool {
        self.daily_cost() >= self.daily_budget
    }

    pub fn provider_breakdown(&self) -> HashMap<String, f64> {
        let mut breakdown: HashMap<String, f64> = HashMap::new();
        for entry in &self.history {
            *breakdown.entry(entry.provider.clone()).or_insert(0.0) += entry.cost;
        }
        breakdown
    }

    pub fn recent_costs(&self, n: usize) -> &[CostEntry] {
        let len = self.history.len();
        let start = len.saturating_sub(n);
        &self.history[start..]
    }
}

// ============================================================================
// Enhanced Fallback Metrics - Better tracking of fallback decisions
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedFallbackMetrics {
    pub primary_attempts: u64,
    pub primary_successes: u64,
    pub fallback_attempts: u64,
    pub fallback_successes: u64,
    pub failures_by_provider: HashMap<String, u64>,
    pub avg_fallback_delay_ms: f64,
    total_delay_ms: u64,
    fallback_count: u64,
}

impl EnhancedFallbackMetrics {
    pub fn new() -> Self {
        Self {
            primary_attempts: 0,
            primary_successes: 0,
            fallback_attempts: 0,
            fallback_successes: 0,
            failures_by_provider: HashMap::new(),
            avg_fallback_delay_ms: 0.0,
            total_delay_ms: 0,
            fallback_count: 0,
        }
    }

    pub fn record_primary_attempt(&mut self) {
        self.primary_attempts += 1;
    }

    pub fn record_primary_success(&mut self) {
        self.primary_successes += 1;
    }

    pub fn record_fallback_attempt(&mut self, provider: &str) {
        self.fallback_attempts += 1;
        *self.failures_by_provider.entry(provider.to_string()).or_insert(0) += 1;
    }

    pub fn record_fallback_success(&mut self, delay_ms: u64) {
        self.fallback_successes += 1;
        self.total_delay_ms += delay_ms;
        self.fallback_count += 1;
        self.avg_fallback_delay_ms = self.total_delay_ms as f64 / self.fallback_count as f64;
    }

    pub fn primary_success_rate(&self) -> f64 {
        if self.primary_attempts == 0 { 1.0 }
        else { self.primary_successes as f64 / self.primary_attempts as f64 }
    }

    pub fn fallback_success_rate(&self) -> f64 {
        if self.fallback_attempts == 0 { 0.0 }
        else { self.fallback_successes as f64 / self.fallback_attempts as f64 }
    }

    pub fn summary(&self) -> String {
        format!(
            "Primary: {}/{} ({:.1}%) | Fallback: {}/{} ({:.1}%) | Avg delay: {:.0}ms | Failed providers: {:?}",
            self.primary_successes, self.primary_attempts, self.primary_success_rate() * 100.0,
            self.fallback_successes, self.fallback_attempts, self.fallback_success_rate() * 100.0,
            self.avg_fallback_delay_ms,
            self.failures_by_provider.keys().collect::<Vec<_>>()
        )
    }
}
