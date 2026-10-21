//! The blog: read from the WriteFreely instance on this host and rendered here.
//!
//! The blog itself is a separate, federated WriteFreely instance (see
//! `deploy/writefreely`). It cannot be *framed* into this site: Uberspace sets
//! `X-Frame-Options: SAMEORIGIN` on every domain, and `blog.alicemow.org` is a
//! different origin from `alicemow.org`, so the browser refuses the frame.
//!
//! So instead of embedding it, this module reads the instance's RSS feeds and
//! renders the posts through our own template at `/blog`. That page *is*
//! same-origin, so it frames inside the content panel perfectly — and it keeps
//! the site's own look instead of WriteFreely's.
//!
//! Feeds are read over the loopback interface in plain HTTP. WriteFreely listens
//! on `0.0.0.0:8082` on the same machine, so the request never leaves the host —
//! which is also why this crate carries no TLS client at all. The deploy builds
//! a static musl binary, and keeping OpenSSL/ring out of it is worth a lot. Set
//! `BLOG_FEED_BASE` to point somewhere else (a test server, say).
//!
//! The HTML inside a feed item is written by the site owner in their own blog
//! and is rendered as-is, the same way any feed reader treats it. The one thing
//! that is not trusted is the *link* scheme, which is checked before it reaches
//! an `href` (see [`is_web_url`]).

use crate::site::{Blog, Site};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long a successful read is reused before WriteFreely is asked again. The
/// read is two tiny requests over loopback, so this stays short on purpose:
/// after publishing, a refresh shows the post within a minute.
const TTL: Duration = Duration::from_secs(60);

/// How long to wait before retrying after a failed read. Without this, a blog
/// that is down would add a request timeout to every single page view.
const RETRY_AFTER: Duration = Duration::from_secs(30);

/// Loopback round trip; a healthy instance answers in milliseconds.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

/// One published post, shaped for the template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Post {
    pub title: String,
    /// Canonical URL on the blog, if the feed gave us a usable one.
    pub url: Option<String>,
    /// ISO-8601, for the `<time datetime="…">` attribute.
    pub published: Option<String>,
    /// The same instant written for people, e.g. "6 Oct 2026".
    pub published_label: Option<String>,
    /// Post body as HTML, straight from `<content:encoded>`.
    pub html: String,
}

/// One section of the blog (main / politics) with its posts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Section {
    pub label: String,
    pub slug: String,
    /// Same-origin path that renders this section.
    pub url: String,
    /// Canonical RSS feed, advertised through `<link rel="alternate">`.
    pub feed: String,
    pub posts: Vec<Post>,
}

/// Reads the blog feeds and holds on to the last good answer.
pub struct Cache {
    client: reqwest::Client,
    base: String,
    snapshot: Mutex<Option<Snapshot>>,
}

struct Snapshot {
    sections: Vec<Section>,
    at: Instant,
    /// False when this is a fallback kept after a failed read, which is retried
    /// much sooner than a successful one.
    ok: bool,
}

impl Cache {
    pub fn new() -> Self {
        let base =
            std::env::var("BLOG_FEED_BASE").unwrap_or_else(|_| "http://127.0.0.1:8082".to_string());
        Self::with_base(base)
    }

    fn with_base(base: String) -> Self {
        let base = base.trim().trim_end_matches('/').to_string();
        if !base.starts_with("http://") {
            tracing::warn!(
                "BLOG_FEED_BASE should be plain http:// — this build has no TLS client, \
                 so remote origins cannot be read. Got {base:?}"
            );
        }
        tracing::info!("Reading blog feeds from {base}");

        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("failed to build the blog HTTP client");

        Cache {
            client,
            base,
            snapshot: Mutex::new(None),
        }
    }

    /// The posts of every section of `site`, in the order `site.blogs` lists
    /// them, served from cache whenever that is still current.
    pub async fn sections(&self, site: &Site) -> Vec<Section> {
        // The professional mirror has no blogs. Return before touching the
        // cache: it is shared between profiles, so storing an empty list here
        // would hand the personal site 404s until the entry expired.
        if site.blogs.is_empty() {
            return Vec::new();
        }

        if let Some(sections) = self.reusable() {
            return sections;
        }

        match self.read(site).await {
            Ok(sections) => {
                self.store(sections.clone(), true);
                sections
            }
            Err(err) => {
                // Better a slightly old blog than a blank one, so keep serving
                // the last good copy through a restart of WriteFreely.
                tracing::warn!("Blog feeds unavailable ({err}); serving the last good copy");
                let sections = self.last_good(site);
                self.store(sections.clone(), false);
                sections
            }
        }
    }

    /// The cached sections while they are still fresh enough to reuse.
    fn reusable(&self) -> Option<Vec<Section>> {
        let snapshot = self.snapshot.lock().ok()?;
        let snapshot = snapshot.as_ref()?;
        let window = if snapshot.ok { TTL } else { RETRY_AFTER };
        (snapshot.at.elapsed() < window).then(|| snapshot.sections.clone())
    }

    /// Whatever we held before, or empty sections if that belongs to a
    /// different profile or an older blog list.
    fn last_good(&self, site: &Site) -> Vec<Section> {
        let cached = self
            .snapshot
            .lock()
            .ok()
            .and_then(|snapshot| snapshot.as_ref().map(|held| held.sections.clone()));

        match cached {
            Some(sections) if sections.len() == site.blogs.len() => sections,
            _ => empty_sections(site),
        }
    }

    fn store(&self, sections: Vec<Section>, ok: bool) {
        if let Ok(mut snapshot) = self.snapshot.lock() {
            *snapshot = Some(Snapshot {
                sections,
                at: Instant::now(),
                ok,
            });
        }
    }

    async fn read(&self, site: &Site) -> Result<Vec<Section>, String> {
        let mut sections = Vec::with_capacity(site.blogs.len());
        let mut failed = 0;

        for blog in site.blogs {
            let url = format!("{}/{}/feed/", self.base, blog.slug);
            match self.fetch(&url).await.and_then(|body| parse(&body)) {
                Ok(posts) => sections.push(section_of(blog, posts)),
                Err(err) => {
                    tracing::warn!("Blog feed {url}: {err}");
                    failed += 1;
                    sections.push(section_of(blog, Vec::new()));
                }
            }
        }

        // One dead feed still leaves the other readable. Only when they are all
        // dead is the instance itself down, and the caller should fall back to
        // what it already had.
        if failed == site.blogs.len() {
            return Err(format!("all {} blog feeds failed", site.blogs.len()));
        }
        Ok(sections)
    }

    async fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
        let response = self.client.get(url).send().await.map_err(|e| e.to_string())?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("HTTP {status}"));
        }
        response
            .bytes()
            .await
            .map(|body| body.to_vec())
            .map_err(|e| e.to_string())
    }
}

/// Which section a request is asking for: `/blog` is the first one,
/// `/blog/<slug>` the matching one, and anything else is not published.
pub fn index_of(site: &Site, slug: Option<&str>) -> Option<usize> {
    match slug {
        None => (!site.blogs.is_empty()).then_some(0),
        Some(slug) => site.blogs.iter().position(|blog| blog.slug == slug),
    }
}

fn section_of(blog: &Blog, posts: Vec<Post>) -> Section {
    Section {
        label: blog.label.to_string(),
        slug: blog.slug.to_string(),
        url: blog.url.to_string(),
        feed: blog.feed.to_string(),
        posts,
    }
}

fn empty_sections(site: &Site) -> Vec<Section> {
    site.blogs.iter().map(|blog| section_of(blog, Vec::new())).collect()
}

fn parse(body: &[u8]) -> Result<Vec<Post>, String> {
    let feed = feed_rs::parser::parse(body).map_err(|e| e.to_string())?;

    // Carry the timestamp alongside each post so ordering does not depend on the
    // textual form of the date: <pubDate> offsets differ between these feeds.
    let mut dated: Vec<(Option<DateTime<Utc>>, Post)> = feed
        .entries
        .into_iter()
        .map(|entry| {
            let when = entry.published.or(entry.updated);
            (when, post_of(entry, when))
        })
        .collect();

    dated.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(dated.into_iter().map(|(_, post)| post).collect())
}

fn post_of(entry: feed_rs::model::Entry, when: Option<DateTime<Utc>>) -> Post {
    let title = entry
        .title
        .map(|text| text.content)
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| "(untitled)".to_string());

    Post {
        title,
        url: entry
            .links
            .iter()
            .map(|link| link.href.clone())
            .find(|href| is_web_url(href)),
        published: when.map(|when| when.to_rfc3339()),
        published_label: when.map(|when| when.format("%-d %b %Y").to_string()),
        html: entry.content.and_then(|content| content.body).unwrap_or_default(),
    }
}

/// Only `http(s)` links are rendered into an `href`. The feed is remote input,
/// and a `javascript:` URL would otherwise be a script injection.
fn is_web_url(href: &str) -> bool {
    let href = href.trim_start();
    ["http://", "https://"].iter().any(|scheme| {
        href.get(..scheme.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(scheme))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exactly what `https://blog.alicemow.org/alice/feed/` serves.
    const ALICE_FEED: &str = r#"<?xml version="1.0" encoding="UTF-8"?><rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/">
  <channel>
    <title>blog</title>
    <link>https://blog.alicemow.org/alice/</link>
    <description>hello, this is where I post my thoughts :)</description>
    <pubDate>Tue, 06 Oct 2026 21:08:51 +0200</pubDate>
    <item>
      <title>Hello World!</title>
      <link>https://blog.alicemow.org/alice/hello-world</link>
      <description>&lt;![CDATA[Hello World!&#xA;]]&gt;</description>
      <content:encoded><![CDATA[<p>Hello World!</p>
]]></content:encoded>
      <guid>https://blog.alicemow.org/alice/hello-world</guid>
      <pubDate>Tue, 06 Oct 2026 18:58:31 +0000</pubDate>
    </item>
  </channel>
</rss>
"#;

    /// A blog with no posts yet is a normal state, not an error.
    const POLITICS_FEED: &str = r#"<?xml version="1.0" encoding="UTF-8"?><rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/">
  <channel>
    <title>Politics</title>
    <link>https://blog.alicemow.org/politics/</link>
    <description></description>
    <pubDate>Tue, 06 Oct 2026 21:08:51 +0200</pubDate>
  </channel>
</rss>
"#;

    #[test]
    fn a_writefreely_post_is_parsed() {
        let posts = parse(ALICE_FEED.as_bytes()).unwrap();

        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].title, "Hello World!");
        assert_eq!(
            posts[0].url.as_deref(),
            Some("https://blog.alicemow.org/alice/hello-world")
        );
        assert_eq!(posts[0].published_label.as_deref(), Some("6 Oct 2026"));
        // The body has to survive as HTML: the template marks it safe.
        assert!(posts[0].html.contains("<p>Hello World!</p>"));
    }

    #[test]
    fn an_empty_feed_is_not_an_error() {
        assert!(parse(POLITICS_FEED.as_bytes()).unwrap().is_empty());
    }

    #[test]
    fn newer_posts_come_first() {
        let feed = ALICE_FEED.replace(
            "</channel>",
            "<item><title>Later</title><link>https://blog.alicemow.org/alice/later</link>\
             <pubDate>Wed, 07 Oct 2026 09:00:00 +0000</pubDate></item></channel>",
        );

        let posts = parse(feed.as_bytes()).unwrap();
        assert_eq!(posts.len(), 2);
        assert_eq!(posts[0].title, "Later");
    }

    #[test]
    fn only_web_links_become_hrefs() {
        assert!(is_web_url("https://blog.alicemow.org/alice/x"));
        assert!(is_web_url("HTTP://example.com/x"));
        assert!(!is_web_url("javascript:alert(1)"));
        assert!(!is_web_url("data:text/html,<script>"));
        // Must not panic on a multi-byte prefix shorter than a scheme.
        assert!(!is_web_url("ünter"));
        assert!(!is_web_url(""));
    }

    #[test]
    fn sections_are_found_by_slug() {
        let site = Site::personal();
        assert_eq!(index_of(&site, None), Some(0));
        assert_eq!(index_of(&site, Some("alice")), Some(0));
        assert_eq!(index_of(&site, Some("politics")), Some(1));
        // Not a section we publish — and the professional mirror has none at all.
        assert_eq!(index_of(&site, Some("nope")), None);
        assert_eq!(index_of(&Site::professional(), None), None);
    }

    #[tokio::test]
    async fn the_professional_mirror_cannot_poison_the_shared_cache() {
        // Nothing listens on port 1, so every read fails fast and the module
        // has to fall back to empty sections of the right length.
        let cache = Cache::with_base("http://127.0.0.1:1".to_string());

        assert!(cache.sections(&Site::professional()).await.is_empty());

        let personal = cache.sections(&Site::personal()).await;
        assert_eq!(personal.len(), 2, "the empty professional list must not be reused");
        assert!(personal.iter().all(|section| section.posts.is_empty()));
        // The tabs must still name both blogs when the feeds are unreachable.
        assert_eq!(personal[0].label, "Main");
        assert_eq!(personal[1].url, "/blog/politics");
    }
}
