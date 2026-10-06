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

/// One section of the externally hosted blog, offered by the blog switcher.
///
/// Bear Blog runs everything on one blog, so the sections are tag filters over
/// it (`?q=politics` to keep only political posts, `?q=-politics` to drop them)
/// rather than separate blogs — a second blog is a paid Bear feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Blog {
    /// Switcher label, e.g. "Main" or "Politics".
    pub label: &'static str,
    /// Where the switcher sends the reader.
    pub url: &'static str,
    /// Feed advertised for this section through `<link rel="alternate">`.
    pub feed: &'static str,
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
    /// The line type-written into the ASCII water.
    pub greeting: &'static str,
    /// The one-line bio under the button bar.
    pub bio: &'static str,
    pub email: &'static str,
    pub github_url: &'static str,
    pub github_handle: &'static str,
    /// `None` hides the link entirely (used to drop X/Discord in professional).
    pub x_url: Option<&'static str>,
    pub x_handle: Option<&'static str>,
    pub discord_url: Option<&'static str>,
    pub discord_handle: Option<&'static str>,
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
/// Bear Blog serves a single blog, so these are two views over
/// `alicemow.bearblog.dev` rather than separate blogs (a second blog is a paid
/// Bear feature). Both point at *clean* URLs on purpose: Bear's tag filter also
/// exists as `?q=politics`, but query-string requests hit a Cloudflare
/// challenge that a cross-origin iframe cannot pass, because third-party
/// cookies are blocked — the frame just renders "Verification failed".
///
/// So each section is a Bear **page** ("Pages" → new page) whose body is a
/// filtered post list, and the feeds carry the same filter as a query string
/// (feeds are fetched by readers rather than framed, so `?q=` is fine there):
///
/// * `main`    → `{{ posts|tag:-politics }}` (everything except politics)
/// * `politics`→ `{{ posts|tag:politics }}`
const PERSONAL_BLOGS: &[Blog] = &[
    Blog {
        label: "Main",
        url: "https://alicemow.bearblog.dev/main/",
        feed: "https://alicemow.bearblog.dev/feed/?q=-politics",
    },
    Blog {
        label: "Politics",
        url: "https://alicemow.bearblog.dev/politics/",
        feed: "https://alicemow.bearblog.dev/feed/?q=politics",
    },
];

impl Site {
    /// The real site.
    pub const fn personal() -> Self {
        Site {
            name: "Alice",
            full_name: "Alice",
            short: "alice",
            greeting: "Hi, I'm Alice",
            bio: "artist & developer \u{2014} building things that live at the intersection of code and art.",
            email: "alice@herkula.info",
            github_url: "https://github.com/peanutmow",
            github_handle: "peanutmow",
            x_url: Some("https://x.com/Alice_mow"),
            x_handle: Some("Alice_mow"),
            discord_url: Some("https://discord.gg/sdTrfEHF"),
            discord_handle: Some("alice_meower"),
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
            greeting: "Hi, I'm Dave",
            bio: "developer \u{2014} building software and systems that hold up under real use.",
            email: "dave@herkula.info",
            github_url: "https://github.com/peanutmow",
            github_handle: "peanutmow",
            x_url: None,
            x_handle: None,
            discord_url: None,
            discord_handle: None,
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
        assert_eq!(site.blogs[0].url, "https://alicemow.bearblog.dev/main/");
        assert_eq!(site.blogs[0].feed, "https://alicemow.bearblog.dev/feed/?q=-politics");
        assert_eq!(site.blogs[1].url, "https://alicemow.bearblog.dev/politics/");
        assert_eq!(site.blogs[1].feed, "https://alicemow.bearblog.dev/feed/?q=politics");
        // Query-string URLs cannot pass Bear's Cloudflare check inside the
        // iframe, so the browsable URLs must stay query-free.
        assert!(site.blogs.iter().all(|blog| !blog.url.contains('?')));
        // Main drops politics, Politics keeps only it: no post appears twice.
        assert!(site.blogs[0].feed.contains("q=-politics"));
        assert!(site.blogs[1].feed.contains("q=politics"));

        let professional = detect(&host_header("daveherkula.org"));
        assert!(professional.blogs.is_empty());
    }

    #[test]
    fn subdomains_and_ports_are_handled() {
        assert!(detect(&host_header("www.daveherkula.org")).professional);
        assert!(detect(&host_header("daveherkula.org:8080")).professional);
        // ...but a lookalike host must not match.
        assert!(!detect(&host_header("notdaveherkula.org")).professional);
    }
}
