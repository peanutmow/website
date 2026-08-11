use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Project {
    pub title: String,
    pub blurb: String,
    pub tags: Vec<String>,
    /// LIVE / WIP / IDEA
    pub status: String,
    /// featured / work / ideas — which section the card renders in
    pub section: String,
    /// internal link (e.g. /portfolio)
    pub href: Option<String>,
    /// external link (e.g. GitHub)
    pub url: Option<String>,
}

impl Project {
    fn new(
        title: &str,
        blurb: &str,
        tags: &[&str],
        status: &str,
        section: &str,
        href: Option<&str>,
        url: Option<&str>,
    ) -> Self {
        Project {
            title: title.to_string(),
            blurb: blurb.to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            status: status.to_string(),
            section: section.to_string(),
            href: href.map(|s| s.to_string()),
            url: url.map(|s| s.to_string()),
        }
    }
}

/// Projects shown on /projects
pub fn load_projects() -> Vec<Project> {
    vec![
        Project::new(
            "ascii-render",
            "my 3d renderer. load an obj, drag it around, watch it degrade into beautiful monospace.",
            &["Rust", "3D", "ASCII"],
            "LIVE",
            "featured",
            None,
            Some("https://github.com/peanutmow/ascii-render"),
        ),
        Project::new(
            "This website",
            "my corner of the internet, where I display my projects, socials, and my love for art.",
            &["HTML", "JS", "CSS"],
            "LIVE",
            "work",
            None,
            Some("https://github.com/peanutmow/website/"),
        ),
        Project::new(
            "marx-knowledge-base",
            "a fully-local ai assistant over 125+ works of marx, engels & lenin. hybrid retrieval, cited answers, zero cloud.",
            &["Python", "RAG", "Ollama"],
            "LIVE",
            "work",
            None,
            Some("https://github.com/peanutmow/marx-knowledge-base"),
        ),
    ]
}

/// Dreams shown in the dreaming section — edit these to change the presets.
pub fn load_dreams() -> Vec<Project> {
    vec![
        Project::new(
            "Secure Messaging App",
            "a secure, end-to-end encrypted messaging app with a focus on privacy and usability.",
            &["Rust", "Encryption"],
            "IDEA",
            "ideas",
            None,
            None,
        ),
        Project::new(
            "Videogame",
            "a Videogame in the making, made by me and my friends.",
            &["Videogame", "Steam"],
            "IDEA",
            "ideas",
            None,
            None,
        )    
    ]
}
