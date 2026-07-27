//! HTTP API Server for CULI Agent
//! Provides REST endpoints for frontend integration

pub mod routes;
pub mod models;
pub mod background;
pub mod streaming;

use anyhow::Result;
use axum::Router;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::orchestrator::Orchestrator;
use crate::memory::MemoryPipeline;
use crate::provider::{ProviderRouter, RouterConfig, LLMResponse, Message};

/// Lightweight chat service — calls LLM directly without full orchestrator
/// This avoids the Send issue since Orchestrator contains non-Send SubAgent types.
pub struct ChatService {
    router: Arc<tokio::sync::Mutex<ProviderRouter>>,
    model: String,
}

impl ChatService {
    pub fn new(config: &Config) -> Self {
        let router_config = RouterConfig {
            max_retries: config.provider.max_retries,
            timeout_seconds: config.provider.timeout_seconds,
            enable_fallback: true,
            enable_token_tracking: true,
            use_composite_tiers: false,
        };
        let mut router = ProviderRouter::new(router_config);

        // ── Priority 1: Qveris (user-facing "CULI Models")
        // Registered first — used when user explicitly picks a CULI model
        router.register_provider("qveris", Box::new(crate::provider::QverisProvider::new()));
        tracing::info!("✅ QverisProvider registered (CULI Models layer)");

        // ── Priority 2: Blackbox (harness layer — free)
        router.register_provider("blackbox", Box::new(crate::provider::BlackboxProvider::new()));
        tracing::info!("✅ BlackboxProvider registered (harness layer)");

        // ── Priority 3: Sixth AI (harness layer — free, better quality)
        router.register_provider("sixth", Box::new(crate::provider::SixthProvider::new()));
        tracing::info!("✅ SixthProvider registered (harness layer)");

        // ── Fallback: Ollama (local, always available)
        router.register_provider("ollama", Box::new(crate::provider::OllamaProvider::new()));
        tracing::info!("✅ Ollama registered (local fallback)");

        // ── Routing chain:
        // - CULI models (culi-*) → resolved to qveris → if qveris fails → sixth → blackbox
        // - Harness tasks         → sixth → blackbox (bypass qveris to save credits)
        // - Fallback              → ollama
        router.set_primary("qveris").ok();
        router.set_fallback_chain(vec![
            "qveris".into(),
            "sixth".into(),
            "blackbox".into(),
            "ollama".into(),
        ]);
        tracing::info!("✅ Provider chain: qveris → sixth → blackbox → ollama");

        Self {
            router: Arc::new(tokio::sync::Mutex::new(router)),
            model: config.provider.model.clone(),
        }
    }

    /// Standard chat — resolves CULI model names before calling
    pub async fn chat(&self, messages: Vec<Message>) -> Result<LLMResponse> {
        self.chat_with_model(messages, &self.model.clone(), &[]).await
    }

    /// Chat with explicit model — resolves "culi-*" to underlying Qveris model
    /// Now accepts tool definitions for function calling
    pub async fn chat_with_model(
        &self,
        messages: Vec<Message>,
        model: &str,
        tools: &[crate::provider::ToolDefinition],
    ) -> Result<LLMResponse> {
        use crate::provider::resolve_culi_model;

        let resolved = resolve_culi_model(model);
        let effective = if resolved.is_empty() { model } else { &resolved };

        tracing::info!("💬 ChatService: calling {} with {} tools", effective, tools.len());

        let mut router = self.router.lock().await;
        let response = router.chat(&messages, effective, tools).await?;
        let _ = router.track_token_usage(&response.provider, &response.usage);
        Ok(response)
    }

    /// Harness chat — always uses Sixth/Blackbox, NEVER Qveris (save credits)
    pub async fn harness_chat(&self, messages: Vec<Message>) -> Result<LLMResponse> {
        #[allow(unused_mut)]
        let mut router = self.router.lock().await;
        let tools = vec![]; // Harness tasks typically don't need tools

        // Try Sixth first (better quality for summaries)
        match router.chat_with_provider("sixth", &messages, "claude-fable-5", &tools).await {
            Ok(r) => return Ok(r),
            Err(e) => tracing::warn!("Harness: Sixth failed ({}), trying Blackbox", e),
        }
        // Fallback to Blackbox
        router.chat_with_provider("blackbox", &messages, "deepseek-v4-flash", &tools).await
    }
}

/// Shared application state for API handlers
pub struct AppState {
    pub orchestrator: Arc<tokio::sync::Mutex<Orchestrator>>,
    pub memory: Arc<tokio::sync::Mutex<MemoryPipeline>>,
    pub chat_service: ChatService,
    pub graph_storage: Arc<tokio::sync::Mutex<Option<crate::graph::persistence::GraphStorage>>>,
    pub config: Config,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

impl AppState {
    pub fn new(orchestrator: Orchestrator, config: Config) -> Self {
        // Initialize persistent SQLite storage if data_dir is configured
        let storage = config.data_dir.as_ref().map(|data_dir| {
            let db_path = format!("{}/memory.db", data_dir);
            match crate::memory::MemoryStorage::new(&db_path) {
                Ok(s) => {
                    tracing::info!("Memory storage initialized at {}", db_path);
                    Some(s)
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize memory storage at {}: {}", db_path, e);
                    None
                }
            }
        }).flatten();

        let graph_storage = config.data_dir.as_ref().map(|data_dir| {
            let db_path = format!("{}/code_graph.db", data_dir);
            match crate::graph::persistence::GraphStorage::open(std::path::Path::new(&db_path)) {
                Ok(s) => {
                    tracing::info!("Graph storage initialized at {}", db_path);
                    Some(s)
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize graph storage at {}: {}", db_path, e);
                    None
                }
            }
        }).flatten();

        Self {
            orchestrator: Arc::new(tokio::sync::Mutex::new(orchestrator)),
            memory: Arc::new(tokio::sync::Mutex::new(MemoryPipeline::with_storage(storage))),
            chat_service: ChatService::new(&config),
            graph_storage: Arc::new(tokio::sync::Mutex::new(graph_storage)),
            config,
            start_time: chrono::Utc::now(),
        }
    }
}

/// Start the HTTP API server
pub async fn start_server(state: AppState, port: u16) -> Result<()> {
    let app_state = Arc::new(state);

    // Start background tasks (harness layer)
    background::start_background_tasks(app_state.clone());

    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    let app = Router::new()
        .nest("/api", routes::create_router())
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(app_state);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("🚀 CULI API server starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
