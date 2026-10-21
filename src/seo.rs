use crate::blog::Section;
use crate::projects::Project;
use crate::site::Site;

pub fn sitemap(
    site: &Site,
    origin: &str,
    sections: &[Section],
    projects: &[Project],
) -> String {
    let mut paths: Vec<(String, Option<String>)> = vec![
        ("/".to_string(), None),
        ("/projects".to_string(), None),
        ("/about".to_string(), None),
        ("/faq".to_string(), None),
        ("/gallery".to_string(), None),
        ("/socials".to_string(), None),
        ("/llms.txt".to_string(), None),
        ("/llms-full.txt".to_string(), None),
    ];
    if !site.professional {
        paths.push(("/music".to_string(), None));
    }
    for blog in site.blogs {
        paths.push((
            blog.url.to_string(),
            latest_post(blog.url, sections).map(|date| date.to_string()),
        ));
    }
    for project in projects
        .iter()
        .filter(|p| p.section == "featured" || p.section == "work")
    {
        paths.push((format!("/projects/{}", project.slug), None));
    }

    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for (path, lastmod) in paths {
        let loc = format!("{}{}", escape_xml(origin), escape_xml(&path));
        match lastmod {
            Some(lastmod) => xml.push_str(&format!(
                "  <url><loc>{loc}</loc><lastmod>{}</lastmod></url>\n",
                escape_xml(&lastmod)
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
        let projects = crate::projects::load_projects();
        let body = sitemap(&Site::personal(), "https://alicemow.org", &[], &projects);
        for url in [
            "https://alicemow.org/</loc>",
            "https://alicemow.org/projects</loc>",
            "https://alicemow.org/projects/ascii-render</loc>",
            "https://alicemow.org/projects/polyaxis</loc>",
            "https://alicemow.org/about</loc>",
            "https://alicemow.org/faq</loc>",
            "https://alicemow.org/gallery</loc>",
            "https://alicemow.org/socials</loc>",
            "https://alicemow.org/music</loc>",
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
        let projects = crate::projects::load_projects();
        let body = sitemap(&Site::professional(), "https://daveherkula.org", &[], &projects);
        assert!(body.contains("https://daveherkula.org/projects</loc>"));
        assert!(!body.contains("/blog"));
        assert!(!body.contains("/music"));
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

        let body = sitemap(&Site::personal(), "https://alicemow.org", &sections, &[]);
        assert!(body.contains(
            "https://alicemow.org/blog</loc><lastmod>2026-10-06T12:00:00Z</lastmod>"
        ));
    }
}
