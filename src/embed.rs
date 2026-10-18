//! Discord link previews.
//!
//! Discord unfurls a page by reading OpenGraph tags from the server-rendered
//! HTML. Since October 2026 it also accepts a *component embed*: a small JSON
//! payload that replaces the standard card with a custom layout built from
//! Components V2 — markdown, media galleries, link buttons and an accent
//! colour. That is the only way to make a link preview look like anything
//! other than the usual title/description/image card.
//!
//! Both layers are emitted together on purpose. Discord requires the OG tags
//! as a fallback when the component payload cannot be shown, and the payload
//! has a hard 3,000-byte ceiling, so everything here is generated rather than
//! hand-written and length-checked by the tests at the bottom of this file.
//!
//! See <https://docs.discord.com/developers/link-previews/overview> and
//! <https://docs.discord.com/developers/link-previews/component-embeds>.

use crate::blog::Section;
use crate::projects::Project;
use crate::site::Site;
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::OnceLock;

/// Discord's documented ceiling for the escaped component JSON.
pub const COMPONENT_LIMIT: usize = 3000;

/// Discord's documented limit for `og:title`.
const TITLE_LIMIT: usize = 70;

/// Discord's documented limit for `og:description`.
const DESCRIPTION_LIMIT: usize = 350;

/// Bump when the artwork changes. Discord's media CDN caches proxied assets
/// independently of the page, and only a new URL makes it refetch them.
pub const ASSET_VERSION: &str = "1";

/// The accent used by both layers. The site is monochrome and the artwork is
/// black, so a near-black bar is more coherent than a default Discord colour
/// (and matches `--bg-color` on the site itself).
const ACCENT_HEX: &str = "#0a0a0a";
const ACCENT_DECIMAL: u64 = 0x0a0a0a;

/// One page's preview metadata, shaped for `templates/meta.html`.
#[derive(Debug, Clone, Serialize)]
pub struct Meta {
    pub title: String,
    pub description: String,
    pub site_name: String,
    pub og_type: &'static str,
    pub url: String,
    pub image: String,
    pub image_width: u32,
    pub image_height: u32,
    pub image_alt: String,
    pub accent: &'static str,
    pub oembed: String,
    /// Already escaped for a `<script type="application/json">` block.
    pub component_json: String,
}

/// The scene artwork is recorded at this size; keeping the OG dimensions in
/// step lets Discord lay the card out without measuring the GIF first.
pub const IMAGE_WIDTH: u32 = 1200;
pub const IMAGE_HEIGHT: u32 = 600;

/// The home page: the water, the name, and the way in.
pub fn home(site: &Site, origin: &str) -> Meta {
    let mut row = vec![
        link_button(format!("{origin}/gallery"), "Gallery"),
        link_button(format!("{origin}/projects"), "Projects"),
    ];
    if !site.blogs.is_empty() {
        row.push(link_button(format!("{origin}/blog"), "Blog"));
    }
    if let Some(url) = site.discord_url {
        row.push(link_button(url, "Discord"));
    }

    let component = json!({
        "component": {
            "type": 17,
            "accent_color": ACCENT_DECIMAL,
            "components": [
                {
                    "type": 9,
                    "components": [{
                        "type": 10,
                        "content": format!("# {}\n-# {}", site.greeting, site.bio)
                    }],
                    "accessory": link_button(format!("{origin}/"), "Open")
                },
                {
                    "type": 12,
                    "items": [
                        {
                            "media": { "url": artwork(site, origin) },
                            "description": "ASCII water rippling behind the greeting"
                        }
                    ]
                },
                {
                    "type": 10,
                    "content": "-# rendered live by the site's own fluid solver."
                },
                { "type": 1, "components": row }
            ]
        }
    });

    meta(
        site,
        origin,
        Page {
            path: "/",
            title: site.greeting.to_string(),
            description: site.bio.to_string(),
            og_type: "website",
        },
        component,
    )
}

/// The project list: counts plus the three projects that are actually live.
pub fn projects(
    site: &Site,
    origin: &str,
    projects: &[Project],
    live: usize,
    dreams: usize,
) -> Meta {
    let mut content = format!("# Projects\n-# {live} live · {dreams} dreaming");
    for project in projects
        .iter()
        .filter(|p| p.section == "featured" || p.section == "work")
        .take(3)
    {
        let href = project
            .url
            .clone()
            .or_else(|| project.href.clone())
            .unwrap_or_else(|| format!("{origin}/projects"));
        content.push_str(&format!(
            "\n- **[{}]({href})** — {}",
            md_text(&project.title),
            truncate_words(&project.blurb, 90)
        ));
    }

    let component = json!({
        "component": {
            "type": 17,
            "accent_color": ACCENT_DECIMAL,
            "components": [
                {
                    "type": 9,
                    "components": [{ "type": 10, "content": content }],
                    "accessory": link_button(format!("{origin}/projects"), "All projects")
                },
                {
                    "type": 1,
                    "components": [
                        link_button(format!("{origin}/"), "Home"),
                        link_button(format!("{origin}/gallery"), "Gallery"),
                        link_button(site.github_url, "GitHub")
                    ]
                }
            ]
        }
    });

    meta(
        site,
        origin,
        Page {
            path: "/projects",
            title: format!("Projects — {}", site.name),
            description: format!(
                "{live} live projects and {dreams} dreams — 3D rendering, statistics, retrieval systems and this website."
            ),
            og_type: "website",
        },
        component,
    )
}

/// The gallery: four photographs straight from the gallery's own collection.
pub fn gallery(site: &Site, origin: &str) -> Meta {
    let photos = gallery_photos();
    let items: Vec<Value> = photos
        .iter()
        .map(|photo| json!({ "media": { "url": photo } }))
        .collect();
    let count = gallery_count();

    let component = json!({
        "component": {
            "type": 17,
            "accent_color": ACCENT_DECIMAL,
            "components": [
                {
                    "type": 9,
                    "components": [{
                        "type": 10,
                        "content": "# Gallery\n-# photographs, 3D art & experiments"
                    }],
                    "accessory": link_button(format!("{origin}/gallery"), "Open")
                },
                { "type": 12, "items": items },
                {
                    "type": 10,
                    "content": format!("-# {count} pieces and counting.")
                },
                {
                    "type": 1,
                    "components": [
                        link_button(format!("{origin}/"), "Home"),
                        link_button(format!("{origin}/projects"), "Projects")
                    ]
                }
            ]
        }
    });

    meta(
        site,
        origin,
        Page {
            path: "/gallery",
            title: format!("Gallery — {}", site.name),
            description: format!("{count} photographs, 3D art and experiments."),
            og_type: "website",
        },
        component,
    )
}

/// Socials: one row of link buttons and nothing else. This is the page where
/// the embed is most useful, because the buttons are the page.
pub fn socials(site: &Site, origin: &str) -> Meta {
    let mut buttons = Vec::new();
    let mut lines = String::from("# Find me");
    if let Some(url) = site.mastodon_url {
        buttons.push(link_button(url, "Mastodon"));
        lines.push_str(&format!("\n- [Mastodon]({url}) — @{}", md_text(site.mastodon_handle.unwrap_or(""))));
    }
    if let Some(url) = site.x_url {
        buttons.push(link_button(url, "X"));
        lines.push_str(&format!("\n- [X]({url}) — {}", md_text(site.x_handle.unwrap_or(""))));
    }
    buttons.push(link_button(site.github_url, "GitHub"));
    lines.push_str(&format!("\n- [GitHub]({}) — {}", site.github_url, md_text(site.github_handle)));
    if let Some(url) = site.discord_url {
        buttons.push(link_button(url, "Discord"));
        lines.push_str(&format!("\n- [Discord]({url}) — {}", md_text(site.discord_handle.unwrap_or(""))));
    }

    let component = json!({
        "component": {
            "type": 17,
            "accent_color": ACCENT_DECIMAL,
            "components": [
                { "type": 10, "content": lines },
                { "type": 1, "components": buttons }
            ]
        }
    });

    let description = if site.professional {
        format!("Where to find {} online — GitHub.", site.full_name)
    } else {
        format!("Where to find {} online — GitHub, Mastodon, X and Discord.", site.name)
    };

    meta(
        site,
        origin,
        Page {
            path: "/socials",
            title: format!("Find me — {}", site.name),
            description,
            og_type: "website",
        },
        component,
    )
}

/// One blog section: the latest three posts as masked links.
pub fn blog(site: &Site, origin: &str, section: &Section) -> Meta {
    let mut content = format!("# {} — {}\n-# latest from the blog", section.label, site.name);
    for post in section.posts.iter().take(3) {
        let href = post
            .url
            .clone()
            .unwrap_or_else(|| format!("{origin}{}", section.url));
        content.push_str(&format!(
            "\n- **[{}]({href})**",
            md_text(&truncate_words(&post.title, 70))
        ));
        if let Some(date) = &post.published_label {
            content.push_str(&format!(" · {date}"));
        }
    }
    if section.posts.is_empty() {
        content.push_str("\n-# nothing here yet.");
    }

    let mut row = vec![link_button(format!("{origin}{}", section.url), "Read")];
    for other in site.blogs.iter().filter(|blog| blog.slug != section.slug) {
        row.push(link_button(format!("{origin}{}", other.url), other.label));
    }
    row.push(link_button(format!("{origin}/"), "Home"));

    let component = json!({
        "component": {
            "type": 17,
            "accent_color": ACCENT_DECIMAL,
            "components": [
                {
                    "type": 9,
                    "components": [{ "type": 10, "content": content }],
                    "accessory": link_button(format!("{origin}{}", section.url), "Read")
                },
                { "type": 1, "components": row }
            ]
        }
    });

    let description = section
        .posts
        .first()
        .map(|post| format!("Latest in {}: {}", section.label, post.title))
        .unwrap_or_else(|| format!("Writing from {}.", site.name));

    meta(
        site,
        origin,
        Page {
            path: &section.url,
            title: format!("{} — {}", section.label, site.name),
            description,
            og_type: "website",
        },
        component,
    )
}

/// oEmbed is the only way to put a *linked author* on the classic card, so a
/// tiny JSON document fills that one gap. Everything else is already in the OG
/// tags; Discord ignores the rest of the oEmbed fields.
pub fn oembed(site: &Site, origin: &str) -> Value {
    json!({
        "version": "1.0",
        "type": "link",
        "author_name": site.full_name,
        "author_url": format!("{origin}/"),
        "provider_name": site.full_name,
        "provider_url": format!("{origin}/"),
    })
}

// ─── Building blocks ───────────────────────────────────────────────

/// The text half of a preview: what the classic card shows.
struct Page<'a> {
    path: &'a str,
    title: String,
    description: String,
    og_type: &'static str,
}

fn meta(site: &Site, origin: &str, page: Page<'_>, component: Value) -> Meta {
    let title = truncate(&page.title, TITLE_LIMIT);
    let description = truncate_words(&page.description, DESCRIPTION_LIMIT);
    let image = artwork(site, origin);
    let image_alt = format!("ASCII water rippling behind the words \"{}\"", site.greeting);

    Meta {
        title: title.clone(),
        description,
        site_name: site.full_name.to_string(),
        og_type: page.og_type,
        url: format!("{origin}{}", page.path),
        image,
        image_width: IMAGE_WIDTH,
        image_height: IMAGE_HEIGHT,
        image_alt,
        accent: ACCENT_HEX,
        oembed: format!("{origin}/oembed.json"),
        component_json: encode(component, &title),
    }
}

/// The recorded scene for this profile. The professional mirror gets its own
/// file: the personal artwork has Alice's name printed into the water.
fn artwork(site: &Site, origin: &str) -> String {
    let who = if site.professional { "dave" } else { "alice" };
    format!("{origin}/static/embed/water-{who}.gif?v={ASSET_VERSION}")
}

fn link_button(url: impl Into<String>, label: &str) -> Value {
    json!({ "type": 2, "style": 5, "url": url.into(), "label": label })
}

/// Serialise the payload for a `<script type="application/json">` element.
///
/// Escaping `<` matters even though the data is JSON: an HTML parser ends a
/// script element at the literal string `</script`, wherever it appears.
/// Discord counts the escaped bytes against the 3,000-byte budget, so the
/// length check runs *after* escaping.
fn encode(component: Value, fallback_text: &str) -> String {
    let escaped = escape_json(&component);
    if escaped.len() <= COMPONENT_LIMIT {
        return escaped;
    }

    // Shipping an oversized payload means no component card at all. Drop to a
    // text-only card instead, which the same 3,000-byte budget always holds.
    debug_assert!(
        false,
        "component payload is {} bytes, over Discord's {COMPONENT_LIMIT}",
        escaped.len()
    );
    let fallback = json!({
        "component": {
            "type": 17,
            "accent_color": ACCENT_DECIMAL,
            "components": [{
                "type": 10,
                "content": format!("# {}", md_text(fallback_text))
            }]
        }
    });
    escape_json(&fallback)
}

fn escape_json(value: &Value) -> String {
    serde_json::to_string(value)
        .expect("component payload is serialisable")
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
}

/// Escape the characters Discord's markdown would otherwise interpret inside a
/// link label or a plain sentence.
fn md_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(ch, '\\' | '*' | '_' | '~' | '`' | '[' | ']' | '|') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Cut `text` to at most `max` bytes on a character boundary, with an ellipsis.
fn truncate(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let room = max.saturating_sub('…'.len_utf8());
    let end = text
        .char_indices()
        .take_while(|(i, ch)| i + ch.len_utf8() <= room)
        .last()
        .map(|(i, ch)| i + ch.len_utf8())
        .unwrap_or(0);
    format!("{}…", &text[..end])
}

/// Like [`truncate`], but backs up to the previous word so a blurb does not
/// end mid-word.
fn truncate_words(text: &str, max: usize) -> String {
    let cut = truncate(text, max);
    let Some(body) = cut.strip_suffix('…') else {
        return cut;
    };
    match body.rfind(char::is_whitespace) {
        Some(space) if space > 0 => format!("{}…", &body[..space]),
        _ => cut,
    }
}

/// Four photographs from the gallery's own collection, at Flickr's 800px size
/// so Discord can fetch and measure them well inside its ten-second budget.
fn gallery_photos() -> &'static [&'static str] {
    &[
        "https://live.staticflickr.com/65535/55335865751_ae878e3e04_c.jpg",
        "https://live.staticflickr.com/65535/55334930857_62296866e4_c.jpg",
        "https://live.staticflickr.com/65535/55335865911_2c8e673fc1_c.jpg",
        "https://live.staticflickr.com/65535/55334930922_72947ab303_c.jpg",
    ]
}

/// The gallery hard-codes its collection in the template, so count it there
/// rather than keeping a second number in sync by hand.
fn gallery_count() -> usize {
    static COUNT: OnceLock<usize> = OnceLock::new();
    *COUNT.get_or_init(|| {
        include_str!("../gallery/index.html")
            // Every artwork entry declares at least one subtype; the renderer's
            // JavaScript template uses `subtypes: ' + ...` and is not counted.
            .matches("subtypes: [")
            .count()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGIN: &str = "https://example.test";

    fn section(posts: usize) -> Section {
        Section {
            label: "Main".to_string(),
            slug: "alice".to_string(),
            url: "/blog".to_string(),
            feed: "https://blog.alicemow.org/alice/feed/".to_string(),
            posts: (0..posts)
                .map(|i| crate::blog::Post {
                    title: format!("Post number {i} about [things]"),
                    url: Some(format!("https://blog.alicemow.org/alice/post-{i}")),
                    published: None,
                    published_label: Some("6 Oct 2026".to_string()),
                    html: String::new(),
                })
                .collect(),
        }
    }

    fn pages(site: &Site) -> Vec<(&'static str, Meta)> {
        let list = crate::projects::load_projects();
        let live = list
            .iter()
            .filter(|p| p.section == "featured" || p.section == "work")
            .count();
        let dreams = crate::projects::load_dreams().len();
        let mut pages = vec![
            ("home", home(site, ORIGIN)),
            ("projects", projects_page(site, live, dreams)),
            ("gallery", gallery(site, ORIGIN)),
            ("socials", socials(site, ORIGIN)),
        ];
        // The professional mirror has no blogs, so there is no blog card to
        // test there — its handler 404s before it can render one.
        if !site.blogs.is_empty() {
            pages.push(("blog", blog(site, ORIGIN, &section(5))));
        }
        pages
    }

    fn projects_page(site: &Site, live: usize, dreams: usize) -> Meta {
        let list = crate::projects::load_projects();
        projects(site, ORIGIN, &list, live, dreams)
    }

    fn payload(meta: &Meta) -> Value {
        serde_json::from_str(&meta.component_json).expect("payload is valid JSON")
    }

    fn walk<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
        out.push(value);
        match value {
            Value::Array(items) => items.iter().for_each(|item| walk(item, out)),
            Value::Object(map) => map.values().for_each(|item| walk(item, out)),
            _ => {}
        }
    }

    #[test]
    fn every_page_fits_inside_discords_payload_budget() {
        for site in [Site::personal(), Site::professional()] {
            for (name, meta) in pages(&site) {
                // The escaped form is what Discord counts. `meta` already holds
                // that, so a re-serialise would undercount; assert on the raw
                // string.
                assert!(
                    meta.component_json.len() <= COMPONENT_LIMIT,
                    "{name} ({}): {} bytes",
                    site.name,
                    meta.component_json.len()
                );
            }
        }
    }

    #[test]
    fn every_page_has_a_single_container_with_supported_children() {
        const ALLOWED: &[u64] = &[1, 2, 9, 10, 11, 12, 14, 17];
        for site in [Site::personal(), Site::professional()] {
            for (name, meta) in pages(&site) {
                let payload = payload(&meta);
                let top = payload["component"].as_object().expect("container");
                assert_eq!(top["type"], 17, "{name}");
                assert_eq!(payload.as_object().unwrap().len(), 1, "{name}");

                let mut flat = Vec::new();
                walk(&payload, &mut flat);
                let typed: Vec<&Value> = flat
                    .iter()
                    .copied()
                    .filter(|v| v.get("type").and_then(Value::as_u64).is_some())
                    .collect();
                assert!(typed.len() <= 40, "{name} has too many components");
                for component in typed {
                    let kind = component["type"].as_u64().unwrap();
                    assert!(ALLOWED.contains(&kind), "{name}: type {kind}");
                }
            }
        }
    }

    #[test]
    fn media_galleries_respect_the_ten_item_cap() {
        for site in [Site::personal(), Site::professional()] {
            for (name, meta) in pages(&site) {
                let mut flat = Vec::new();
                let payload = payload(&meta);
                walk(&payload, &mut flat);
                let items: usize = flat
                    .iter()
                    .filter(|v| v.get("type").and_then(Value::as_u64) == Some(12))
                    .map(|v| v["items"].as_array().map(Vec::len).unwrap_or(0))
                    .sum();
                assert!(items <= 10, "{name}: {items} media items");
            }
        }
    }

    #[test]
    fn buttons_are_link_style_and_carry_only_documented_keys() {
        const ALLOWED: &[&str] = &["id", "type", "url", "style", "label", "emoji", "disabled"];
        for site in [Site::personal(), Site::professional()] {
            for (name, meta) in pages(&site) {
                let mut flat = Vec::new();
                let payload = payload(&meta);
                walk(&payload, &mut flat);
                for button in flat
                    .iter()
                    .filter(|v| v.get("type").and_then(Value::as_u64) == Some(2))
                {
                    assert_eq!(button["style"], 5, "{name}: non-link button");
                    for key in button.as_object().unwrap().keys() {
                        assert!(ALLOWED.contains(&key.as_str()), "{name}: key {key}");
                    }
                    assert!(button["label"].as_str().is_some(), "{name}: no label");
                }
            }
        }
    }

    #[test]
    fn media_and_buttons_point_at_absolute_urls() {
        for site in [Site::personal(), Site::professional()] {
            for (name, meta) in pages(&site) {
                let mut flat = Vec::new();
                let payload = payload(&meta);
                walk(&payload, &mut flat);
                for value in flat.iter().filter_map(|v| v.get("url").and_then(Value::as_str)) {
                    assert!(
                        value.starts_with("http://") || value.starts_with("https://"),
                        "{name}: {value} is not absolute"
                    );
                }
                // The classic card must be absolute too.
                assert!(meta.image.starts_with(ORIGIN), "{name}");
                assert!(meta.url.starts_with(ORIGIN), "{name}");
            }
        }
    }

    #[test]
    fn classic_metadata_respects_discords_byte_caps() {
        for site in [Site::personal(), Site::professional()] {
            for (name, meta) in pages(&site) {
                assert!(meta.title.len() <= TITLE_LIMIT, "{name}: title");
                assert!(meta.description.len() <= DESCRIPTION_LIMIT, "{name}: description");
                assert_eq!(meta.image_width, IMAGE_WIDTH);
                assert_eq!(meta.image_height, IMAGE_HEIGHT);
                assert_eq!(meta.og_type, "website");
            }
        }
    }

    #[test]
    fn the_professional_payload_hides_the_personal_identity() {
        let site = Site::professional();
        for (name, meta) in pages(&site) {
            let text = format!("{} {} {}", meta.title, meta.description, meta.component_json);
            for secret in ["Alice", "alicemow", "mastodon", "discord.gg", "blog."] {
                assert!(
                    !text.to_ascii_lowercase().contains(&secret.to_ascii_lowercase()),
                    "{name} leaks {secret}"
                );
            }
            assert!(meta.image.contains("water-dave.gif"), "{name}");
        }
    }

    #[test]
    fn the_personal_artwork_is_the_personal_one() {
        let meta = home(&Site::personal(), ORIGIN);
        assert!(meta.image.contains("/static/embed/water-alice.gif?v="));
        assert!(meta.component_json.contains("water-alice.gif"));
        // The card is image-only: no video files ship with the site.
        assert!(!meta.component_json.contains(".mp4"));
    }

    #[test]
    fn oembed_names_the_right_author() {
        assert_eq!(oembed(&Site::personal(), ORIGIN)["author_name"], "Alice");
        assert_eq!(
            oembed(&Site::professional(), ORIGIN)["author_name"],
            "Dave Herkula"
        );
    }

    #[test]
    fn truncation_never_splits_a_character() {
        let text = "é".repeat(100);
        let cut = truncate(&text, 11);
        assert!(cut.len() <= 11);
        assert!(cut.ends_with('…'));
        assert_eq!(truncate("short", 11), "short");
    }

    #[test]
    fn word_truncation_does_not_cut_a_word_in_half() {
        let text = "a three-axis political assessment — ownership, change and authority measured separately";
        let cut = truncate_words(text, 60);
        assert!(cut.len() <= 60);
        assert!(cut.ends_with('…'));
        assert!(!cut.contains("separate…"), "{cut}");
        assert!(cut.starts_with("a three-axis"));
    }

    #[test]
    fn the_gallery_count_ignores_the_renderer_template() {
        let total = include_str!("../gallery/index.html").matches("{ src:").count();
        // Exactly one of those matches is the JavaScript that builds a card.
        assert_eq!(gallery_count(), total - 1);
        assert!(gallery_count() > 100);
    }

    #[test]
    fn an_overlong_blog_title_is_cut_instead_of_blowing_the_budget() {
        let mut long = section(3);
        for post in &mut long.posts {
            post.title = "A very long post title that keeps going and going and going and going \
                          and going and going and going and going and going and going"
                .to_string();
            post.url = Some(format!("https://blog.alicemow.org/alice/{}", "x".repeat(120)));
        }
        let meta = blog(&Site::personal(), ORIGIN, &long);
        assert!(meta.component_json.len() <= COMPONENT_LIMIT);
        assert!(meta.component_json.contains('…'));
    }
}
