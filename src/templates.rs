use axum::response::{Html, IntoResponse, Response};
use minijinja::Environment;
use serde::Serialize;

pub struct TemplateEngine {
    env: Environment<'static>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        let mut env = Environment::new();
        env.add_template("index.html", include_str!("../templates/index.html")).unwrap();
        env.add_template("projects.html", include_str!("../templates/projects.html")).unwrap();
        env.add_template("redherring.html", include_str!("../templates/redherring.html")).unwrap();
        env.add_template("conejillo.html", include_str!("../templates/conejillo.html")).unwrap();
        // Gallery / socials used to be served as static files behind a
        // placeholder SSR stub that iframed them. They are rendered directly now,
        // so the profile's name reaches them and nothing is nested twice.
        env.add_template("content_gallery.html", include_str!("../gallery/index.html")).unwrap();
        env.add_template("content_socials.html", include_str!("../socials/index.html")).unwrap();
        // The blog is a WriteFreely instance on this host that refuses to be
        // framed from here, so its feeds are rendered through this template
        // instead — see `crate::blog`.
        env.add_template("blog.html", include_str!("../templates/blog.html")).unwrap();
        TemplateEngine { env }
    }

    pub fn render(&self, name: &str, ctx: &impl Serialize) -> String {
        self.env.get_template(name).unwrap().render(ctx).unwrap()
    }

    pub fn render_response(&self, name: &str, ctx: &impl Serialize) -> Response {
        match self.env.get_template(name).unwrap().render(ctx) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Template error: {}", e);
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::site::Site;

    fn home(site: Site) -> String {
        TemplateEngine::new().render(
            "index.html",
            &serde_json::json!({ "title": "Home", "site": site }),
        )
    }

    /// minijinja escapes `/` as `&#x2f;` inside attribute values. That is valid
    /// HTML — the parser Mastodon crawls with decodes it, so the link works —
    /// but it hides URLs from naive string assertions, hence the round trip.
    fn decode_escapes(html: &str) -> String {
        html.replace("&#x2f;", "/")
    }

    /// Mastodon decides whether to show a profile link as verified by fetching
    /// the page and looking for `<a rel="me">`. Its crawler does not run
    /// JavaScript, so the link has to be in the HTML the server sends.
    #[test]
    fn the_home_page_links_back_to_mastodon_with_rel_me() {
        let html = decode_escapes(&home(Site::personal()));
        assert!(html.contains("href=\"https://mastodon.social/@alicemow\""));
        assert!(html.contains("rel=\"me"));
    }

    #[test]
    fn the_professional_home_page_shows_no_personal_accounts() {
        let html = decode_escapes(&home(Site::professional()));
        assert!(!html.contains("mastodon.social"));
        assert!(!html.contains("x.com/Alice_mow"));
        assert!(!html.contains("discord.gg"));
        // GitHub stays: it is the same identity on both profiles.
        assert!(html.contains("https://github.com/peanutmow"));
    }
}
