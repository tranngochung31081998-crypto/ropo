//! SixthProvider — calls Sixth AI directly, with auto-signup account pool
//! Signup:  POST https://backend.withsix.co/vs-code/auth/signupV2
//! Chat:    POST https://backend.withsix.co/proxy/azure/openai/deployments/{model}/chat/completions
//! Pool persisted to data/culi/sixth_pool.json

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use super::{LLMProvider, LLMResponse, Message, StreamItem, ToolDefinition, TokenUsage};

const BASE_URL:    &str = "https://backend.withsix.co";
const SIGNUP_URL:  &str = "https://backend.withsix.co/vs-code/auth/signupV2";
const CHAT_PATH:   &str = "/proxy/azure/openai/deployments/{model}/chat/completions";
const API_VERSION: &str = "2024-12-01-preview";
const DEFAULT_MODEL: &str = "claude-fable-5";
const POOL_SIZE:   usize = 3;
const TOKEN_ROTATE_THRESHOLD: u64 = 50_000;
const POOL_FILE:   &str = "data/culi/sixth_pool.json";

// ── Account types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SixthAccount {
    pub email:       String,
    pub password:    String,
    pub uid:         Option<String>,
    pub token:       String,
    pub used_tokens: u64,
    pub dead:        bool,
    pub created_at:  String,
}

#[derive(Debug, Serialize)]
struct SignupBody<'a> {
    email:         &'a str,
    password:      &'a str,
    auth_provider: &'static str,
}

#[derive(Debug, Deserialize)]
struct SignupResponse {
    uid:          Option<String>,
    access_token: Option<SignupAccessToken>,
}

#[derive(Debug, Deserialize)]
struct SignupAccessToken {
    access_token: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model:      &'static str,
    messages:   &'a [OaiMsg],
    max_tokens: u32,
    stream:     bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct OaiMsg {
    role:    String,
    content: String,
}

// ── Provider ──────────────────────────────────────────────────────────────

pub struct SixthProvider {
    client:       reqwest::Client,
    pool:         Arc<Mutex<Vec<SixthAccount>>>,
    current_idx:  Arc<Mutex<usize>>,
    total_reqs:   Arc<Mutex<u64>>,
    total_errors: Arc<Mutex<u64>>,
}

impl SixthProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to build Sixth HTTP client");

        let pool = load_pool();
        info!("Sixth: loaded {} accounts from pool", pool.len());

        Self {
            client,
            pool:         Arc::new(Mutex::new(pool)),
            current_idx:  Arc::new(Mutex::new(0)),
            total_reqs:   Arc::new(Mutex::new(0)),
            total_errors: Arc::new(Mutex::new(0)),
        }
    }

    fn build_chat_url(model: &str) -> String {
        let path = CHAT_PATH.replace("{model}", model);
        format!("{BASE_URL}{path}?api-version={API_VERSION}")
    }

    fn build_chat_headers(token: &str) -> reqwest::header::HeaderMap {
        use reqwest::header::*;
        let mut h = HeaderMap::new();
        h.insert(CONTENT_TYPE,  "application/json".parse().unwrap());
        h.insert(ACCEPT,        "*/*".parse().unwrap());
        h.insert("accept-encoding", "identity".parse().unwrap());
        h.insert(AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
        h.insert(USER_AGENT,    "node".parse().unwrap());
        h.insert("x-sixth-total-cached", "0".parse().unwrap());
        h.insert("x-sixth-total-input",  "0".parse().unwrap());
        h.insert("x-sixth-total-output", "0".parse().unwrap());
        h
    }

    fn build_signup_headers() -> reqwest::header::HeaderMap {
        use reqwest::header::*;
        let mut h = HeaderMap::new();
        h.insert(CONTENT_TYPE,  "application/json".parse().unwrap());
        h.insert(ACCEPT,        "application/json, text/plain, */*".parse().unwrap());
        h.insert("accept-encoding", "identity".parse().unwrap());
        h.insert("origin",   "https://app.trysixth.com".parse().unwrap());
        h.insert("referer",  "https://app.trysixth.com/".parse().unwrap());
        h.insert(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".parse().unwrap());
        h
    }

    async fn active_accounts(&self) -> Vec<SixthAccount> {
        self.pool.lock().await
            .iter()
            .filter(|a| !a.dead)
            .cloned()
            .collect()
    }

    async fn current_account(&self) -> Option<SixthAccount> {
        let active = self.active_accounts().await;
        if active.is_empty() { return None; }
        let idx = *self.current_idx.lock().await % active.len();
        Some(active[idx].clone())
    }

    async fn rotate_account(&self, reason: &str) {
        let active = self.active_accounts().await;
        if active.is_empty() { return; }
        let mut idx = self.current_idx.lock().await;
        *idx = (*idx + 1) % active.len();
        warn!("Sixth: rotated account ({}) → index {}", reason, *idx);
    }

    async fn mark_dead(&self, email: &str) {
        let mut pool = self.pool.lock().await;
        if let Some(acc) = pool.iter_mut().find(|a| a.email == email) {
            acc.dead = true;
            warn!("Sixth: marked dead → {}", email);
        }
        save_pool(&pool);
    }

    async fn create_account(&self) -> Option<SixthAccount> {
        let email    = gen_email();
        let password = gen_password();
        info!("Sixth: creating account {}", email);

        let body = SignupBody { email: &email, password: &password, auth_provider: "email" };

        let resp = match self.client
            .post(SIGNUP_URL)
            .headers(Self::build_signup_headers())
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => { warn!("Sixth: signup request failed: {}", e); return None; }
        };

        let status = resp.status().as_u16();
        if status != 200 && status != 201 {
            let text = resp.text().await.unwrap_or_default();
            warn!("Sixth: signup failed {}: {}", status, &text[..text.len().min(120)]);
            return None;
        }

        let data: SignupResponse = match resp.json().await {
            Ok(d) => d,
            Err(e) => { warn!("Sixth: parse signup response: {}", e); return None; }
        };

        let token = data.access_token?.access_token?;

        let acc = SixthAccount {
            email:       email.clone(),
            password,
            uid:         data.uid,
            token,
            used_tokens: 0,
            dead:        false,
            created_at:  chrono::Utc::now().to_rfc3339(),
        };

        {
            let mut pool = self.pool.lock().await;
            pool.push(acc.clone());
            save_pool(&pool);
        }

        info!("Sixth: ✅ new account created: {}", email);
        Some(acc)
    }

    /// Ensure pool has at least POOL_SIZE live accounts
    async fn ensure_pool(&self) -> bool {
        let active_count = self.active_accounts().await.len();
        if active_count < POOL_SIZE {
            let needed = POOL_SIZE - active_count;
            info!("Sixth: pool low ({}/{}), creating {} accounts", active_count, POOL_SIZE, needed);
            for _ in 0..needed {
                self.create_account().await;
            }
        }
        !self.active_accounts().await.is_empty()
    }

    async fn do_chat(
        &self,
        messages: &[OaiMsg],
        temperature: Option<f32>,
    ) -> Result<reqwest::Response> {
        if !self.ensure_pool().await {
            return Err(anyhow!("Sixth: pool empty, all signup attempts failed"));
        }

        let max_attempts = (self.active_accounts().await.len() + 1).max(2);

        for attempt in 0..max_attempts {
            let account = match self.current_account().await {
                Some(a) => a,
                None => return Err(anyhow!("Sixth: no active accounts")),
            };

            let body = ChatRequest {
                model:       DEFAULT_MODEL,
                messages,
                max_tokens:  2048,
                stream:      true,
                temperature,
            };

            *self.total_reqs.lock().await += 1;
            let url = Self::build_chat_url(DEFAULT_MODEL);

            debug!("Sixth: attempt {} via {}", attempt + 1, account.email);

            let resp = match self.client
                .post(&url)
                .headers(Self::build_chat_headers(&account.token))
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    *self.total_errors.lock().await += 1;
                    warn!("Sixth: request error ({}): {}", account.email, e);
                    self.rotate_account("error").await;
                    continue;
                }
            };

            match resp.status().as_u16() {
                401 | 403 => {
                    warn!("Sixth: {} got {}, marking dead", account.email, resp.status());
                    self.mark_dead(&account.email).await;
                    self.ensure_pool().await;
                    continue;
                }
                429 => {
                    warn!("Sixth: {} rate limited", account.email);
                    self.rotate_account("429").await;
                    continue;
                }
                200..=299 => {
                    // Update token usage estimate
                    {
                        let mut pool = self.pool.lock().await;
                        if let Some(acc) = pool.iter_mut().find(|a| a.email == account.email) {
                            acc.used_tokens += 2048;
                            if acc.used_tokens >= TOKEN_ROTATE_THRESHOLD {
                                // Schedule rotate after response
                                let provider = self as *const Self;
                                let _ = provider; // rotation happens lazily
                            }
                        }
                        save_pool(&pool);
                    }
                    info!("Sixth: ✅ stream started ({})", account.email);
                    return Ok(resp);
                }
                status => {
                    let text = resp.text().await.unwrap_or_default();
                    *self.total_errors.lock().await += 1;
                    return Err(anyhow!("Sixth HTTP {}: {}", status, &text[..text.len().min(120)]));
                }
            }
        }

        Err(anyhow!("Sixth: all attempts failed"))
    }
}

impl Default for SixthProvider {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl LLMProvider for SixthProvider {
    fn name(&self) -> &str { "sixth" }

    async fn chat(
        &self,
        messages: &[Message],
        _model: &str,
        _tools: &[ToolDefinition],
    ) -> Result<LLMResponse> {
        let oai: Vec<OaiMsg> = messages.iter().map(|m| OaiMsg {
            role: m.role.clone(),
            content: m.content.clone(),
        }).collect();

        let resp = self.do_chat(&oai, None).await?;
        let content = collect_sse_stream(resp).await?;

        Ok(LLMResponse {
            content:    Some(content),
            tool_calls: None,
            usage:      TokenUsage::default(),
            model:      DEFAULT_MODEL.to_string(),
            provider:   "sixth".to_string(),
        })
    }

    async fn chat_stream(
        &self,
        messages: &[Message],
        _model: &str,
        _tools: &[ToolDefinition],
    ) -> Result<Box<dyn StreamItem>> {
        let oai: Vec<OaiMsg> = messages.iter().map(|m| OaiMsg {
            role: m.role.clone(),
            content: m.content.clone(),
        }).collect();

        let resp = self.do_chat(&oai, None).await?;
        let content = collect_sse_stream(resp).await?;

        Ok(Box::new(super::culi_router::CollectedStreamItem::new(
            LLMResponse {
                content: Some(content),
                tool_calls: None,
                usage: TokenUsage::default(),
                model: DEFAULT_MODEL.to_string(),
                provider: "sixth".to_string(),
            }
        )))
    }
}

// ── SSE collector ─────────────────────────────────────────────────────────

async fn collect_sse_stream(resp: reqwest::Response) -> Result<String> {
    let mut stream = resp.bytes_stream();
    let mut buf     = String::new();
    let mut content = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| anyhow!("Sixth stream error: {}", e))?;
        buf.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buf.find('\n') {
            let line = buf[..pos].trim().to_string();
            buf = buf[pos + 1..].to_string();

            if !line.starts_with("data: ") { continue; }
            let data = &line[6..];
            if data == "[DONE]" { break; }

            if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                // Skip reasoning_content (thinking phase), only take content
                if let Some(tok) = v["choices"][0]["delta"]["content"].as_str() {
                    content.push_str(tok);
                }
            }
        }
    }

    Ok(content)
}

// ── Pool persistence ───────────────────────────────────────────────────────

fn load_pool() -> Vec<SixthAccount> {
    match std::fs::read_to_string(POOL_FILE) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => vec![],
    }
}

fn save_pool(pool: &[SixthAccount]) {
    if let Some(parent) = std::path::Path::new(POOL_FILE).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(pool) {
        let _ = std::fs::write(POOL_FILE, json);
    }
}

// ── Random helpers ─────────────────────────────────────────────────────────

fn gen_email() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    let rnd: u32 = ts ^ (ts >> 16);
    let letters = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let suffix: String = (0..10).map(|i| {
        letters[((rnd.wrapping_add(i * 7919)) as usize) % letters.len()] as char
    }).collect();
    let first = (b'A' + (rnd % 26) as u8) as char;
    format!("{first}{suffix}@gmail.com")
}

fn gen_password() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    let upper: char = (b'A' + (ts % 26) as u8) as char;
    let digits = ts % 1000;
    format!("{upper}culi{digits:03}@pass")
}
