use axum::{
    extract::Path,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use std::sync::Arc;
use tower_http::services::ServeDir;
use tower_http::compression::CompressionLayer;
use tracing_subscriber::EnvFilter;

mod blog;
mod templates;

pub struct AppState {
    pub tmpl: templates::TemplateEngine,
    pub blog_posts: Vec<blog::BlogPost>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let blog_posts = blog::load_blog_posts().unwrap_or_default();
    tracing::info!("Loaded {} blog posts", blog_posts.len());

    let state = Arc::new(AppState {
        tmpl: templates::TemplateEngine::new(),
        blog_posts,
    });

    let app = Router::new()
        // SSR pages (rendered by Rust)
        .route("/", get(root_page))
        .route("/index.html", get(root_page))
        .route("/portfolio", get(portfolio_page))
        .route("/portfolio/", get(portfolio_page))
        .route("/portfolio/index.html", get(portfolio_page))
        // Blog - SSR rendered listing + markdown posts
        .nest_service("/blog", ServeDir::new("blog"))
        .route("/blog/posts/{slug}", get(blog_post_handler))
        // Gallery & Socials - SSR pages at root, serve original static sub-pages
        .route("/gallery", get(gallery_page))
        .route("/gallery/", get(gallery_page))
        .route("/gallery/index.html", get(|| serve_file("gallery/index.html", "text/html; charset=utf-8")))
        .route("/socials", get(socials_page))
        .route("/socials/", get(socials_page))
        .route("/socials/index.html", get(|| serve_file("socials/index.html", "text/html; charset=utf-8")))
        // Static directories (avoid conflicts with SSR routes)
        .nest_service("/fonts", ServeDir::new("fonts"))
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/wasm", ServeDir::new("wasm-sim/pkg"))
        // Portfolio sub-pages & assets (individual routes to avoid SSR conflicts)
        .route("/portfolio/styles.css", get(|| serve_file("portfolio/styles.css", "text/css")))
        .route("/portfolio/crt.css", get(|| serve_file("portfolio/crt.css", "text/css")))
        .route("/portfolio/error.css", get(|| serve_file("portfolio/error.css", "text/css")))
        .route("/portfolio/error.html", get(|| serve_file("portfolio/error.html", "text/html; charset=utf-8")))
        .route("/portfolio/script.js", get(|| serve_file("portfolio/script.js", "application/javascript")))
        .route("/portfolio/ghost-typist.js", get(|| serve_file("portfolio/ghost-typist.js", "application/javascript")))
        .route("/portfolio/liquid-text.html", get(|| serve_file("portfolio/liquid-text.html", "text/html; charset=utf-8")))
        .route("/portfolio/globe.html", get(|| serve_file("portfolio/globe.html", "text/html; charset=utf-8")))
        .route("/portfolio/pilots.html", get(|| serve_file("portfolio/pilots.html", "text/html; charset=utf-8")))
        .route("/portfolio/ascii_portrait.html", get(|| serve_file("portfolio/ascii_portrait.html", "text/html; charset=utf-8")))
        .route("/portfolio/ascii-cam.html", get(|| serve_file("portfolio/ascii-cam.html", "text/html; charset=utf-8")))
        // Root-level static files
        .route("/water-sim.js", get(|| serve_file("water-sim.js", "application/javascript")))
        .route("/script.js", get(|| serve_file("script.js", "application/javascript")))
        .route("/styles.css", get(|| serve_file("styles.css", "text/css")))
        .route("/crt.css", get(|| serve_file("crt.css", "text/css")))
        .route("/error.css", get(|| serve_file("error.css", "text/css")))
        .route("/error.html", get(|| serve_file("error.html", "text/html; charset=utf-8")))
        .route("/globe.html", get(|| serve_file("globe.html", "text/html; charset=utf-8")))
        .route("/liquid-text.html", get(|| serve_file("liquid-text.html", "text/html; charset=utf-8")))
        .route("/pilots.html", get(|| serve_file("pilots.html", "text/html; charset=utf-8")))
        .route("/ghost-typist.js", get(|| serve_file("ghost-typist.js", "application/javascript")))
        .route("/GlassBlock.png", get(|| serve_file("GlassBlock.png", "image/png")))
        .route("/GlassBlock4K.png", get(|| serve_file("GlassBlock4K.png", "image/png")))
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

async fn portfolio_page(axum::extract::State(state): axum::extract::State<Arc<AppState>>) -> Response {
    state.tmpl.render_response("portfolio.html", &serde_json::json!({"title": "SYSTEM"}))
}

async fn gallery_page(axum::extract::State(state): axum::extract::State<Arc<AppState>>) -> Response {
    state.tmpl.render_response("gallery.html", &serde_json::json!({"title": "Gallery — Alice"}))
}

async fn socials_page(axum::extract::State(state): axum::extract::State<Arc<AppState>>) -> Response {
    state.tmpl.render_response("socials.html", &serde_json::json!({"title": "Socials — Alice"}))
}

async fn blog_post_handler(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Response {
    if let Some(post) = state.blog_posts.iter().find(|p| p.slug == slug) {
        let content_html = blog::render_markdown(&post.content);
        state.tmpl.render_response("blog_post.html", &serde_json::json!({
            "title": format!("{} — Alice", post.title),
            "post": post,
            "content_html": content_html,
        }))
    } else {
        not_found().await
    }
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

async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, Html(include_str!("../templates/404.html"))).into_response()
}
