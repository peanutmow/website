use crate::blog::Section;
use crate::site::Site;

pub fn sitemap(site: &Site, origin: &str, sections: &[Section]) -> String {
    let mut paths: Vec<(&str, Option<&str>)> = vec![
        ("/", None),
        ("/projects", None),
        ("/about", None),
        ("/faq", None),
        ("/gallery", None),
        ("/socials", None),
        ("/llms.txt", None),
        ("/llms-full.txt", None),
    ];
    for blog in site.blogs {
        paths.push((blog.url, latest_post(blog.url, sections)));
    }

    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for (path, lastmod) in paths {
        let loc = format!("{}{}", escape_xml(origin), escape_xml(path));
        match lastmod {
            Some(lastmod) => xml.push_str(&format!(
                "  <url><loc>{loc}</loc><lastmod>{}</lastmod></url>\n",
                escape_xml(lastmod)
            )),
            None => xml.push_str(&format!("  <url><loc>{loc}</loc></url>\n")),
        }
    }
    xml.push_str("</urlset>\n");
    xml
}

fn latest_post<'a>(url: &str, sections: &'a [Section]) -> Option<&'a str> {
    sections
        .iter()
        .find(|section| section.url == url)
        .and_then(|section| {
            section
                .posts
                .iter()
                .filter_map(|post| post.published.as_deref())
                .max()
        })
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
    fn the_personal_sitemap_lists_every_public_page() {
        let body = sitemap(&Site::personal(), "https://alicemow.org", &[]);
        for url in [
            "https://alicemow.org/</loc>",
            "https://alicemow.org/projects</loc>",
            "https://alicemow.org/about</loc>",
            "https://alicemow.org/faq</loc>",
            "https://alicemow.org/gallery</loc>",
            "https://alicemow.org/socials</loc>",
            "https://alicemow.org/llms.txt</loc>",
            "https://alicemow.org/llms-full.txt</loc>",
            "https://alicemow.org/blog</loc>",
            "https://alicemow.org/blog/politics</loc>",
        ] {
            assert!(body.contains(url), "missing {url}");
        }
    }

    #[test]
    fn the_professional_sitemap_has_no_blog() {
        let body = sitemap(&Site::professional(), "https://daveherkula.org", &[]);
        assert!(body.contains("https://daveherkula.org/projects</loc>"));
        assert!(!body.contains("/blog"));
    }

    #[test]
    fn blog_entries_carry_the_latest_post_date() {
        use crate::blog::Post;

        let sections = vec![Section {
            label: "Main".to_string(),
            slug: "alice".to_string(),
            url: "/blog".to_string(),
            feed: String::new(),
            posts: vec![
                Post {
                    title: "Newer".to_string(),
                    url: None,
                    published: Some("2026-10-06T12:00:00Z".to_string()),
                    published_label: None,
                    html: String::new(),
                },
                Post {
                    title: "Older".to_string(),
                    url: None,
                    published: Some("2026-09-01T08:00:00Z".to_string()),
                    published_label: None,
                    html: String::new(),
                },
            ],
        }];

        let body = sitemap(&Site::personal(), "https://alicemow.org", &sections);
        assert!(body.contains(
            "https://alicemow.org/blog</loc><lastmod>2026-10-06T12:00:00Z</lastmod>"
        ));
    }
}
