use anyhow::Result;
use tracing_subscriber::EnvFilter;

#[cfg(not(feature = "desktop"))]
use std::io::{self, BufRead, Write};

// ═══════════════════════════════════════════════════════════════════════════
// DESKTOP MODE (Tauri v2) — Production app
// ═══════════════════════════════════════════════════════════════════════════
#[cfg(feature = "desktop")]
fn main() {
    use tauri::Manager;
    use tauri::Emitter; // for .emit()

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    tracing::info!("Starting CULI Agent (Desktop mode)...");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Setup system tray
            culi::tray::setup_tray(app)?;

            let handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                // ── Step 1: Spawn CulirouterAPI từ bundled resources ────
                let router_pid = spawn_router_sidecar(&handle);
                match router_pid {
                    Some(pid) => tracing::info!("✅ CulirouterAPI started (pid={})", pid),
                    None      => tracing::warn!("⚠️  CulirouterAPI not started — will try localhost:4000"),
                }

                // ── Step 2: Wait for router health check ─────────────
                let router_ready = wait_for_router(10).await;
                if router_ready {
                    tracing::info!("✅ CulirouterAPI ready at :4000");
                } else {
                    tracing::warn!("⚠️  CulirouterAPI not responding — chat will use fallback");
                }

                // ── Step 3: Init CULI orchestrator ───────────────────
                tracing::info!("Initializing CULI orchestrator...");
                match culi::initialize(None).await {
                    Ok(orchestrator) => {
                        let state = culi::tauri_commands::TauriAppState::new(orchestrator);
                        handle.manage(state);
                        tracing::info!("✅ CULI ready");
                        let _ = handle.emit("culi://ready", serde_json::json!({
                            "router": router_ready,
                        }));
                    }
                    Err(e) => {
                        tracing::error!("❌ CULI init failed: {}", e);
                        let _ = handle.emit("culi://error", format!("{}", e));
                    }
                }

                // ── Step 4: Show main window after init ──────────────
                {
                    use tauri::Manager;
                    if let Some(window) = handle.get_webview_window("main") {
                        window.show().unwrap_or_default();
                        window.set_focus().unwrap_or_default();
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // X button → minimize to tray (không quit)
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap_or_default();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            culi::tauri_commands::send_chat,
            culi::tauri_commands::get_memory_stats,
            culi::tauri_commands::get_router_stats,
            culi::tauri_commands::get_health,
            culi::tauri_commands::run_audit,
            culi::tauri_commands::get_context_summary,
            quit_app,
            show_window,
            get_app_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Spawn CulirouterAPI từ bundled resources
/// Tìm node.exe trong: <resource_dir>/router/node.exe
/// Chạy: node.exe <resource_dir>/router/server.js
#[cfg(feature = "desktop")]
fn spawn_router_sidecar(app: &tauri::AppHandle) -> Option<u32> {
    use tauri::Manager;

    let resource_dir = app.path().resource_dir().ok()?;
    let router_dir   = resource_dir.join("router");
    let node_exe     = router_dir.join("node.exe");
    let server_js    = router_dir.join("server.js");

    if !node_exe.exists() {
        tracing::warn!("node.exe not found at {}", node_exe.display());
        // Fallback: try system node
        let node_exe = std::path::PathBuf::from("node");
        return spawn_with_node(&node_exe, &server_js, &router_dir);
    }

    spawn_with_node(&node_exe, &server_js, &router_dir)
}

#[cfg(feature = "desktop")]
fn spawn_with_node(
    node_exe: &std::path::Path,
    server_js: &std::path::Path,
    working_dir: &std::path::Path,
) -> Option<u32> {
    use std::process::{Command, Stdio};

    tracing::info!("Spawning: {} {}", node_exe.display(), server_js.display());

    let child = Command::new(node_exe)
        .arg(server_js)
        .current_dir(working_dir)
        .env("PORT", "4000")
        .env("CULI_EMBEDDED", "1")  // Router biết đang chạy embedded
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    match child {
        Ok(c) => {
            let pid = c.id();
            // Leak child intentionally — OS will clean up when Tauri process exits
            std::mem::forget(c);
            Some(pid)
        }
        Err(e) => {
            tracing::warn!("Failed to spawn node: {}", e);
            None
        }
    }
}

/// Poll :4000/health until ready or timeout (seconds)
#[cfg(feature = "desktop")]
async fn wait_for_router(timeout_secs: u64) -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();

    let deadline = std::time::Instant::now()
        + std::time::Duration::from_secs(timeout_secs);

    while std::time::Instant::now() < deadline {
        if let Ok(r) = client.get("http://127.0.0.1:4000/health").send().await {
            if r.status().is_success() {
                return true;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    false
}

/// Quit app từ frontend
#[cfg(feature = "desktop")]
#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    tracing::info!("Quit requested");
    app.exit(0);
}

/// Show window (từ tray menu)
#[cfg(feature = "desktop")]
#[tauri::command]
fn show_window(app: tauri::AppHandle) {
    use tauri::Manager;
    if let Some(w) = app.get_webview_window("main") {
        w.show().unwrap_or_default();
        w.set_focus().unwrap_or_default();
    }
}

/// Get app version
#[cfg(feature = "desktop")]
#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ═══════════════════════════════════════════════════════════════════════════
// CLI MODE (Default - Standalone server or interactive REPL)
// ═══════════════════════════════════════════════════════════════════════════
#[cfg(not(feature = "desktop"))]
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    tracing::info!("Starting CULI Agent (CLI mode)...");

    // Check for --serve flag to start HTTP API server
    let args: Vec<String> = std::env::args().collect();
    let serve_mode = args.contains(&"--serve".to_string());

    // Initialize CULI engine
    let mut orchestrator = culi::initialize(None).await?;
    
    tracing::info!("CULI Agent ready.");
    tracing::info!("Session ID: {}", orchestrator.session_id());

    // If --serve flag is present, start HTTP API server
    if serve_mode {
        let mut config = culi::config::Config::default();
        config.data_dir = Some("data/culi".to_string());
        let app_state = culi::api::AppState::new(orchestrator, config);
        culi::api::start_server(app_state, 3111).await?;
        return Ok(());
    }

    tracing::info!("Type '/help' for commands.");

    // Interactive CLI loop
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    loop {
        print!("\n> ");
        stdout.flush()?;
        
        let mut input = String::new();
        stdin.lock().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }

        // Handle commands
        match input {
            "/quit" | "/exit" => {
                println!("Goodbye!");
                break;
            }
            "/help" => {
                println!("CULI Agent Commands:");
                println!("  /help              - Show this help");
                println!("  /quit, /exit       - Exit CULI");
                println!("  /audit <path>      - Run security gate audit");
                println!("  /errors            - Show error memory");
                println!("  /reflect           - Generate error reflection");
                println!("  /context           - Show context summary");
                println!("  /fast <path>       - Get fast project context");
                println!("  <anything else>    - Send to agent loop");
                continue;
            }
            "/audit " if input.starts_with("/audit ") => {
                let path = input.trim_start_matches("/audit ").trim();
                println!("🔍 Running security audit on: {}", path);
                
                use culi::agents::security_auditor::SecurityAuditor;
                use std::path::Path;
                
                let auditor = SecurityAuditor::new();
                match auditor.audit_codebase(Path::new(path)) {
                    Ok(report) => {
                        println!("\n{}", report.to_markdown());
                        
                        // Save reports
                        if let Err(e) = std::fs::write("audit_report.md", report.to_markdown()) {
                            eprintln!("Failed to save markdown report: {}", e);
                        }
                        if let Ok(json) = report.to_json() {
                            if let Err(e) = std::fs::write("audit_report.json", json) {
                                eprintln!("Failed to save JSON report: {}", e);
                            }
                        }
                        println!("\n📄 Reports saved: audit_report.md, audit_report.json");
                        
                        // Exit with error if critical issues
                        if report.stats.critical > 0 {
                            println!("\n❌ Audit failed with {} critical issues", report.stats.critical);
                        } else {
                            println!("\n✅ Audit passed!");
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Audit failed: {}", e);
                    }
                }
                continue;
            }
            "/errors" => {
                if let Ok(response) = orchestrator.query_sub_agent(
                    culi::subagent::SubAgentRequest::GetRelevantErrors {
                        query: "recent".to_string(),
                        limit: 10,
                    }
                ).await {
                    match response {
                        culi::subagent::SubAgentResponse::ErrorList(errors) => {
                            if errors.is_empty() {
                                println!("No errors recorded.");
                            } else {
                                println!("Recent errors:");
                                for e in &errors {
                                    println!("  [{}] {} (x{}) - {}",
                                        e.error_type, e.title, e.frequency,
                                        if e.resolved { "✓ resolved" } else { "✗ unresolved" }
                                    );
                                }
                            }
                        }
                        _ => println!("Unexpected response from sub-agent"),
                    }
                } else {
                    println!("Sub-agent not available (errors module)");
                }
                continue;
            }
            "/reflect" => {
                if let Ok(response) = orchestrator.query_sub_agent(
                    culi::subagent::SubAgentRequest::Reflect(())
                ).await {
                    match response {
                        culi::subagent::SubAgentResponse::Reflection(r) => {
                            println!("=== Error Reflection ===");
                            println!("Total errors: {}", r.total_errors);
                            println!("Resolved: {}", r.resolved_errors);
                            println!("Unresolved: {}", r.unresolved_errors);
                            println!("Top patterns:");
                            for p in &r.recurring_patterns {
                                println!("  - {}", p);
                            }
                        }
                        _ => println!("Unexpected response"),
                    }
                }
                continue;
            }
            "/context" => {
                println!("{}", orchestrator.context.get_context_summary());
                continue;
            }
            "/fast " if input.starts_with("/fast ") => {
                let path = input.trim_start_matches("/fast ").trim();
                if let Ok(response) = orchestrator.query_sub_agent(
                    culi::subagent::SubAgentRequest::GetFastContext {
                        project_path: path.to_string(),
                    }
                ).await {
                    match response {
                        culi::subagent::SubAgentResponse::FastContext(ctx) => {
                            println!("=== Fast Context: {} ===", ctx.project_name);
                            println!("Language: {}", ctx.language);
                            println!("Files: {}", ctx.file_count);
                            println!("Entry points: {:?}", ctx.entry_points);
                        }
                        _ => println!("Unexpected response"),
                    }
                }
                continue;
            }
            _ => {} // Normal input - process below
        }

        // Run the agent loop
        match orchestrator.run(input).await {
            Ok(response) => {
                match response {
                    culi::orchestrator::AgentResponse::Complete(output) => {
                        println!("\n{}", output.content);
                        if !output.tool_calls.is_empty() {
                            println!("\n[Tools used: {} iterations, {} tokens]",
                                output.iterations, output.tokens_used);
                        }
                    }
                    culi::orchestrator::AgentResponse::Partial(output) => {
                        println!("\n{}", output.content);
                        println!("\n[Partial: max iterations reached]");
                    }
                    culi::orchestrator::AgentResponse::Error(e) => {
                        println!("\n[Error] {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[Error] Agent loop failed: {}", e);
                // Auto-record error
                let _ = orchestrator.log_event("error", &format!("Agent loop failed: {}", e)).await;
            }
        }
    }

    Ok(())
}
