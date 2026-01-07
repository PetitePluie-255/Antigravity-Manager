use antigravity_server::api::build_routes;
use antigravity_server::state::AppState;
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = 3000)]
    port: u16,

    #[arg(short, long, env = "DATA_DIR")]
    data_dir: Option<PathBuf>,

    /// Directory containing static frontend files (for production)
    #[arg(long, env = "STATIC_DIR")]
    static_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    let state = if let Some(dir) = args.data_dir {
        AppState::with_data_dir(dir)
            .await
            .expect("Failed to init state")
    } else {
        AppState::new().await.expect("Failed to init state")
    };

    let app_state = Arc::new(state);

    // Load accounts into TokenManager on startup
    match app_state.token_manager.load_accounts().await {
        Ok(count) => {
            tracing::info!("Loaded {} accounts into token pool on startup", count);
        }
        Err(e) => {
            tracing::warn!("Failed to load accounts on startup: {} (this may be expected if no accounts exist yet)", e);
        }
    }

    // Add CORS
    let cors = CorsLayer::permissive();

    // --- Proxy Routes Integration ---
    use antigravity_server::api::common;
    use antigravity_server::proxy::handlers;
    use axum::{
        extract::State,
        routing::{get, post},
        Router,
    };

    // Check enabled middleware
    let check_proxy_enabled = |State(state): State<Arc<AppState>>,
                               req: axum::http::Request<axum::body::Body>,
                               next: axum::middleware::Next| async move {
        if state
            .proxy_enabled
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            Ok(next.run(req).await)
        } else {
            Err(axum::http::StatusCode::SERVICE_UNAVAILABLE)
        }
    };

    let api_routes = build_routes(app_state.clone());

    let proxy_routes = Router::new()
        // OpenAI
        .route(
            "/v1/chat/completions",
            post(handlers::openai::handle_chat_completions),
        )
        .route(
            "/v1/completions",
            post(handlers::openai::handle_completions),
        )
        // OpenAI Responses (Codex) - for /v1/responses endpoint
        .route("/v1/responses", post(handlers::openai::handle_completions))
        // OpenAI Images API (merged from main)
        .route(
            "/v1/images/generations",
            post(handlers::openai::handle_images_generations),
        )
        .route(
            "/v1/images/edits",
            post(handlers::openai::handle_images_edits),
        )
        .route("/v1/models", get(handlers::openai::handle_list_models))
        // Claude
        .route("/v1/messages", post(handlers::claude::handle_messages))
        // Gemini
        .route(
            "/v1beta/models/:model",
            post(handlers::gemini::handle_generate).get(handlers::gemini::handle_get_model),
        )
        // z.ai MCP routes
        .route(
            "/mcp/web-search",
            post(handlers::mcp::handle_web_search_prime)
                .get(handlers::mcp::handle_web_search_prime)
                .delete(handlers::mcp::handle_web_search_prime),
        )
        .route(
            "/mcp/web-reader",
            post(handlers::mcp::handle_web_reader)
                .get(handlers::mcp::handle_web_reader)
                .delete(handlers::mcp::handle_web_reader),
        )
        .route(
            "/mcp/zai-vision",
            post(handlers::mcp::handle_zai_mcp_server)
                .get(handlers::mcp::handle_zai_mcp_server)
                .delete(handlers::mcp::handle_zai_mcp_server),
        )
        // Compatibility Aliases
        .route(
            "/v1/v1/chat/completions",
            post(handlers::openai::handle_chat_completions),
        )
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            check_proxy_enabled,
        ))
        .with_state(app_state.clone()); // Attach state to proxy routes to finalize them
                                        // Note: We might want to add auth middleware here if needed, consistent with proxy config

    let app = Router::new()
        .merge(api_routes)
        .merge(proxy_routes) // Merge proxy routes at root level
        .layer(cors.clone())
        .layer(axum::middleware::from_fn(common::request_logger));

    // Add static file serving if STATIC_DIR is provided (production mode)
    let app = if let Some(static_dir) = &args.static_dir {
        let index_path = static_dir.join("index.html");
        if static_dir.exists() && index_path.exists() {
            tracing::info!("Serving static files from {:?}", static_dir);
            // ServeDir with fallback to index.html for SPA routing
            let serve_dir =
                ServeDir::new(static_dir).not_found_service(ServeFile::new(&index_path));
            app.fallback_service(serve_dir)
        } else {
            tracing::warn!("Static directory {:?} or index.html not found", static_dir);
            app
        }
    } else {
        app
    };

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
