use crate::site::Site;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Clone, Serialize)]
pub struct Link {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Qa {
    pub question: String,
    pub answer: String,
    pub links: Vec<Link>,
}

fn qa(question: &str, answer: &str, links: &[(&str, &str)]) -> Qa {
    Qa {
        question: question.to_string(),
        answer: answer.to_string(),
        links: links
            .iter()
            .map(|(label, url)| Link {
                label: label.to_string(),
                url: url.to_string(),
            })
            .collect(),
    }
}

pub fn faq(site: &Site) -> Vec<Qa> {
    if site.professional {
        professional()
    } else {
        personal()
    }
}

fn personal() -> Vec<Qa> {
    vec![
        qa(
            "Who is Alice?",
            "Alice, who publishes as alicemow, is an artist and developer. Her work spans interactive ASCII art, photography, 3D rendering and software.",
            &[
                ("Projects", "/projects"),
                ("GitHub", "https://github.com/peanutmow"),
            ],
        ),
        qa(
            "What does Alice work on?",
            "She builds small, self-contained tools and experiments, including a 3D-to-ASCII renderer, a political assessment site and a local retrieval system for large text collections. She also writes about technology and politics.",
            &[
                ("ascii-render", "https://github.com/peanutmow/ascii-render"),
                ("polyaxis", "https://polyaxis.link/"),
                ("marx-knowledge-base", "https://github.com/peanutmow/marx-knowledge-base"),
                ("All projects", "/projects"),
            ],
        ),
        qa(
            "What is ascii-render?",
            "ascii-render is a 3D renderer that loads an OBJ model, lets you rotate it and renders it as interactive monospace ASCII art. It is written in Rust and available at github.com/peanutmow/ascii-render.",
            &[
                ("ascii-render on GitHub", "https://github.com/peanutmow/ascii-render"),
                ("All projects", "/projects"),
            ],
        ),
        qa(
            "What is polyaxis?",
            "polyaxis is a three-axis political assessment. Ownership, change and authority are measured separately and reported with confidence intervals. It requires no account, sets no cookies and lives at polyaxis.link.",
            &[
                ("polyaxis.link", "https://polyaxis.link/"),
                ("All projects", "/projects"),
            ],
        ),
        qa(
            "What is marx-knowledge-base?",
            "marx-knowledge-base is a fully local AI assistant over more than 125 works by Marx, Engels and Lenin. It uses hybrid retrieval and gives cited answers without sending text to a cloud service.",
            &[
                ("marx-knowledge-base on GitHub", "https://github.com/peanutmow/marx-knowledge-base"),
                ("All projects", "/projects"),
            ],
        ),
        qa(
            "What is this website built with?",
            "The site is a Rust web application served by axum on Uberspace. The ASCII water is a WebAssembly simulation, and every page is server-rendered so crawlers and link previews receive real HTML.",
            &[
                ("Website source", "https://github.com/peanutmow/website"),
                ("ascii-render", "https://github.com/peanutmow/ascii-render"),
            ],
        ),
        qa(
            "What is BunGPT?",
            "BunGPT is a small AI chatbot at ai.alicemow.org. It runs on Cloudflare Workers and Workers AI.",
            &[("ai.alicemow.org", "https://ai.alicemow.org/")],
        ),
        qa(
            "What does Alice write about?",
            "The blog has two sections: Main, for technology and personal writing, and Politics. Both are published with a self-hosted WriteFreely instance at blog.alicemow.org.",
            &[("blog.alicemow.org", "https://blog.alicemow.org/")],
        ),
        qa(
            "Is the source code available?",
            "Yes. Most projects, including this website, are public on GitHub at github.com/peanutmow.",
            &[("GitHub", "https://github.com/peanutmow")],
        ),
        qa(
            "How can I contact Alice?",
            "Email alice@herkula.info, or reach her on Mastodon as @alicemow@mastodon.social, on X as @Alice_mow, or on Discord as alice_meower.",
            &[
                ("Email", "mailto:alice@herkula.info"),
                ("Mastodon", "https://mastodon.social/@alicemow"),
                ("X", "https://x.com/Alice_mow"),
                ("Discord", "https://discord.gg/sdTrfEHF"),
            ],
        ),
    ]
}

fn professional() -> Vec<Qa> {
    vec![
        qa(
            "Who is Dave Herkula?",
            "Dave Herkula is a software developer who builds software and systems that hold up under real use.",
            &[
                ("Projects", "/projects"),
                ("GitHub", "https://github.com/peanutmow"),
            ],
        ),
        qa(
            "What does Dave work on?",
            "He works on software systems and focused tools, including a 3D-to-ASCII renderer, a statistical political assessment site and a local retrieval system for large text collections.",
            &[
                ("ascii-render", "https://github.com/peanutmow/ascii-render"),
                ("polyaxis", "https://polyaxis.link/"),
                ("marx-knowledge-base", "https://github.com/peanutmow/marx-knowledge-base"),
                ("All projects", "/projects"),
            ],
        ),
        qa(
            "What is ascii-render?",
            "ascii-render is a 3D renderer that loads an OBJ model, lets you rotate it and renders it as interactive monospace ASCII art. It is written in Rust and available at github.com/peanutmow/ascii-render.",
            &[
                ("ascii-render on GitHub", "https://github.com/peanutmow/ascii-render"),
                ("All projects", "/projects"),
            ],
        ),
        qa(
            "What is polyaxis?",
            "polyaxis is a three-axis political assessment. Ownership, change and authority are measured separately and reported with confidence intervals. It requires no account, sets no cookies and lives at polyaxis.link.",
            &[
                ("polyaxis.link", "https://polyaxis.link/"),
                ("All projects", "/projects"),
            ],
        ),
        qa(
            "What is marx-knowledge-base?",
            "marx-knowledge-base is a fully local AI assistant over more than 125 works by Marx, Engels and Lenin. It uses hybrid retrieval and gives cited answers without sending text to a cloud service.",
            &[
                ("marx-knowledge-base on GitHub", "https://github.com/peanutmow/marx-knowledge-base"),
                ("All projects", "/projects"),
            ],
        ),
        qa(
            "What is this website built with?",
            "The site is a Rust web application served by axum on Uberspace. The ASCII water is a WebAssembly simulation, and every page is server-rendered.",
            &[
                ("Website source", "https://github.com/peanutmow/website"),
                ("ascii-render", "https://github.com/peanutmow/ascii-render"),
            ],
        ),
        qa(
            "Is the source code available?",
            "Yes. Most projects, including this website, are public on GitHub at github.com/peanutmow.",
            &[("GitHub", "https://github.com/peanutmow")],
        ),
        qa(
            "How can I contact Dave?",
            "Email dave@herkula.info or find him on GitHub at github.com/peanutmow.",
            &[
                ("Email", "mailto:dave@herkula.info"),
                ("GitHub", "https://github.com/peanutmow"),
            ],
        ),
    ]
}

pub fn faq_json_ld(entries: &[Qa]) -> String {
    let questions: Vec<serde_json::Value> = entries
        .iter()
        .map(|entry| {
            json!({
                "@type": "Question",
                "name": entry.question,
                "acceptedAnswer": {
                    "@type": "Answer",
                    "text": entry.answer
                }
            })
        })
        .collect();

    json!({
        "@context": "https://schema.org",
        "@type": "FAQPage",
        "mainEntity": questions,
    })
    .to_string()
    .replace('<', "\\u003c")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_personal_faq_covers_projects_and_contact() {
        let entries = faq(&Site::personal());
        let text: String = entries
            .iter()
            .map(|entry| format!("{} {}", entry.question, entry.answer))
            .collect();
        assert!(text.contains("ascii-render"));
        assert!(text.contains("BunGPT"));
        assert!(text.contains("blog.alicemow.org"));
        assert!(text.contains("alice@herkula.info"));
    }

    #[test]
    fn the_professional_faq_leaks_no_personal_accounts() {
        let entries = faq(&Site::professional());
        let text: String = entries
            .iter()
            .map(|entry| format!("{} {}", entry.question, entry.answer))
            .collect();
        assert!(!text.contains("alicemow"));
        assert!(!text.contains("mastodon"));
        assert!(!text.contains("BunGPT"));
        assert!(!text.contains("blog."));
        assert!(text.contains("dave@herkula.info"));
    }

    #[test]
    fn faq_json_ld_is_a_valid_faq_page() {
        let body = faq_json_ld(&faq(&Site::personal()));
        let value: serde_json::Value = serde_json::from_str(&body).expect("valid json");
        assert_eq!(value["@type"], "FAQPage");
        assert!(value["mainEntity"].as_array().unwrap().len() >= 8);
        assert!(body.contains("\"@type\":\"Question\""));
        assert!(!body.contains('<'));
    }
}
