//! Harness-powered memory consolidation
//! Uses free models (Sixth AI, Blackbox) for LLM-based summarization

use anyhow::Result;
use crate::provider::{Message, LLMResponse};

/// Harness Consolidator - Sử dụng free models để tóm tắt và trích xuất
pub struct HarnessConsolidator {
    /// Callback để gọi harness chat (Sixth/Blackbox)
    harness_fn: Box<dyn Fn(Vec<Message>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LLMResponse>> + Send>> + Send + Sync>,
}

impl HarnessConsolidator {
    /// Create with harness chat callback
    pub fn new<F, Fut>(harness_fn: F) -> Self
    where
        F: Fn(Vec<Message>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<LLMResponse>> + Send + 'static,
    {
        Self {
            harness_fn: Box::new(move |msgs| Box::pin(harness_fn(msgs))),
        }
    }

    /// Extract facts using LLM (Sixth AI free)
    pub async fn extract_facts(&self, content: &str) -> Result<Vec<String>> {
        let prompt = format!(
            "Extract key facts from this memory entry (return as bullet points, max 5):\n\n{}",
            content
        );

        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }];

        let response = (self.harness_fn)(messages).await?;
        
        let facts: Vec<String> = response.content
            .unwrap_or_default()
            .lines()
            .filter(|l| {
                let trimmed = l.trim();
                trimmed.starts_with('-') || trimmed.starts_with('•') || trimmed.starts_with('*')
            })
            .map(|l| {
                l.trim_start_matches('-')
                    .trim_start_matches('•')
                    .trim_start_matches('*')
                    .trim()
                    .to_string()
            })
            .filter(|s| s.len() > 10)
            .take(5)
            .collect();

        if facts.is_empty() {
            // Fallback: first sentence
            Ok(vec![content.lines().next().unwrap_or("").to_string()])
        } else {
            Ok(facts)
        }
    }

    /// Extract concepts using LLM (Blackbox free)
    pub async fn extract_concepts(&self, content: &str) -> Result<Vec<String>> {
        let prompt = format!(
            "Extract key technical concepts/terms from this text (max 10 keywords):\n\n{}",
            content
        );

        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }];

        let response = (self.harness_fn)(messages).await?;
        
        let concepts: Vec<String> = response.content
            .unwrap_or_default()
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != ':'))
            .filter(|w| !w.is_empty())
            .map(|w| w.to_string())
            .take(10)
            .collect();

        Ok(concepts)
    }

    /// Detect workflow patterns using LLM
    pub async fn extract_workflow(&self, content: &str) -> Result<Option<String>> {
        let prompt = format!(
            "Is this a workflow or step-by-step procedure? If yes, summarize it. If no, return 'NONE'.\n\n{}",
            content
        );

        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }];

        let response = (self.harness_fn)(messages).await?;
        
        let result = response.content.unwrap_or_default();
        
        if result.trim().to_uppercase().starts_with("NONE") {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }

    /// Summarize multiple entries into one (episodic → semantic)
    pub async fn summarize_entries(&self, entries: &[String]) -> Result<String> {
        let combined = entries.join("\n---\n");
        let prompt = format!(
            "Summarize these related memory entries into one concise paragraph:\n\n{}",
            combined
        );

        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }];

        let response = (self.harness_fn)(messages).await?;
        
        Ok(response.content.unwrap_or_default())
    }

    /// Detect duplicates using LLM semantic similarity
    pub async fn are_similar(&self, content1: &str, content2: &str) -> Result<bool> {
        let prompt = format!(
            "Are these two memory entries semantically similar? Answer YES or NO only.\n\nEntry 1:\n{}\n\nEntry 2:\n{}",
            content1, content2
        );

        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }];

        let response = (self.harness_fn)(messages).await?;
        
        let answer = response.content.unwrap_or_default().trim().to_uppercase();
        Ok(answer.starts_with("YES"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn mock_harness(messages: Vec<Message>) -> Result<LLMResponse> {
        let content = messages.first()
            .and_then(|m| Some(m.content.clone()))
            .unwrap_or_default();
        
        if content.contains("Extract key facts") {
            Ok(LLMResponse {
                content: Some("- Fact 1\n- Fact 2\n- Fact 3".to_string()),
                provider: "mock".to_string(),
                model: "mock".to_string(),
                usage: Default::default(),
                tool_calls: None,
            })
        } else if content.contains("Extract key technical concepts") {
            Ok(LLMResponse {
                content: Some("Rust API memory consolidation".to_string()),
                provider: "mock".to_string(),
                model: "mock".to_string(),
                usage: Default::default(),
                tool_calls: None,
            })
        } else if content.contains("workflow") {
            Ok(LLMResponse {
                content: Some("NONE".to_string()),
                provider: "mock".to_string(),
                model: "mock".to_string(),
                usage: Default::default(),
                tool_calls: None,
            })
        } else {
            Ok(LLMResponse {
                content: Some("Test response".to_string()),
                provider: "mock".to_string(),
                model: "mock".to_string(),
                usage: Default::default(),
                tool_calls: None,
            })
        }
    }

    #[tokio::test]
    async fn test_extract_facts() {
        let consolidator = HarnessConsolidator::new(mock_harness);
        let facts = consolidator.extract_facts("Some memory content").await.unwrap();
        assert!(!facts.is_empty());
    }

    #[tokio::test]
    async fn test_extract_concepts() {
        let consolidator = HarnessConsolidator::new(mock_harness);
        let concepts = consolidator.extract_concepts("Rust memory API").await.unwrap();
        assert!(!concepts.is_empty());
    }
}
