use axum::{
    extract::Path,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use std::sync::Arc;
use tower_http::services::ServeDir;
use tower_http::compression::CompressionLayer;
use tracing_subscriber::EnvFilter;

mod blog;
mod projects;
mod templates;

pub struct AppState {
    pub tmpl: templates::TemplateEngine,
    pub blog_posts: Vec<blog::BlogPost>,
    pub projects: Vec<projects::Project>,
    pub dreams: Vec<projects::Project>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let blog_posts = blog::load_blog_posts().unwrap_or_default();
    tracing::info!("Loaded {} blog posts", blog_posts.len());

    let projects = projects::load_projects();
    tracing::info!("Loaded {} projects", projects.len());

    let dreams = projects::load_dreams();
    tracing::info!("Loaded {} dreams", dreams.len());

    let state = Arc::new(AppState {
        tmpl: templates::TemplateEngine::new(),
        blog_posts,
        projects,
        dreams,
    });

    let app = Router::new()
        // SSR pages (rendered by Rust)
        .route("/", get(root_page))
        .route("/index.html", get(root_page))
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
        .route("/GlassBlock.png", get(|| serve_file("GlassBlock.png", "image/png")))
        .route("/GlassBlock4K.png", get(|| serve_file("GlassBlock4K.png", "image/png")))
        .route("/qr-error.png", get(|| serve_file("templates/QRCode(3).png", "image/png")))
        // Easter egg pages
        .route("/etc/hosts", get(etchosts_page))
        .route("/dev/null", get(dev_null_redirect))
        .route("/changelog", get(changelog_page))
        .route("/man", get(man_page))
        .route("/a", get(aaencode_page))
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

// ─── Easter egg handlers ───────────────────────────────────────────

async fn aaencode_page() -> Response {
    match tokio::fs::read_to_string("codes/2.txt").await {
        Ok(content) => {
            let escaped = content
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            let html = format!(
                r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>///</title>
<link href="https://fonts.googleapis.com/css2?family=Share+Tech+Mono&display=swap" rel="stylesheet">
<style>
*{{margin:0;padding:0;box-sizing:border-box;}}
html,body{{width:100%;min-height:100%;background:#000;color:rgba(255,255,255,0.15);}}
body{{padding:2rem;font-family:'Share Tech Mono',monospace;font-size:0.5rem;line-height:1.1;word-break:break-all;}}
</style></head><body><pre style="white-space:pre-wrap;word-break:break-all;">{}</pre></body></html>"#,
                escaped
            );
            Response::builder()
                .header("Content-Type", "text/html; charset=utf-8")
                .body(axum::body::Body::from(html))
                .unwrap()
        }
        Err(_) => not_found().await,
    }
}

async fn etchosts_page(axum::extract::State(state): axum::extract::State<Arc<AppState>>) -> Response {
    state.tmpl.render_response("etchosts.html", &serde_json::json!({"title": "/etc/hosts — Alice"}))
}

async fn dev_null_redirect() -> Redirect {
    Redirect::to("/")
}

async fn changelog_page(axum::extract::State(state): axum::extract::State<Arc<AppState>>) -> Response {
    state.tmpl.render_response("changelog.html", &serde_json::json!({"title": "CHANGELOG — Alice"}))
}

async fn man_page(axum::extract::State(state): axum::extract::State<Arc<AppState>>) -> Response {
    state.tmpl.render_response("man.html", &serde_json::json!({"title": "alice(1) — Manual"}))
}

async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, Html(include_str!("../templates/404.html"))).into_response()
}
