use axum::{
    extract::{Extension, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use std::sync::Arc;
use tower_http::services::ServeDir;
use tower_http::compression::CompressionLayer;
use tracing_subscriber::EnvFilter;

mod projects;
mod site;
mod templates;

use site::Site;

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
        // Blog — profile-aware index, static post files underneath
        .route("/blog", get(|| async { Redirect::permanent("/blog/") }))
        .route("/blog/", get(blog_page))
        .nest_service("/blog/posts", ServeDir::new("blog/posts"))
        // Gallery & Socials — the real pages, rendered per profile. These used to
        // be a placeholder stub that iframed the content page, which stacked a
        // second scroll area and dumped unstyled debug text above the artwork.
        .route("/gallery", get(gallery_content_page))
        .route("/gallery/", get(gallery_content_page))
        .route("/gallery/index.html", get(gallery_content_page))
        .route("/socials", get(socials_content_page))
        .route("/socials/", get(socials_content_page))
        .route("/socials/index.html", get(socials_content_page))
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
        // Resolve the site identity (personal vs professional) for every request.
        .layer(middleware::from_fn(resolve_site))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    tracing::info!("Server running on http://0.0.0.0:8080");
    axum::serve(listener, app).await.unwrap();
}

// ─── Site resolution ───────────────────────────────────────────────

/// Attach the identity (personal vs professional) to every request so handlers
/// can render the matching profile. See [`site::detect`].
async fn resolve_site(mut req: Request, next: Next) -> Response {
    let site = site::detect(req.headers());
    req.extensions_mut().insert(site);
    next.run(req).await
}

// ─── SSR Handlers ──────────────────────────────────────────────────

async fn root_page(State(state): State<Arc<AppState>>, Extension(site): Extension<Site>) -> Response {
    let title = format!("{} Portfolio", site.full_name);
    state.tmpl.render_response("index.html", &serde_json::json!({ "title": title, "site": site }))
}

async fn projects_page(State(state): State<Arc<AppState>>, Extension(site): Extension<Site>) -> Response {
    let live_count = state
        .projects
        .iter()
        .filter(|p| p.section == "featured" || p.section == "work")
        .count();
    let dream_count = state.dreams.len();
    let title = format!("Projects — {}", site.name);
    state.tmpl.render_response("projects.html", &serde_json::json!({
        "title": title,
        "site": site,
        "projects": state.projects,
        "dreams": state.dreams,
        "live_count": live_count,
        "dream_count": dream_count,
    }))
}

// ─── Content pages (rendered so the profile's name reaches them) ───

async fn gallery_content_page(State(state): State<Arc<AppState>>, Extension(site): Extension<Site>) -> Response {
    state.tmpl.render_response("content_gallery.html", &serde_json::json!({ "site": site }))
}

async fn socials_content_page(State(state): State<Arc<AppState>>, Extension(site): Extension<Site>) -> Response {
    state.tmpl.render_response("content_socials.html", &serde_json::json!({ "site": site }))
}

async fn blog_page(State(state): State<Arc<AppState>>, Extension(site): Extension<Site>) -> Response {
    state.tmpl.render_response("content_blog.html", &serde_json::json!({ "site": site }))
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

async fn redherring_page(State(state): State<Arc<AppState>>, Extension(site): Extension<Site>) -> Response {
    if site.professional {
        return not_found().await;
    }
    state.tmpl.render_response("redherring.html", &serde_json::json!({"title": "///", "site": site}))
}

async fn conejillo_page(State(state): State<Arc<AppState>>, Extension(site): Extension<Site>) -> Response {
    if site.professional {
        return not_found().await;
    }
    state.tmpl.render_response("conejillo.html", &serde_json::json!({"title": "🐰 conejillo de indias", "site": site}))
}

async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, Html(include_str!("../templates/404.html"))).into_response()
}
