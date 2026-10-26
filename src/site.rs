//! Site identity profiles.
//!
//! The same binary serves two identities:
//!
//! * **personal** — the real one (Alice), used on the normal domains and in
//!   local development.
//! * **professional** — a plain-portfolio mirror (Dave Herkula) used for HR,
//!   recruiters and anyone who has been given the "other" name. It swaps the
//!   name everywhere and hides the easter eggs, novelty pages and personal
//!   social filler.
//!
//! The profile for a request is picked from the `Host` header (see
//! [`detect`]), so a single service can back both domains. Set the
//! `SITE_PROFILE` env var to force one (useful for local testing), and
//! `SITE_PROFESSIONAL_HOSTS` to add hosts when the new domain goes live.

use axum::http::{header, HeaderMap};
use serde::Serialize;

/// Domains that serve the professional profile. Add the real ones here (or via
/// the `SITE_PROFESSIONAL_HOSTS` env var) once DNS is pointed at this server.
const PROFESSIONAL_HOSTS: &[&str] = &["daveherkula.org"];

const PERSONAL_ORIGIN: &str = "https://alicemow.org";
const PROFESSIONAL_ORIGIN: &str = "https://daveherkula.org";

/// One section of the blog, offered by the blog switcher.
///
/// The blog is a self-hosted WriteFreely instance (see `deploy/writefreely`) with
/// one collection per section, because a second blog there is free where Bear
/// Blog charged for it. Its posts are read and rendered by `crate::blog`, so the
/// switcher loads a page of *this* site rather than an external one — Uberspace
/// sets `X-Frame-Options: SAMEORIGIN` on every domain, so a frame pointed at
/// `blog.alicemow.org` would simply be refused by the browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Blog {
    /// Switcher label, e.g. "Main" or "Politics".
    pub label: &'static str,
    /// Same-origin path the switcher loads (rendered by `crate::blog`).
    pub url: &'static str,
    /// Canonical public URL of the collection on the blog host.
    pub public_url: &'static str,
    /// Feed advertised for this section through `<link rel="alternate">`.
    pub feed: &'static str,
    /// The WriteFreely collection this section reads.
    pub slug: &'static str,
}

/// Everything the templates need to know about who the site belongs to.
///
/// `Copy` on purpose: it is cheap, and the middleware can drop a copy into the
/// request extensions without any locking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Site {
    /// Display name used in headings and the ASCII greeting ("Alice" / "Dave").
    pub name: &'static str,
    /// Full name for page titles ("Alice" / "Dave Herkula").
    pub full_name: &'static str,
    /// Lowercase first name for wordmarks ("alice" / "dave").
    pub short: &'static str,
    /// Public handle for the personal identity ("alicemow"); `None` here.
    pub handle: Option<&'static str>,
    /// The line type-written into the ASCII water.
    pub greeting: &'static str,
    /// The one-line bio under the button bar.
    pub bio: &'static str,
    pub email: &'static str,
    pub github_url: &'static str,
    pub github_handle: &'static str,
    /// Mastodon profile. The home page links to it with `rel="me"`, which is
    /// what Mastodon fetches to decide whether to show the link as verified.
    pub mastodon_url: Option<&'static str>,
    /// Handle without the leading `@`, e.g. `alicemow@mastodon.social`.
    pub mastodon_handle: Option<&'static str>,
    /// Full handle with the leading `@`, for `<meta name="fediverse:creator">`.
    /// Mastodon shows this account as the page author on link previews. It must
    /// stay `None` on the professional mirror: naming Alice's account there
    /// would tie the two identities together.
    pub fediverse_creator: Option<&'static str>,
    /// `None` hides the link entirely (used to drop personal accounts in
    /// professional).
    pub x_url: Option<&'static str>,
    pub x_handle: Option<&'static str>,
    pub discord_url: Option<&'static str>,
    pub discord_handle: Option<&'static str>,
    /// The numeric Discord account id (snowflake).
    ///
    /// A link preview cannot ping anyone — only message content can — so the
    /// closest thing is the oEmbed author line, whose `author_url` makes the
    /// name clickable. Pointing it at the profile is as close to a mention as
    /// an unfurl allows. `None` on the professional mirror for the same reason
    /// as the handles above.
    pub discord_user_id: Option<&'static str>,
    /// Blog sections, in switcher order. Empty drops the Blog button, the
    /// switcher and the feed links, because the professional mirror must not
    /// point recruiters at personal and political writing.
    pub blogs: &'static [Blog],
    /// True for the professional mirror: hides easter eggs, novelty pages and
    /// the "Friends" / "Cool Sites" link lists.
    pub professional: bool,
}

/// The blog sections shown on the personal site.
///
/// Both are collections on the self-hosted WriteFreely instance described in
/// `deploy/writefreely`, which federates them as `@alice@blog.alicemow.org` and
/// `@politics@blog.alicemow.org`.
///
/// `url` is deliberately a path on *this* site: the run-time reads the two feeds
/// and renders the posts itself, so the reader stays here and the page can be
/// framed (see `crate::blog`). `feed` stays canonical, so feed readers and
/// subscribers point straight at the instance.
const PERSONAL_BLOGS: &[Blog] = &[
    Blog {
        label: "Main",
        url: "/blog",
        public_url: "https://blog.alicemow.org/alice/",
        feed: "https://blog.alicemow.org/alice/feed/",
        slug: "alice",
    },
    Blog {
        label: "Politics",
        url: "/blog/politics",
        public_url: "https://blog.alicemow.org/politics/",
        feed: "https://blog.alicemow.org/politics/feed/",
        slug: "politics",
    },
];

impl Site {
    /// The real site.
    pub const fn personal() -> Self {
        Site {
            name: "Alice",
            full_name: "Alice",
            short: "alice",
            handle: Some("alicemow"),
            greeting: "Hi, I'm Alice",
            bio: "artist & developer",
            email: "alice@herkula.info",
            github_url: "https://github.com/peanutmow",
            github_handle: "peanutmow",
            mastodon_url: Some("https://mastodon.social/@alicemow"),
            mastodon_handle: Some("alicemow@mastodon.social"),
            fediverse_creator: Some("@alicemow@mastodon.social"),
            x_url: Some("https://x.com/Alice_mow"),
            x_handle: Some("Alice_mow"),
            discord_url: Some("https://discord.gg/sdTrfEHF"),
            discord_handle: Some("alice_meower"),
            discord_user_id: Some("462513388195545093"),
            blogs: PERSONAL_BLOGS,
            professional: false,
        }
    }

    /// The professional mirror: same site, plain name, no easter eggs.
    pub const fn professional() -> Self {
        Site {
            name: "Dave",
            full_name: "Dave Herkula",
            short: "dave",
            handle: None,
            greeting: "Hi, I'm Dave",
            bio: "developer \u{2014} building software and systems that hold up under real use.",
            email: "dave@herkula.info",
            github_url: "https://github.com/peanutmow",
            github_handle: "peanutmow",
            mastodon_url: None,
            mastodon_handle: None,
            fediverse_creator: None,
            x_url: None,
            x_handle: None,
            discord_url: None,
            discord_handle: None,
            discord_user_id: None,
            blogs: &[],
            professional: true,
        }
    }
}

/// Pick the profile for an incoming request.
///
/// `SITE_PROFILE` wins if set (handy for local checks); otherwise the host
/// decides, falling back to the personal site.
pub fn detect(headers: &HeaderMap) -> Site {
    if let Some(site) = from_env() {
        return site;
    }
    match host_of(headers) {
        Some(host) if is_professional_host(&host) => Site::professional(),
        _ => Site::personal(),
    }
}

fn from_env() -> Option<Site> {
    let raw = std::env::var("SITE_PROFILE").ok()?;
    Some(match raw.trim().to_ascii_lowercase().as_str() {
        "professional" | "pro" | "dave" => Site::professional(),
        _ => Site::personal(),
    })
}

/// The absolute origin of a request, e.g. `https://alicemow.org`.
///
/// Link previews need absolute URLs: Discord's crawler fetches them from its
/// own servers, so a root-relative `og:image` would point at nowhere. Known
/// production hosts collapse to the canonical domain for their profile, so the
/// same content served from a second host still advertises one canonical URL.
/// Local development keeps its own origin. Set `SITE_ORIGIN` to force one when
/// the proxy does not forward the scheme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin(pub String);

/// Resolve the request's origin from `X-Forwarded-Proto`/`Host`, unless
/// `SITE_ORIGIN` forces one; fall back to local development when there is no
/// `Host` header at all.
pub fn origin(headers: &HeaderMap) -> Origin {
    let forced = std::env::var("SITE_ORIGIN").ok();
    origin_with(headers, forced.as_deref())
}

fn origin_with(headers: &HeaderMap, forced: Option<&str>) -> Origin {
    if let Some(forced) = forced {
        let forced = forced.trim().trim_end_matches('/');
        if !forced.is_empty() {
            return Origin(forced.to_string());
        }
    }

    let Some(host) = host_with_port(headers) else {
        return Origin("http://localhost:8080".to_string());
    };
    if let Some(canonical) = canonical_origin(&host) {
        return Origin(canonical.to_string());
    }
    Origin(format!("{}://{host}", scheme(headers, &host)))
}

fn canonical_origin(host: &str) -> Option<&'static str> {
    match host.split(':').next().unwrap_or(host) {
        "alicemow.org" | "www.alicemow.org" | "alicemow.uber.space" => Some(PERSONAL_ORIGIN),
        "daveherkula.org" | "www.daveherkula.org" => Some(PROFESSIONAL_ORIGIN),
        _ => None,
    }
}

/// The canonical URL to send a request to, when `SITE_CANONICAL_REDIRECT=1`
/// asks for the alias hosts to redirect instead of just advertising a canonical.
/// Local requests, `SITE_ORIGIN` overrides and the canonical host itself are
/// left alone.
pub fn canonical_redirect(headers: &HeaderMap, path_and_query: &str) -> Option<String> {
    if std::env::var("SITE_ORIGIN").is_ok() {
        return None;
    }
    if std::env::var("SITE_CANONICAL_REDIRECT").ok().as_deref() != Some("1") {
        return None;
    }
    let host = host_with_port(headers)?;
    if is_local(&host) {
        return None;
    }
    let canonical = canonical_origin(&host)?;
    let current = format!("{}://{host}", scheme(headers, &host));
    if current == canonical {
        return None;
    }
    Some(format!("{canonical}{path_and_query}"))
}

/// The public scheme, as the reverse proxy saw it.
fn scheme<'a>(headers: &'a HeaderMap, host: &str) -> &'a str {
    if let Some(proto) = headers.get("x-forwarded-proto").and_then(|v| v.to_str().ok()) {
        let first = proto.split(',').next().unwrap_or("").trim();
        if first == "http" || first == "https" {
            return first;
        }
    }
    // Without a proxy header the scheme has to be inferred. Uberspace fronts
    // the app with HTTPS, while local runs are plain HTTP.
    if is_local(host) {
        "http"
    } else {
        "https"
    }
}

fn is_local(host: &str) -> bool {
    host.starts_with("localhost")
        || host.starts_with("127.0.0.1")
        || host.starts_with("[::1]")
}

/// The `Host` header as sent, port included. Unlike [`host_of`] this keeps a
/// non-default port, which is what makes preview URLs work in local runs.
fn host_with_port(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::HOST)?.to_str().ok()?;
    let host = raw.trim().to_ascii_lowercase();
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}

/// The bare hostname from the `Host` header, lowercased and stripped of any
/// port (`daveherkula.org:8080` -> `daveherkula.org`).
fn host_of(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::HOST)?.to_str().ok()?;
    let host = raw.split(':').next().unwrap_or(raw).trim().to_ascii_lowercase();
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}

fn is_professional_host(host: &str) -> bool {
    let extra = std::env::var("SITE_PROFESSIONAL_HOSTS").unwrap_or_default();
    PROFESSIONAL_HOSTS
        .iter()
        .copied()
        .chain(extra.split(','))
        .filter_map(|base| {
            let base = base.split(':').next().unwrap_or(base).trim().to_ascii_lowercase();
            (!base.is_empty()).then_some(base)
        })
        .any(|base| host == base || host.ends_with(&format!(".{base}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host_header(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, value.parse().unwrap());
        headers
    }

    #[test]
    fn personal_by_default() {
        assert!(!detect(&host_header("alicemow.uber.space")).professional);
        assert!(!detect(&HeaderMap::new()).professional);
    }

    #[test]
    fn professional_on_the_new_domain() {
        let site = detect(&host_header("daveherkula.org"));
        assert!(site.professional);
        assert_eq!(site.name, "Dave");
    }

    #[test]
    fn blogs_are_published_only_on_the_personal_profile() {
        let site = Site::personal();
        assert_eq!(site.blogs.len(), 2);
        assert_eq!(site.blogs[0].label, "Main");
        assert_eq!(site.blogs[1].label, "Politics");
        assert_eq!(site.blogs[0].slug, "alice");
        assert_eq!(site.blogs[1].slug, "politics");
        assert_eq!(site.blogs[0].public_url, "https://blog.alicemow.org/alice/");
        assert_eq!(site.blogs[1].public_url, "https://blog.alicemow.org/politics/");
        assert_eq!(site.blogs[0].feed, "https://blog.alicemow.org/alice/feed/");
        assert_eq!(site.blogs[1].feed, "https://blog.alicemow.org/politics/feed/");
        // The switcher loads pages of this site, because a frame pointed at the
        // blog itself is refused: Uberspace sends `X-Frame-Options: SAMEORIGIN`
        // on every domain and the blog is a different origin.
        assert_eq!(site.blogs[0].url, "/blog");
        assert_eq!(site.blogs[1].url, "/blog/politics");
        assert!(site.blogs.iter().all(|blog| blog.url.starts_with('/')));

        let professional = detect(&host_header("daveherkula.org"));
        assert!(professional.blogs.is_empty());
    }

    #[test]
    fn personal_accounts_are_hidden_on_the_professional_mirror() {
        let personal = Site::personal();
        assert_eq!(personal.mastodon_url, Some("https://mastodon.social/@alicemow"));
        assert_eq!(personal.mastodon_handle, Some("alicemow@mastodon.social"));
        assert_eq!(personal.fediverse_creator, Some("@alicemow@mastodon.social"));

        // The mirror exists for recruiters, so it must not point them at
        // personal or political accounts.
        let professional = Site::professional();
        assert!(professional.mastodon_url.is_none());
        assert!(professional.fediverse_creator.is_none());
        assert!(professional.x_url.is_none());
        assert!(professional.discord_url.is_none());
        assert!(professional.discord_user_id.is_none());
    }

    #[test]
    fn subdomains_and_ports_are_handled() {
        assert!(detect(&host_header("www.daveherkula.org")).professional);
        assert!(detect(&host_header("daveherkula.org:8080")).professional);
        // ...but a lookalike host must not match.
        assert!(!detect(&host_header("notdaveherkula.org")).professional);
    }

    #[test]
    fn origins_are_absolute_and_pick_up_the_proxy_scheme() {
        assert_eq!(
            origin_with(&host_header("alicemow.uber.space"), None).0,
            "https://alicemow.org"
        );

        // Local runs are plain HTTP, so previews built during development still
        // point somewhere fetchable.
        assert_eq!(
            origin_with(&host_header("localhost:8080"), None).0,
            "http://localhost:8080"
        );

        // Uberspace terminates TLS in front of the app.
        let mut headers = host_header("alicemow.uber.space");
        headers.insert("x-forwarded-proto", "http,https".parse().unwrap());
        assert_eq!(origin_with(&headers, None).0, "https://alicemow.org");

        // An explicit override wins.
        assert_eq!(
            origin_with(&host_header("alicemow.uber.space"), Some("https://alicemow.org/")).0,
            "https://alicemow.org"
        );
    }

    #[test]
    fn alias_hosts_share_one_canonical_origin() {
        for host in ["alicemow.org", "www.alicemow.org", "alicemow.uber.space"] {
            assert_eq!(origin_with(&host_header(host), None).0, "https://alicemow.org");
        }
        for host in ["daveherkula.org", "www.daveherkula.org"] {
            assert_eq!(origin_with(&host_header(host), None).0, "https://daveherkula.org");
        }
        assert_eq!(
            origin_with(&host_header("127.0.0.1:8080"), None).0,
            "http://127.0.0.1:8080"
        );
    }

    #[test]
    fn the_personal_profile_carries_its_handle() {
        assert_eq!(Site::personal().handle, Some("alicemow"));
        assert_eq!(Site::professional().handle, None);
    }
}
