use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Token usage và cost tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTracker {
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub total_cost: f64,
    pub provider_stats: HashMap<String, ProviderStats>,
    pub session_history: Vec<SessionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderStats {
    pub calls: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub provider: String,
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub cost: f64,
    pub duration_ms: u64,
}

impl TokenTracker {
    pub fn new() -> Self {
        Self {
            total_prompt_tokens: 0,
            total_completion_tokens: 0,
            total_cost: 0.0,
            provider_stats: HashMap::new(),
            session_history: Vec::new(),
        }
    }

    pub fn track(&mut self, provider: &str, model: &str, prompt_tokens: u32, completion_tokens: u32) {
        self.total_prompt_tokens += prompt_tokens as u64;
        self.total_completion_tokens += completion_tokens as u64;
        
        let cost = estimate_cost(provider, model, prompt_tokens, completion_tokens);
        self.total_cost += cost;

        let stats = self.provider_stats.entry(provider.to_string())
            .or_insert_with(ProviderStats::default);
        stats.calls += 1;
        stats.prompt_tokens += prompt_tokens as u64;
        stats.completion_tokens += completion_tokens as u64;
        stats.cost += cost;
    }

    pub fn summary(&self) -> String {
        format!(
            "📊 Token Usage:\n\
             Total: {} prompt + {} completion = {} tokens\n\
             Cost: ${:.4}\n\
             Providers: {}",
            self.total_prompt_tokens,
            self.total_completion_tokens,
            self.total_prompt_tokens + self.total_completion_tokens,
            self.total_cost,
            self.provider_stats.len()
        )
    }

    pub fn provider_breakdown(&self) -> Vec<String> {
        self.provider_stats.iter()
            .map(|(name, stats)| {
                format!(
                    "  {}: {} calls, {} tokens, ${:.4}",
                    name, stats.calls,
                    stats.prompt_tokens + stats.completion_tokens,
                    stats.cost
                )
            })
            .collect()
    }
}

/// Ước tính cost dựa trên provider và model
fn estimate_cost(provider: &str, model: &str, prompt: u32, completion: u32) -> f64 {
    let (prompt_rate, completion_rate) = match provider {
        "openai" => match model {
            m if m.contains("gpt-4o") => (2.50, 10.00),
            m if m.contains("gpt-4") => (30.00, 60.00),
            m if m.contains("gpt-3.5") => (0.50, 1.50),
            _ => (1.00, 2.00),
        },
        "anthropic" => match model {
            m if m.contains("claude-3-opus") => (15.00, 75.00),
            m if m.contains("claude-3-sonnet") => (3.00, 15.00),
            m if m.contains("claude-3-haiku") => (0.25, 1.25),
            _ => (3.00, 15.00),
        },
        "ollama" => (0.0, 0.0), // Local models are free
        _ => (1.00, 2.00),
    };

    let prompt_cost = (prompt as f64 / 1_000_000.0) * prompt_rate;
    let completion_cost = (completion as f64 / 1_000_000.0) * completion_rate;
    
    prompt_cost + completion_cost
}
