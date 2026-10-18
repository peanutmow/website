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
        // Shared <head> block for link previews (OpenGraph + Discord's
        // component-embed payload). Kept in one place so every page emits the
        // same tags in the same order.
        env.add_template("meta.html", include_str!("../templates/meta.html")).unwrap();
        // The fixed-size surface the Discord artwork is recorded from.
        env.add_template("embed_scene.html", include_str!("../templates/embed_scene.html")).unwrap();
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
            &serde_json::json!({
                "title": "Home",
                "site": site,
                "meta": crate::embed::home(&site, "https://example.test"),
            }),
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
    fn the_home_page_credits_its_author_to_the_fediverse() {
        let html = home(Site::personal());
        assert!(html.contains("name=\"fediverse:creator\""));
        assert!(html.contains("content=\"@alicemow@mastodon.social\""));
    }

    /// The guided tour lives in the ASCII menu. Both profiles have to offer it,
    /// and the layout array has to know about it or its click target drifts
    /// away from the word drawn in the water.
    #[test]
    fn the_home_page_offers_the_guided_tour() {
        for site in [Site::personal(), Site::professional()] {
            let html = home(site);
            assert!(html.contains("data-page=\"how\""));
            assert!(html.contains("'How'"));
        }
    }

    /// The home page is what gets pasted into Discord, so it has to carry both
    /// halves of the preview: the OpenGraph fallback and the component payload.
    #[test]
    fn the_home_page_carries_a_discord_preview() {
        // minijinja escapes `/` inside attribute values; the component payload
        // is injected raw.
        let html = decode_escapes(&home(Site::personal()));
        assert!(html.contains("property=\"og:title\""));
        assert!(html.contains(
            "property=\"og:image\" content=\"https://example.test/static/embed/water.gif"
        ));
        assert!(html.contains("name=\"theme-color\" content=\"#0a0a0a\""));
        assert!(html.contains("twitter:card\" content=\"summary_large_image\""));
        assert!(html.contains("application/json+oembed"));
        assert!(html.contains("id=\"discord:component-embed\" type=\"application/json\""));
        // The payload is emitted raw, not HTML-escaped.
        assert!(html.contains("\"type\":17"));
        // ...and it is inside <head>, where the crawler looks for it.
        let head_end = html.find("</head>").expect("the page has a head");
        let payload = html
            .find("discord:component-embed")
            .expect("the payload is present");
        assert!(payload < head_end);
    }

    /// The artwork is the water alone, so it names nobody and both profiles
    /// point at the same file.
    #[test]
    fn the_preview_artwork_is_text_free_and_shared() {
        let personal = decode_escapes(&home(Site::personal()));
        let professional = decode_escapes(&home(Site::professional()));
        assert!(personal.contains("/static/embed/water.gif"));
        assert!(professional.contains("/static/embed/water.gif"));
        // The greeting still belongs to the text, not the picture.
        assert!(!personal.contains("water-alice"));
    }

    #[test]
    fn the_professional_home_page_shows_no_personal_accounts() {
        let html = decode_escapes(&home(Site::professional()));
        assert!(!html.contains("mastodon.social"));
        assert!(!html.contains("fediverse:creator"));
        assert!(!html.contains("x.com/Alice_mow"));
        assert!(!html.contains("discord.gg"));
        // GitHub stays: it is the same identity on both profiles.
        assert!(html.contains("https://github.com/peanutmow"));
    }

    fn blog_page(site: Site) -> String {
        let section = crate::blog::Section {
            label: "Main".to_string(),
            slug: "alice".to_string(),
            url: "/blog".to_string(),
            feed: "https://blog.alicemow.org/alice/feed/".to_string(),
            posts: Vec::new(),
        };
        TemplateEngine::new().render(
            "blog.html",
            &serde_json::json!({
                "title": "Main",
                "site": site,
                "section": section,
                "nav": [],
                "meta": crate::embed::blog(&site, "https://example.test", &section),
            }),
        )
    }

    fn projects_page(site: Site) -> String {
        let projects = crate::projects::load_projects();
        let dreams = crate::projects::load_dreams();
        let live_count = projects
            .iter()
            .filter(|p| p.section == "featured" || p.section == "work")
            .count();
        let dream_count = dreams.len();
        TemplateEngine::new().render(
            "projects.html",
            &serde_json::json!({
                "title": "Projects",
                "site": site,
                "meta": crate::embed::projects(&site, "https://example.test", &projects, live_count, dream_count),
                "projects": projects,
                "dreams": dreams,
                "live_count": live_count,
                "dream_count": dream_count,
            }),
        )
    }

    fn gallery_page(site: Site) -> String {
        TemplateEngine::new().render(
            "content_gallery.html",
            &serde_json::json!({
                "site": site,
                "meta": crate::embed::gallery(&site, "https://example.test"),
                "photos_json": crate::gallery::photos_json(),
            }),
        )
    }

    fn socials_page(site: Site) -> String {
        TemplateEngine::new().render(
            "content_socials.html",
            &serde_json::json!({
                "site": site,
                "meta": crate::embed::socials(&site, "https://example.test"),
            }),
        )
    }

    /// Every page that can be pasted into Discord has to include the shared
    /// head block; one missing include means a bare link.
    #[test]
    fn every_page_template_includes_the_preview() {
        for site in [Site::personal(), Site::professional()] {
            for (name, html) in [
                ("home", home(site)),
                ("projects", projects_page(site)),
                ("gallery", gallery_page(site)),
                ("socials", socials_page(site)),
                ("blog", blog_page(site)),
            ] {
                assert!(html.contains("property=\"og:title\""), "{name} has no OG title");
                assert!(html.contains("name=\"theme-color\""), "{name} has no accent");
                assert!(
                    html.contains("id=\"discord:component-embed\""),
                    "{name} has no component payload"
                );
                assert!(
                    html.contains("application/json+oembed"),
                    "{name} has no oEmbed discovery"
                );
            }
        }
    }

    /// Mastodon 4.3 credits the page author on link previews by reading
    /// `fediverse:creator` out of `<head>`. Its crawler runs no JavaScript, so
    /// the tag has to be in the HTML the server sends.
    #[test]
    fn the_blog_page_credits_its_author_to_the_fediverse() {
        let html = blog_page(Site::personal());
        assert!(html.contains("name=\"fediverse:creator\""));
        assert!(html.contains("@alicemow@mastodon.social"));
    }

    #[test]
    fn the_professional_mirror_credits_nobody() {
        // Naming Alice's account there would tie the two identities together.
        let html = blog_page(Site::professional());
        assert!(!html.contains("fediverse:creator"));
        assert!(!html.contains("mastodon.social"));
        assert!(!html.contains("rel=\"me\""));
    }

    /// Mastodon verifies a profile by finding a `rel="me"` link whose href
    /// matches the profile URL *exactly*, so both halves of this matter.
    #[test]
    fn the_blog_page_links_back_to_mastodon_for_verification() {
        let html = decode_escapes(&blog_page(Site::personal()));
        assert!(html.contains("rel=\"me\""));
        assert!(html.contains("href=\"https://mastodon.social/@alicemow\""));
    }
}
