use crate::site::Site;

pub fn robots(origin: &str) -> String {
    format!(
        "User-agent: *\n\
         Allow: /\n\
         Disallow: /oembed.json\n\
         Disallow: /embed-scene\n\
         Disallow: /dev/null\n\
         Disallow: /conejillo\n\
         Disallow: /redherring\n\
         \n\
         Sitemap: {origin}/sitemap.xml\n"
    )
}

pub fn sitemap(site: &Site, origin: &str) -> String {
    let mut paths = vec!["/", "/projects", "/gallery", "/socials"];
    for blog in site.blogs {
        paths.push(blog.url);
    }

    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for path in paths {
        xml.push_str(&format!(
            "  <url><loc>{}{}</loc></url>\n",
            escape_xml(origin),
            escape_xml(path)
        ));
    }
    xml.push_str("</urlset>\n");
    xml
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn robots_points_crawlers_at_the_sitemap() {
        let body = robots("https://alicemow.org");
        assert!(body.contains("Allow: /"));
        assert!(body.contains("Sitemap: https://alicemow.org/sitemap.xml"));
        assert!(body.contains("Disallow: /oembed.json"));
    }

    #[test]
    fn the_personal_sitemap_lists_every_public_page() {
        let body = sitemap(&Site::personal(), "https://alicemow.org");
        for url in [
            "https://alicemow.org/</loc>",
            "https://alicemow.org/projects</loc>",
            "https://alicemow.org/gallery</loc>",
            "https://alicemow.org/socials</loc>",
            "https://alicemow.org/blog</loc>",
            "https://alicemow.org/blog/politics</loc>",
        ] {
            assert!(body.contains(url), "missing {url}");
        }
    }

    #[test]
    fn the_professional_sitemap_has_no_blog() {
        let body = sitemap(&Site::professional(), "https://daveherkula.org");
        assert!(body.contains("https://daveherkula.org/projects</loc>"));
        assert!(!body.contains("/blog"));
    }
}
