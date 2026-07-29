use pulldown_cmark::{Parser, html};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct BlogPost {
    pub slug: String,
    pub title: String,
    pub date: String,
    pub category: String,
    pub excerpt: String,
    pub content: String,
}

/// Load blog posts from the blog/posts/ directory
pub fn load_blog_posts() -> Result<Vec<BlogPost>, String> {
    let posts_dir = Path::new("blog/posts");
    if !posts_dir.exists() {
        return Ok(Vec::new());
    }

    let mut posts = Vec::new();

    for entry in fs::read_dir(posts_dir).map_err(|e| format!("Failed to read blog directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        // Only process .md files
        if path.extension().map_or(true, |ext| ext != "md") {
            continue;
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        let slug = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Parse frontmatter and content
        let (metadata, body) = parse_frontmatter(&content);

        let title = metadata.get("title").cloned().unwrap_or_else(|| slug.clone());
        let date = metadata.get("date").cloned().unwrap_or_default();
        let category = metadata.get("category").cloned().unwrap_or_default();
        let excerpt = metadata.get("excerpt").cloned().unwrap_or_else(|| {
            // Auto-generate excerpt from first ~150 chars of body
            let clean = body.trim();
            if clean.len() > 150 {
                format!("{}...", &clean[..150])
            } else {
                clean.to_string()
            }
        });

        posts.push(BlogPost {
            slug,
            title,
            date,
            category,
            excerpt,
            content: body,
        });
    }

    // Sort by date descending (most recent first)
    posts.sort_by(|a, b| b.date.cmp(&a.date));

    Ok(posts)
}

/// Simple frontmatter parser (supports YAML-like --- delimited blocks)
fn parse_frontmatter(content: &str) -> (std::collections::HashMap<String, String>, String) {
    let mut metadata = std::collections::HashMap::new();
    let mut body = content.to_string();

    if content.starts_with("---") {
        if let Some(end) = content[3..].find("\n---") {
            let frontmatter = &content[3..3 + end];
            body = content[3 + end + 5..].trim().to_string();

            for line in frontmatter.lines() {
                if let Some(pos) = line.find(':') {
                    let key = line[..pos].trim().to_string();
                    let value = line[pos + 1..].trim().trim_matches('"').to_string();
                    metadata.insert(key, value);
                }
            }
        }
    }

    (metadata, body)
}

/// Render markdown to HTML
pub fn render_markdown(markdown: &str) -> String {
    let parser = Parser::new(markdown);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}
