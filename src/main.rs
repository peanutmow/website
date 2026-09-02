use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use std::sync::Arc;
use tower_http::services::ServeDir;
use tower_http::compression::CompressionLayer;
use tracing_subscriber::EnvFilter;

mod projects;
mod templates;

pub struct AppState {
    pub tmpl: templates::TemplateEngine,
    pub projects: Vec<projects::Project>,
    pub dreams: Vec<projects::Project>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let projects = projects::load_projects();
    tracing::info!("Loaded {} projects", projects.len());

    let dreams = projects::load_dreams();
    tracing::info!("Loaded {} dreams", dreams.len());

    let state = Arc::new(AppState {
        tmpl: templates::TemplateEngine::new(),
        projects,
        dreams,
    });

    let app = Router::new()
        // SSR pages (rendered by Rust)
        .route("/", get(root_page))
        .route("/index.html", get(root_page))
        // Blog - static files
        .nest_service("/blog", ServeDir::new("blog"))
        // Gallery & Socials - SSR pages at root, serve original static sub-pages
        .route("/gallery", get(gallery_page))
        .route("/gallery/", get(gallery_page))
        .route("/gallery/index.html", get(|| serve_file("gallery/index.html", "text/html; charset=utf-8")))
        .route("/socials", get(socials_page))
        .route("/socials/", get(socials_page))
        .route("/socials/index.html", get(|| serve_file("socials/index.html", "text/html; charset=utf-8")))
        // Projects - SSR
        .route("/projects", get(projects_page))
        .route("/projects/", get(projects_page))
        .route("/projects/index.html", get(projects_page))
        // Static directories (avoid conflicts with SSR routes)
        .nest_service("/fonts", ServeDir::new("fonts"))
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/assets", ServeDir::new("assets"))
        .nest_service("/wasm", ServeDir::new("wasm-sim/pkg"))
        // Root-level static files
        .route("/water-sim.js", get(|| serve_file("water-sim.js", "application/javascript")))
        .route("/qr-error.png", get(|| serve_file("templates/QRCode(3).png", "image/png")))
        // Easter egg pages
        .route("/dev/null", get(dev_null_redirect))
        .route("/redherring", get(redherring_page))
        // The bunny page — any /conejillo/… URL (long paths, matrix params,
        // percent-encoding, query strings, fragments) lands here.
        .route("/conejillo", get(conejillo_page))
        .route("/conejillo/*rest", get(conejillo_page))
        // Fallback 404
        .fallback(not_found)
        .layer(CompressionLayer::new())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    tracing::info!("Server running on http://0.0.0.0:8080");
    axum::serve(listener, app).await.unwrap();
}

// ─── SSR Handlers ──────────────────────────────────────────────────

async fn root_page(axum::extract::State(state): axum::extract::State<Arc<AppState>>) -> Response {
    state.tmpl.render_response("index.html", &serde_json::json!({"title": "Alice Portfolio"}))
}

async fn gallery_page(axum::extract::State(state): axum::extract::State<Arc<AppState>>) -> Response {
    state.tmpl.render_response("gallery.html", &serde_json::json!({"title": "Gallery — Alice"}))
}

async fn socials_page(axum::extract::State(state): axum::extract::State<Arc<AppState>>) -> Response {
    state.tmpl.render_response("socials.html", &serde_json::json!({"title": "Socials — Alice"}))
}

async fn projects_page(axum::extract::State(state): axum::extract::State<Arc<AppState>>) -> Response {
    let live_count = state
        .projects
        .iter()
        .filter(|p| p.section == "featured" || p.section == "work")
        .count();
    let dream_count = state.dreams.len();
    state.tmpl.render_response("projects.html", &serde_json::json!({
        "title": "Projects — Alice",
        "projects": state.projects,
        "dreams": state.dreams,
        "live_count": live_count,
        "dream_count": dream_count,
    }))
}

// ─── File serving ──────────────────────────────────────────────────

async fn serve_file(path: &str, mime: &str) -> Response {
    match tokio::fs::read(path).await {
        Ok(data) => Response::builder()
            .header("Content-Type", mime)
            .body(axum::body::Body::from(data)).unwrap(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

// ─── Easter egg handlers ───────────────────────────────────────────

async fn dev_null_redirect() -> Redirect {
    Redirect::to("/")
}

async fn redherring_page(axum::extract::State(state): axum::extract::State<Arc<AppState>>) -> Response {
    state.tmpl.render_response("redherring.html", &serde_json::json!({"title": "///"}))
}

async fn conejillo_page(axum::extract::State(state): axum::extract::State<Arc<AppState>>) -> Response {
    state.tmpl.render_response("conejillo.html", &serde_json::json!({"title": "🐰 conejillo de indias"}))
}

async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, Html(include_str!("../templates/404.html"))).into_response()
}
