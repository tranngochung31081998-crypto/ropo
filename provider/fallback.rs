use serde::{Deserialize, Serialize};

/// Fallback chain configuration - inspired by 9router
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfig {
    pub enabled: bool,
    pub chain: Vec<String>,
    pub max_retries_per_provider: u32,
    pub timeout_seconds: u64,
    pub exponential_backoff: bool,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            chain: vec![
                "openai".into(),
                "anthropic".into(),
                "ollama".into(),
            ],
            max_retries_per_provider: 3,
            timeout_seconds: 120,
            exponential_backoff: true,
        }
    }
}

/// Reason for fallback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallbackReason {
    RateLimited,
    Timeout,
    AuthError,
    ServerError,
    NetworkError,
    QuotaExceeded,
    ModelUnavailable,
    Other(String),
}

impl FallbackReason {
    pub fn is_retryable(&self) -> bool {
        matches!(self, 
            FallbackReason::RateLimited 
            | FallbackReason::Timeout 
            | FallbackReason::ServerError
            | FallbackReason::NetworkError
        )
    }

    pub fn description(&self) -> &str {
        match self {
            FallbackReason::RateLimited => "Rate limited",
            FallbackReason::Timeout => "Request timeout",
            FallbackReason::AuthError => "Authentication error",
            FallbackReason::ServerError => "Server error",
            FallbackReason::NetworkError => "Network error",
            FallbackReason::QuotaExceeded => "Quota exceeded",
            FallbackReason::ModelUnavailable => "Model unavailable",
            FallbackReason::Other(s) => s.as_str(),
        }
    }
}

/// Fallback event log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub from_provider: String,
    pub to_provider: String,
    pub reason: FallbackReason,
    pub attempt: u32,
    pub success: bool,
}

/// Fallback metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FallbackMetrics {
    pub total_fallbacks: u64,
    pub successful_fallbacks: u64,
    pub failed_fallbacks: u64,
    pub fallback_history: Vec<FallbackEvent>,
}

impl FallbackMetrics {
    pub fn record_fallback(&mut self, event: FallbackEvent) {
        self.total_fallbacks += 1;
        if event.success {
            self.successful_fallbacks += 1;
        } else {
            self.failed_fallbacks += 1;
        }
        self.fallback_history.push(event);
        
        // Giữ lịch sử tối đa 1000 events
        if self.fallback_history.len() > 1000 {
            self.fallback_history.remove(0);
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_fallbacks == 0 {
            return 1.0;
        }
        self.successful_fallbacks as f64 / self.total_fallbacks as f64
    }

    pub fn summary(&self) -> String {
        format!(
            "Fallbacks: {} total, {} successful ({:.1}%), {} failed",
            self.total_fallbacks,
            self.successful_fallbacks,
            self.success_rate() * 100.0,
            self.failed_fallbacks
        )
    }
}