use axum::{
    extract::{Extension, Path, Request, State},
    http::{header, HeaderValue, StatusCode, Uri},
    middleware::{self, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use std::sync::Arc;
use tower::Layer;
use tower_http::services::ServeDir;
use tower_http::compression::CompressionLayer;
use tower_http::set_header::{SetResponseHeader, SetResponseHeaderLayer};
use tracing_subscriber::EnvFilter;

mod blog;
mod content;
mod embed;
mod gallery;
mod projects;
mod seo;
mod site;
mod templates;

use site::Site;

pub struct AppState {
    pub tmpl: templates::TemplateEngine,
    pub projects: Vec<projects::Project>,
    pub dreams: Vec<projects::Project>,
    pub blog: blog::Cache,
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
        blog: blog::Cache::new(),
    });

    let app = Router::new()
        // SSR pages (rendered by Rust)
        .route("/", get(root_page))
        .route("/index.html", get(root_page))
        // Blog — a self-hosted WriteFreely instance, read and rendered by
        // `crate::blog`. These paths are same-origin so the content panel can
        // frame them; the blog's own domain sends `X-Frame-Options: SAMEORIGIN`
        // and cannot be framed at all.
        .route("/blog", get(blog_page))
        .route("/blog/", get(blog_page))
        .route("/blog/:slug", get(blog_section_page))
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
        .route("/about", get(about_page))
        .route("/about/", get(about_page))
        .route("/faq", get(faq_page))
        .route("/faq/", get(faq_page))
        // Static directories (avoid conflicts with SSR routes). Cache-Control
        // keeps repeat navigation from re-downloading artwork and scripts.
        .nest_service("/fonts", cached(ServeDir::new("fonts"), "public, max-age=31536000, immutable"))
        .nest_service("/static", cached(ServeDir::new("static"), "public, max-age=3600"))
        .nest_service("/assets", cached(ServeDir::new("assets"), "public, max-age=604800"))
        .nest_service("/wasm", cached(ServeDir::new("wasm-sim/pkg"), "public, max-age=86400"))
        // Root-level static files
        .route("/water-sim.js", get(|| serve_file("water-sim.js", "application/javascript")))
        .route("/qr-error.png", get(|| serve_file("templates/QRCode(3).png", "image/png")))
        // Easter egg pages
        .route("/dev/null", get(dev_null_redirect))
        .route("/redherring", get(redherring_page))
        // Link previews: the fixed-size surface the Discord artwork is
        // recorded from, and the oEmbed document that credits the author.
        .route("/embed-scene", get(embed_scene))
        .route("/oembed.json", get(oembed))
        .route("/robots.txt", get(robots_txt))
        .route("/sitemap.xml", get(sitemap_xml))
        .route("/llms.txt", get(llms_txt))
        .route("/llms-full.txt", get(llms_full_txt))
        .route("/llm.txt", get(llm_txt_redirect))
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

    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8080);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await.unwrap();
    tracing::info!("Server running on http://0.0.0.0:{port}");
    axum::serve(listener, app).await.unwrap();
}

// ─── Site resolution ───────────────────────────────────────────────

/// Attach the identity (personal vs professional) to every request so handlers
/// can render the matching profile. See [`site::detect`].
async fn resolve_site(mut req: Request, next: Next) -> Response {
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|value| value.as_str())
        .unwrap_or("/");
    if let Some(target) = site::canonical_redirect(req.headers(), path_and_query) {
        return Redirect::permanent(&target).into_response();
    }
    let site = site::detect(req.headers());
    let origin = site::origin(req.headers());
    req.extensions_mut().insert(site);
    req.extensions_mut().insert(origin);
    next.run(req).await
}

// ─── SSR Handlers ──────────────────────────────────────────────────

async fn root_page(
    State(state): State<Arc<AppState>>,
    Extension(site): Extension<Site>,
    Extension(origin): Extension<site::Origin>,
) -> Response {
    let title = match site.handle {
        Some(handle) => format!("{} ({}) — {}", site.full_name, handle, site.bio),
        None => format!("{} Portfolio", site.full_name),
    };
    let meta = embed::home(&site, &origin.0);
    state.tmpl.render_response("index.html", &serde_json::json!({
        "title": title,
        "site": site,
        "meta": meta,
    }))
}

async fn projects_page(
    State(state): State<Arc<AppState>>,
    Extension(site): Extension<Site>,
    Extension(origin): Extension<site::Origin>,
) -> Response {
    let live_count = state
        .projects
        .iter()
        .filter(|p| p.section == "featured" || p.section == "work")
        .count();
    let dream_count = state.dreams.len();
    let title = format!("Projects — {}", site.name);
    let meta = embed::projects(
        &site,
        &origin.0,
        &state.projects,
        live_count,
        dream_count,
    );
    state.tmpl.render_response("projects.html", &serde_json::json!({
        "title": title,
        "site": site,
        "meta": meta,
        "projects": state.projects,
        "dreams": state.dreams,
        "live_count": live_count,
        "dream_count": dream_count,
    }))
}

// ─── Content pages (rendered so the profile's name reaches them) ───

async fn about_page(
    State(state): State<Arc<AppState>>,
    Extension(site): Extension<Site>,
    Extension(origin): Extension<site::Origin>,
) -> Response {
    let title = format!("About — {}", site.full_name);
    let meta = embed::about(&site, &origin.0);
    state.tmpl.render_response("about.html", &serde_json::json!({
        "title": title,
        "site": site,
        "meta": meta,
        "projects": &state.projects,
    }))
}

async fn faq_page(
    State(state): State<Arc<AppState>>,
    Extension(site): Extension<Site>,
    Extension(origin): Extension<site::Origin>,
) -> Response {
    let entries = content::faq(&site);
    let title = format!("FAQ — {}", site.full_name);
    let meta = embed::faq(&site, &origin.0);
    state.tmpl.render_response("faq.html", &serde_json::json!({
        "title": title,
        "site": site,
        "meta": meta,
        "faqs": &entries,
        "faq_json_ld": content::faq_json_ld(&entries),
    }))
}

async fn gallery_content_page(
    State(state): State<Arc<AppState>>,
    Extension(site): Extension<Site>,
    Extension(origin): Extension<site::Origin>,
) -> Response {
    let meta = embed::gallery(&site, &origin.0);
    state.tmpl.render_response("content_gallery.html", &serde_json::json!({
        "site": site,
        "meta": meta,
        "photos_json": gallery::photos_json(),
    }))
}

async fn socials_content_page(
    State(state): State<Arc<AppState>>,
    Extension(site): Extension<Site>,
    Extension(origin): Extension<site::Origin>,
) -> Response {
    let meta = embed::socials(&site, &origin.0);
    state.tmpl.render_response("content_socials.html", &serde_json::json!({
        "site": site,
        "meta": meta,
    }))
}

// ─── Blog ─────────────────────────────────────────────────────────

/// The first blog section. The switcher in the content panel sends each tab to
/// one of these paths.
async fn blog_page(
    State(state): State<Arc<AppState>>,
    Extension(site): Extension<Site>,
    Extension(origin): Extension<site::Origin>,
) -> Response {
    render_blog(&state, &site, &origin.0, 0).await
}

/// `/blog/<slug>` — the section with that slug, or nothing.
async fn blog_section_page(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    Extension(site): Extension<Site>,
    Extension(origin): Extension<site::Origin>,
) -> Response {
    match blog::index_of(&site, Some(&slug)) {
        Some(index) => render_blog(&state, &site, &origin.0, index).await,
        // Not a section we publish. The professional mirror lands here for any
        // slug, so it never advertises personal or political writing.
        None => page_404(),
    }
}

async fn render_blog(state: &AppState, site: &Site, origin: &str, index: usize) -> Response {
    let sections = state.blog.sections(site).await;
    let Some(section) = sections.get(index) else {
        return page_404();
    };

    let nav: Vec<serde_json::Value> = site
        .blogs
        .iter()
        .enumerate()
        .map(|(position, blog)| {
            serde_json::json!({
                "label": blog.label,
                "url": blog.url,
                "selected": position == index,
            })
        })
        .collect();

    state.tmpl.render_response("blog.html", &serde_json::json!({
        "title": format!("{} — {}", section.label, site.name),
        "site": site,
        "section": section,
        "nav": nav,
        "meta": embed::blog(site, origin, section),
    }))
}

// ─── File serving ──────────────────────────────────────────────────

fn cached<S>(service: S, cache_control: &'static str) -> SetResponseHeader<S, HeaderValue> {
    SetResponseHeaderLayer::overriding(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    )
    .layer(service)
}

async fn serve_file(path: &str, mime: &str) -> Response {
    match tokio::fs::read(path).await {
        Ok(data) => Response::builder()
            .header("Content-Type", mime)
            .header("Cache-Control", "public, max-age=3600")
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

// ─── Search engine files ──────────────────────────────────────────

async fn robots_txt(
    State(state): State<Arc<AppState>>,
    Extension(origin): Extension<site::Origin>,
) -> Response {
    let body = state
        .tmpl
        .render("robots.txt", &serde_json::json!({ "origin": origin.0 }));
    text_file(body)
}

async fn llms_txt(
    State(state): State<Arc<AppState>>,
    Extension(site): Extension<Site>,
    Extension(origin): Extension<site::Origin>,
) -> Response {
    let body = state.tmpl.render(
        "llms.txt",
        &serde_json::json!({ "site": site, "origin": origin.0 }),
    );
    text_file(body)
}

async fn llms_full_txt(
    State(state): State<Arc<AppState>>,
    Extension(site): Extension<Site>,
    Extension(origin): Extension<site::Origin>,
) -> Response {
    let body = state.tmpl.render(
        "llms-full.txt",
        &serde_json::json!({
            "site": site,
            "origin": origin.0,
            "projects": &state.projects,
            "dreams": &state.dreams,
        }),
    );
    text_file(body)
}

async fn llm_txt_redirect() -> Redirect {
    Redirect::permanent("/llms.txt")
}

fn text_file(body: String) -> Response {
    Response::builder()
        .header("Content-Type", "text/plain; charset=utf-8")
        .header("Cache-Control", "public, max-age=3600")
        .body(axum::body::Body::from(body))
        .unwrap()
}

async fn sitemap_xml(
    Extension(site): Extension<Site>,
    Extension(origin): Extension<site::Origin>,
) -> Response {
    Response::builder()
        .header("Content-Type", "application/xml; charset=utf-8")
        .header("Cache-Control", "public, max-age=3600")
        .body(axum::body::Body::from(seo::sitemap(&site, &origin.0)))
        .unwrap()
}

// ─── Link previews ────────────────────────────────────────────────

/// The fixed-size surface `tools/embed/record.mjs` records the Discord artwork
/// from. Nothing links here; it exists so the preview is generated from the
/// real simulator instead of a hand-drawn mock-up. It draws the water alone —
/// no greeting, no menu — because the artwork should be the simulation, not a
/// picture of the site.
async fn embed_scene(State(state): State<Arc<AppState>>) -> Response {
    state
        .tmpl
        .render_response("embed_scene.html", &serde_json::json!({}))
}

/// oEmbed fills the one field OpenGraph cannot: a linked author. Everything
/// else is already served by `embed::*`.
async fn oembed(
    Extension(site): Extension<Site>,
    Extension(origin): Extension<site::Origin>,
) -> Response {
    Response::builder()
        .header("Content-Type", "application/json")
        .header("Cache-Control", "public, max-age=1800")
        .body(axum::body::Body::from(
            embed::oembed(&site, &origin.0).to_string(),
        ))
        .unwrap()
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
                .header("Cache-Control", "public, max-age=604800")
                .body(axum::body::Body::from(data))
                .unwrap();
        }
    }
    page_404()
}

fn page_404() -> Response {
    let body = std::fs::read_to_string("templates/404.html")
        .unwrap_or_else(|_| include_str!("../templates/404.html").to_string());
    (StatusCode::NOT_FOUND, Html(body)).into_response()
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
