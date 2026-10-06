use axum::{
    extract::{Extension, Request, State},
    http::{StatusCode, Uri},
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
        // Blog — published on Bear Blog (see `Site::blog_url`), so these routes
        // forward readers there instead of serving anything locally.
        .route("/blog", get(blog_redirect))
        .route("/blog/", get(blog_redirect))
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

/// Forward `/blog` to the externally hosted blog. The professional mirror has
/// no `blog_url`, so it goes home rather than advertising personal writing.
async fn blog_redirect(Extension(site): Extension<Site>) -> Redirect {
    Redirect::permanent(site.blog_url.unwrap_or("/"))
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
        return page_404();
    }
    state.tmpl.render_response("redherring.html", &serde_json::json!({"title": "///", "site": site}))
}

async fn conejillo_page(State(state): State<Arc<AppState>>, Extension(site): Extension<Site>) -> Response {
    if site.professional {
        return page_404();
    }
    state.tmpl.render_response("conejillo.html", &serde_json::json!({"title": "🐰 conejillo de indias", "site": site}))
}

/// Fallback for anything no route matched: hand back a root-level image if one
/// exists under that name (webring buttons, favicons, stray art), else the 404
/// page. The deploy mirrors the repo root to the server, so dropping a file in
/// the root is enough to make it reachable.
async fn not_found(uri: Uri) -> Response {
    if let Some((file, mime)) = root_image(uri.path()) {
        if let Ok(data) = tokio::fs::read(&file).await {
            return Response::builder()
                .header("Content-Type", mime)
                .body(axum::body::Body::from(data))
                .unwrap();
        }
    }
    page_404()
}

fn page_404() -> Response {
    (StatusCode::NOT_FOUND, Html(include_str!("../templates/404.html"))).into_response()
}

/// Map a request path to a root-level image, e.g. `/webring-button.png`.
///
/// Only a single path segment, no dotfiles and no `..`, and only image
/// extensions qualify — so this can never reach `Cargo.toml`, `src/main.rs`,
/// `deploy/deploy.sh` or anything else that also lives in the deployed tree.
fn root_image(path: &str) -> Option<(String, &'static str)> {
    let name = path.strip_prefix('/')?;
    if name.is_empty() || name.contains('/') || name.starts_with('.') || name.contains("..") {
        return None;
    }
    let ext = name.rsplit_once('.')?.1.to_ascii_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        _ => return None,
    };
    Some((name.to_string(), mime))
}

#[cfg(test)]
mod tests {
    use super::root_image;

    fn served(path: &str) -> Option<String> {
        root_image(path).map(|(file, _)| file)
    }

    #[test]
    fn root_images_are_served() {
        assert_eq!(served("/stupidwebbuttonthingykys.png").as_deref(), Some("stupidwebbuttonthingykys.png"));
        assert_eq!(served("/icon.SVG").as_deref(), Some("icon.SVG"));
    }

    #[test]
    fn non_images_and_unsafe_paths_are_refused() {
        // Source and config files share the deployed tree — must never be served.
        for path in ["/Cargo.toml", "/main.rs", "/deploy/deploy.sh", "/index.html", "/water-sim.js"] {
            assert!(root_image(path).is_none(), "{path} should not be served");
        }
        // No traversal, no dotfiles, no nested paths, no bare root.
        for path in ["/", "/.gitignore", "/.hidden.png", "/../secret.png", "/a/b.png", "nope.png"] {
            assert!(root_image(path).is_none(), "{path} should not be served");
        }
    }
}
