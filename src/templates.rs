use axum::response::{Html, IntoResponse, Response};
use minijinja::Environment;
use serde::Serialize;

const TEMPLATES: &[(&str, &str, &str)] = &[
    ("index.html", "templates/index.html", include_str!("../templates/index.html")),
    ("projects.html", "templates/projects.html", include_str!("../templates/projects.html")),
    ("project.html", "templates/project.html", include_str!("../templates/project.html")),
    ("about.html", "templates/about.html", include_str!("../templates/about.html")),
    ("faq.html", "templates/faq.html", include_str!("../templates/faq.html")),
    ("redherring.html", "templates/redherring.html", include_str!("../templates/redherring.html")),
    ("conejillo.html", "templates/conejillo.html", include_str!("../templates/conejillo.html")),
    ("content_gallery.html", "gallery/index.html", include_str!("../gallery/index.html")),
    ("content_socials.html", "socials/index.html", include_str!("../socials/index.html")),
    ("music.html", "templates/music.html", include_str!("../templates/music.html")),
    ("blog.html", "templates/blog.html", include_str!("../templates/blog.html")),
    ("meta.html", "templates/meta.html", include_str!("../templates/meta.html")),
    ("site_footer.html", "templates/site_footer.html", include_str!("../templates/site_footer.html")),
    ("robots.txt", "templates/robots.txt", include_str!("../templates/robots.txt")),
    ("llms.txt", "templates/llms.txt", include_str!("../templates/llms.txt")),
    ("llms-full.txt", "templates/llms-full.txt", include_str!("../templates/llms-full.txt")),
    ("embed_scene.html", "templates/embed_scene.html", include_str!("../templates/embed_scene.html")),
];

fn read_template(path: &str, embedded: &'static str) -> String {
    match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            tracing::warn!("could not read {path} ({error}); using the embedded copy");
            embedded.to_string()
        }
    }
}

pub struct TemplateEngine {
    env: Environment<'static>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        let mut env = Environment::new();
        for &(name, path, embedded) in TEMPLATES {
            env.add_template_owned(name, read_template(path, embedded)).unwrap();
        }
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

    fn project_page(site: Site) -> String {
        let projects = crate::projects::load_projects();
        let project = &projects[0];
        TemplateEngine::new().render(
            "project.html",
            &serde_json::json!({
                "title": project.title,
                "site": site,
                "meta": crate::embed::project(&site, "https://example.test", project),
                "project": project,
                "index": 1,
                "count": projects.len(),
                "prev": serde_json::Value::Null,
                "next": projects.get(1),
            }),
        )
    }

    fn about_page(site: Site) -> String {
        TemplateEngine::new().render(
            "about.html",
            &serde_json::json!({
                "title": "About",
                "site": site,
                "meta": crate::embed::about(&site, "https://example.test"),
                "projects": crate::projects::load_projects(),
            }),
        )
    }

    fn faq_page(site: Site) -> String {
        let faqs = crate::content::faq(&site);
        TemplateEngine::new().render(
            "faq.html",
            &serde_json::json!({
                "title": "FAQ",
                "site": site,
                "meta": crate::embed::faq(&site, "https://example.test"),
                "faqs": faqs,
                "faq_json_ld": crate::content::faq_json_ld(&faqs),
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

    fn music_page(site: Site) -> String {
        TemplateEngine::new().render(
            "music.html",
            &serde_json::json!({
                "title": "Music",
                "site": site,
                "meta": crate::embed::music(&site, "https://example.test"),
            }),
        )
    }

    #[test]
    fn the_projects_page_links_each_work_to_its_own_page() {
        let html = decode_escapes(&projects_page(Site::personal()));
        assert!(html.contains("href=\"/projects/ascii-render\""));
        assert!(html.contains("href=\"/projects/polyaxis\""));
        assert!(html.contains("href=\"/projects/this-website\""));
        assert!(html.contains("href=\"/projects/marx-knowledge-base\""));
    }

    #[test]
    fn a_project_page_carries_its_specimen_and_notes() {
        let html = project_page(Site::personal());
        assert!(html.contains("specimen"));
        assert!(html.contains("65,535"));
        assert!(html.contains("the limits"));
    }

    /// Every page that can be pasted into Discord has to include the shared
    /// head block; one missing include means a bare link.
    #[test]
    fn every_page_template_includes_the_preview() {
        for site in [Site::personal(), Site::professional()] {
            let mut pages = vec![
                ("home", home(site)),
                ("projects", projects_page(site)),
                ("project", project_page(site)),
                ("about", about_page(site)),
                ("faq", faq_page(site)),
                ("gallery", gallery_page(site)),
                ("socials", socials_page(site)),
                ("blog", blog_page(site)),
            ];
            if !site.professional {
                pages.push(("music", music_page(site)));
            }
            for (name, html) in pages {
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
                assert!(
                    html.contains("rel=\"alternate\" type=\"text/plain\" href=\"/llms.txt\""),
                    "{name} has no llms.txt discovery"
                );
            }
        }
    }

    #[test]
    fn the_music_page_embeds_the_playlist_only_on_the_personal_site() {
        let personal = music_page(Site::personal());
        assert!(personal.contains("open.spotify.com/embed/playlist/4GcHDrtPr1Ok3PGVWHx5Jq"));
        assert!(personal.contains("height=\"352\""));
        assert!(personal.contains("href=\"https://open.spotify.com/playlist/4GcHDrtPr1Ok3PGVWHx5Jq\""));
        assert!(personal.contains("href=\"/music\""));

        let professional = music_page(Site::professional());
        assert!(!professional.contains("open.spotify.com/embed"));
        assert!(!professional.contains("open.spotify.com/playlist"));
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

    #[test]
    fn the_home_page_describes_the_person_for_search_engines() {
        let html = home(Site::personal());
        assert!(html.contains("application/ld+json"));
        assert!(html.contains("\"alternateName\":\"alicemow\""));
        assert!(html.contains("https://mastodon.social/@alicemow"));
        assert!(html.contains("index, follow, max-image-preview:large"));
    }

    #[test]
    fn the_professional_schema_has_no_personal_handle() {
        let html = home(Site::professional());
        assert!(html.contains("application/ld+json"));
        assert!(!html.contains("alternateName"));
        assert!(!html.contains("alicemow"));
    }

    #[test]
    fn the_home_page_still_names_itself_without_javascript() {
        let html = decode_escapes(&home(Site::personal()));
        assert!(html.contains("<noscript>"));
        assert!(html.contains("Alice — alicemow"));
        assert!(html.contains("@alicemow@mastodon.social"));
    }

    #[test]
    fn robots_points_crawlers_at_the_canonical_sitemap() {
        let body = TemplateEngine::new().render(
            "robots.txt",
            &serde_json::json!({ "origin": "https://alicemow.org" }),
        );
        assert!(body.contains("Allow: /"));
        assert!(body.contains("Sitemap: https://alicemow.org/sitemap.xml"));
        assert!(body.contains("Disallow: /oembed.json"));
        assert!(!body.contains("&#x2f;"));
    }

    #[test]
    fn robots_opted_into_ai_crawling() {
        let body = TemplateEngine::new().render(
            "robots.txt",
            &serde_json::json!({ "origin": "https://alicemow.org" }),
        );
        assert!(body.contains("Content-Signal: search=yes, ai-input=yes, ai-train=yes"));
        for agent in [
            "GPTBot",
            "OAI-SearchBot",
            "ChatGPT-User",
            "ClaudeBot",
            "PerplexityBot",
            "Google-Extended",
            "Applebot-Extended",
            "CCBot",
        ] {
            assert!(body.contains(&format!("User-agent: {agent}")), "missing {agent}");
        }
    }

    fn llms_page(site: Site) -> String {
        TemplateEngine::new().render(
            "llms.txt",
            &serde_json::json!({ "site": site, "origin": "https://example.test" }),
        )
    }

    #[test]
    fn the_personal_llm_summary_names_the_handle_and_links() {
        let text = llms_page(Site::personal());
        assert!(text.starts_with("# Alice (alicemow)"));
        assert!(text.contains("https://example.test/projects"));
        assert!(text.contains("https://mastodon.social/@alicemow"));
        assert!(text.contains("https://blog.alicemow.org/alice/"));
        assert!(text.contains("https://blog.alicemow.org/politics/"));
        assert!(text.contains("BunGPT"));
    }

    #[test]
    fn the_professional_llm_summary_hides_personal_accounts() {
        let text = llms_page(Site::professional());
        assert!(text.starts_with("# Dave Herkula"));
        assert!(!text.contains("alicemow"));
        assert!(!text.contains("mastodon"));
        assert!(!text.contains("blog."));
        assert!(!text.contains("BunGPT"));
    }

    fn llms_full_page(site: Site) -> String {
        let projects = crate::projects::load_projects();
        let dreams = crate::projects::load_dreams();
        TemplateEngine::new().render(
            "llms-full.txt",
            &serde_json::json!({
                "site": site,
                "origin": "https://example.test",
                "projects": projects,
                "dreams": dreams,
            }),
        )
    }

    #[test]
    fn the_full_llm_summary_lists_the_projects() {
        let text = llms_full_page(Site::personal());
        for title in ["ascii-render", "polyaxis", "marx-knowledge-base"] {
            assert!(text.contains(title), "missing {title}");
        }
        assert!(text.contains("JavaScript, 3D, ASCII"));
        assert!(text.contains("https://blog.alicemow.org/alice/"));
        assert!(text.contains("https://example.test/sitemap.xml"));
    }

    #[test]
    fn the_about_page_is_profile_aware() {
        let personal = decode_escapes(&about_page(Site::personal()));
        assert!(personal.contains("(alicemow)"));
        assert!(personal.contains("ascii-render"));
        assert!(personal.contains("alice@herkula.info"));
        assert!(personal.contains("https://blog.alicemow.org/alice/"));
        assert!(personal.contains("https://blog.alicemow.org/politics/"));

        let professional = decode_escapes(&about_page(Site::professional()));
        assert!(!professional.contains("alicemow"));
        assert!(professional.contains("dave@herkula.info"));
    }

    #[test]
    fn the_faq_page_carries_faq_structured_data() {
        let html = decode_escapes(&faq_page(Site::personal()));
        assert!(html.contains("\"@type\":\"FAQPage\""));
        assert!(html.contains("Who is Alice?"));
        assert!(html.contains("BunGPT"));
        assert!(html.contains("href=\"https://github.com/peanutmow/ascii-render\""));
        assert!(html.contains("href=\"https://polyaxis.link/\""));
        assert!(html.contains("href=\"/projects\""));

        let professional = faq_page(Site::professional());
        assert!(!professional.contains("alicemow"));
        assert!(!professional.contains("mastodon"));
    }
}
