use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Context Compressor - manages token budgets and compresses context when needed
/// 
/// Inspired by Hermes Agent's context compression mechanism:
/// Auto-compress when approaching context limit to prevent truncation
pub struct ContextCompressor {
    /// Token threshold that triggers compression (e.g. 150k out of 200k)
    pub target_tokens: usize,
    /// Hard limit (e.g. 200k)
    max_tokens: usize,
}

impl ContextCompressor {
    pub fn new(target_tokens: usize) -> Self {
        Self {
            target_tokens,
            max_tokens: target_tokens + (target_tokens / 3), // ~1.33x buffer
        }
    }

    /// Check if compression is needed based on current token count
    pub fn check_budget(&self, current_tokens: usize) -> bool {
        current_tokens >= self.target_tokens
    }

    /// Calculate how much to compress
    pub fn compression_ratio(&self, current_tokens: usize) -> f64 {
        if current_tokens <= self.target_tokens {
            return 1.0; // No compression needed
        }
        self.target_tokens as f64 / current_tokens as f64
    }

    /// Compress messages intelligently
    /// - Truncates old messages when approaching limit
    /// - Preserves system prompt and recent context
    /// - Summarizes middle content
    pub async fn compress(&self, messages: &[String], max_tokens: usize) -> Result<Vec<String>> {
        if messages.len() <= 3 {
            // Too few messages to compress meaningfully
            return Ok(messages.to_vec());
        }

        let total_chars: usize = messages.iter().map(|m| m.len()).sum();
        let avg_chars_per_token = 4; // Rough estimate
        let estimated_tokens = total_chars / avg_chars_per_token;

        if estimated_tokens <= max_tokens {
            return Ok(messages.to_vec());
        }

        // Compression strategy:
        // 1. Keep first 2 messages (system + initial context)
        // 2. Keep last 3 messages (recent conversation)
        // 3. Summarize the middle into bullet points
        let keep_first = 2usize.min(messages.len());
        let keep_last = 3usize.min(messages.len().saturating_sub(keep_first));

        let mut compressed = Vec::new();

        // First messages (system prompt + user's first input)
        for msg in messages.iter().take(keep_first) {
            compressed.push(msg.clone());
        }

        // Middle section - summarize
        let middle: Vec<String> = messages[keep_first..messages.len().saturating_sub(keep_last)].to_vec();
        if !middle.is_empty() {
            let summary = self.summarize_middle(&middle);
            compressed.push(summary);
        }

        // Last messages (most recent context)
        for msg in messages.iter().skip(messages.len().saturating_sub(keep_last)) {
            compressed.push(msg.clone());
        }

        Ok(compressed)
    }

    /// Create a summarized version of middle messages
    fn summarize_middle(&self, messages: &[String]) -> String {
        let sections: Vec<String> = messages
            .iter()
            .enumerate()
            .map(|(i, msg)| {
                let first_line = msg.lines().next().unwrap_or(msg);
                let preview = if first_line.len() > 60 {
                    format!("{}...", &first_line[..60])
                } else {
                    first_line.to_string()
                };
                format!("[section {}] {}", i + 1, preview)
            })
            .collect();

        format!(
            "[CONTEXT COMPRESSED] The following {} messages have been summarized for token efficiency:\n{}\n[END COMPRESSED SECTION]",
            messages.len(),
            sections.join("\n")
        )
    }

    /// Get current budget status as a human-readable report
    pub fn budget_report(&self, current_tokens: usize) -> BudgetReport {
        let usage_pct = (current_tokens as f64 / self.max_tokens as f64) * 100.0;
        BudgetReport {
            current_tokens,
            max_tokens: self.max_tokens,
            target_tokens: self.target_tokens,
            usage_percent: usage_pct,
            needs_compression: current_tokens >= self.target_tokens,
            remaining_tokens: self.max_tokens.saturating_sub(current_tokens),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetReport {
    pub current_tokens: usize,
    pub max_tokens: usize,
    pub target_tokens: usize,
    pub usage_percent: f64,
    pub needs_compression: bool,
    pub remaining_tokens: usize,
}
