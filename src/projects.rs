use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ProjectLink {
    pub label: String,
    pub href: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Fact {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Note {
    pub heading: String,
    pub paragraphs: Vec<String>,
    pub bullets: Vec<String>,
    pub quote: Option<String>,
}

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
    pub slug: String,
    pub summary: String,
    pub facts: Vec<Fact>,
    pub links: Vec<ProjectLink>,
    pub sections: Vec<Note>,
    pub specimen: String,
    pub specimen_caption: String,
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
            slug: String::new(),
            summary: String::new(),
            facts: Vec::new(),
            links: Vec::new(),
            sections: Vec::new(),
            specimen: String::new(),
            specimen_caption: String::new(),
        }
    }
}

fn fact(label: &str, value: &str) -> Fact {
    Fact {
        label: label.to_string(),
        value: value.to_string(),
    }
}

fn link(label: &str, href: &str) -> ProjectLink {
    ProjectLink {
        label: label.to_string(),
        href: href.to_string(),
    }
}

fn note(heading: &str, paragraphs: &[&str], bullets: &[&str], quote: Option<&str>) -> Note {
    Note {
        heading: heading.to_string(),
        paragraphs: paragraphs.iter().map(|s| s.to_string()).collect(),
        bullets: bullets.iter().map(|s| s.to_string()).collect(),
        quote: quote.map(|s| s.to_string()),
    }
}

/// Projects shown on /projects
pub fn load_projects() -> Vec<Project> {
    vec![
        Project {
            slug: "ascii-render".to_string(),
            summary: "a javascript renderer that turns any .obj mesh into an interactive ascii drawing. no canvas, no pixels, no build step.".to_string(),
            facts: vec![
                fact("year", "2026"),
                fact("role", "renderer + converter"),
                fact("stack", "javascript"),
                fact("licence", "AGPL-3.0"),
                fact("dependencies", "none"),
            ],
            links: vec![link("github", "https://github.com/peanutmow/ascii-render")],
            sections: vec![
                note(
                    "what it is",
                    &[
                        "two files. the first drops the stanford bunny into any page as a <pre> you can turn with the cursor. the second is a node script that bakes your own mesh into the same kind of self-contained renderer.",
                    ],
                    &[
                        "one <pre> and one script tag for the bunny demo",
                        "node obj-to-ascii-render.js model.obj writes a plain .js file",
                        "no npm packages, no bundler, no runtime imports",
                    ],
                    None,
                ),
                note(
                    "how it works",
                    &[
                        "positions, normals and triangle indices are loaded once as typed arrays. every frame the mesh is rotated, projected orthographically onto the character grid, and shaded by one key light with two weak fills. luminance picks a glyph from a density ramp. the glyph is the pixel.",
                        "on export the mesh is normalised into the unit cube, so scale and origin in blender do not matter.",
                    ],
                    &[],
                    None,
                ),
                note(
                    "the limits",
                    &[],
                    &[
                        "face indices live in a Uint16Array: 65,535 vertices after processing",
                        "uv maps and .mtl materials are ignored",
                        "models without normals get smooth normals averaged from adjacent faces",
                        "anything larger needs decimating before export",
                    ],
                    Some("a framebuffer is not the only way to make a model legible. at the right resolution, characters carry the silhouette."),
                ),
            ],
            specimen: r#"         %@%         %++**
       ~#%%%*     *@;;+#@#
       ;%%%*~  +#+~=%@@@@*
       +@%#+ *#*+#@@@@@%
      :#%#*#%@*@+=-.
 :=+##%%@@#*+=~,
 :=+#%@#+=+++;
 :=*#@#++@@##*
 :;+*%##@@@##*%%#      **##%%%%#
::::=*%**++++@%%%%%****##%%%%@@@@%#
-----:--,-:#@%%%###*****###%%%@@@@%%#
   -..,+*#####%##****++******#%@@@@%#*
  :::~=++*********++***=+*###%@@@@@%#**
  :::~;=++++++=====+*##%%%@@@%**@@@%#**+
  -::::~~;=+++;~:::=++++**#%%##*****+++=
   ------::~~;;;;-::;;======+*#*+++=====
    ,,-------~~;;----:~;=====;;;;;;====+%#
      ,,,,,,-:~;;~---:::~;;==;;;;+====*#%@%+
           ,::~;;;:--:-:::~~;;====;;~=***+;;
           ::~;=+*;:----:~~;~:::-:::;==;::-
        =*#%%#*+=+#%%*+,,,,,,-;==;---,...
       ::;+***=::;=++++++======:,....."#.to_string(),
            specimen_caption: "the stanford bunny, 2,503 vertices and 4,968 faces, rasterised by the renderer itself. move the cursor over it.".to_string(),
            ..Project::new(
                "ascii-render",
                "my 3d renderer. load an obj, drag it around, watch it degrade into beautiful monospace.",
                &["JavaScript", "3D", "ASCII"],
                "LIVE",
                "featured",
                None,
                Some("https://github.com/peanutmow/ascii-render"),
            )
        },
        Project {
            slug: "polyaxis".to_string(),
            summary: "a political test that refuses to collapse three different questions into one line.".to_string(),
            facts: vec![
                fact("year", "2026"),
                fact("role", "design, statistics, front end"),
                fact("stack", "javascript"),
                fact("items", "230"),
                fact("scoring", "local, in-browser"),
            ],
            links: vec![
                link("polyaxis.link", "https://polyaxis.link/"),
                link("the test", "https://polyaxis.link/test"),
                link("method", "https://polyaxis.link/method"),
                link("figures", "https://polyaxis.link/figures"),
            ],
            sections: vec![
                note(
                    "three axes, not one",
                    &[
                        "ownership asks who owns and directs productive capital. change asks how far inherited norms should be conserved or revised. authority asks how much concentrated, unaccountable power over the individual is acceptable.",
                        "a ministry-owned factory is not worker control, and a centrally directed economy that subordinates workers is not communal ownership either. every item belongs to exactly one axis, and the report gives three coordinates instead of a single left-right score.",
                    ],
                    &[],
                    None,
                ),
                note(
                    "scoring",
                    &[
                        "agree/disagree and true/false items use a graded response model. multiple choice items use a position-based model, where each option is itself a point on the axis. the estimate is the posterior mean under a standard-normal prior, reported with an interval of ±1.96 posterior standard deviations.",
                        "skipped questions are excluded. they are not counted as centrist.",
                    ],
                    &[],
                    Some("one axis cannot hold ownership, change and authority without flattening at least two of them."),
                ),
                note(
                    "what it does not know",
                    &[
                        "the item parameters are provisional. calibration needs a large representative sample, and this instrument has never had one. the discrimination and threshold values encode an explicit judgement about what each statement measures, and they are published in the engine source.",
                    ],
                    &[
                        "12 of 230 items sit in the extreme difficulty band",
                        "above roughly 80, the interval matters more than the number",
                        "figures on the site are editorial estimates, not measurements",
                    ],
                    None,
                ),
                note(
                    "privacy",
                    &[
                        "there are no accounts and no cookies. scoring, rendering and export all run in the browser. a shared result travels in the url fragment, which browsers never send to a server.",
                        "finishing a test adds each answer to an anonymous tally so the items can be improved. what is stored is a count per question per answer, not a record of a person. the test scores identically whether or not that tally is reachable.",
                    ],
                    &[],
                    None,
                ),
            ],
            specimen: r#"                         authority
                             ▲
                             │
                    ┌────────┴────────┐
                   ╱│                ╱│
                  ┌─┼──────────────┐ │
                  │ └──────────────┼─┘
                  │ ╱              │╱
                  └────────────────┘
      collective ────────┼─────────────▶ private
                          │      ownership
                          │
                continuity ────────────▶ change"#.to_string(),
            specimen_caption: "three axes, one coordinate. every item belongs to exactly one.".to_string(),
            ..Project::new(
                "polyaxis",
                "a three-axis political assessment — ownership, change and authority measured separately, reported with confidence intervals. no accounts, no cookies.",
                &["Politics", "Statistics", "Web"],
                "LIVE",
                "work",
                Some("https://polyaxis.link/"),
                None,
            )
        },
        Project {
            slug: "this-website".to_string(),
            summary: "the page you are on: a rust server, a hand-written ascii water simulation, and no front-end framework.".to_string(),
            facts: vec![
                fact("year", "2026"),
                fact("role", "everything"),
                fact("stack", "rust, axum, minijinja"),
                fact("front end", "hand-written css + js"),
                fact("deploy", "uberspace"),
            ],
            links: vec![link("github", "https://github.com/peanutmow/website/")],
            sections: vec![
                note(
                    "the server",
                    &[
                        "axum serves the pages, minijinja renders the templates on the server, and the blog reads a self-hosted writefreely instance instead of framing it. static directories carry their own cache headers. templates are read from disk at startup and compiled into the binary as a fallback.",
                        "there is no client-side router. each page is a document, and every internal link is a normal link.",
                    ],
                    &[],
                    None,
                ),
                note(
                    "the water",
                    &[
                        "the landing page is an ascii water simulation drawn into a <pre>. the grid is sized from the viewport, the characters are the pixels, and typing stirs the surface. the same simulation runs dimmed behind the blog, with the landing instance paused while it is off screen.",
                        "the artwork is avif. the grain is an inline svg turbulence filter, not an image request.",
                    ],
                    &[],
                    None,
                ),
                note(
                    "two names, one binary",
                    &[
                        "the host header picks the identity. the personal site keeps the easter eggs, the social links and the blog; the professional mirror hides everything that would tie the two names together. canonical redirects are resolved before a page is rendered, so a request never lands on the wrong identity.",
                    ],
                    &[
                        "server-side html with minijinja",
                        "plain css, no preprocessor",
                        "vanilla javascript, no bundler",
                        "one deploy, two domains",
                    ],
                    Some("a site about the water should be built out of the same characters the water is made of."),
                ),
            ],
            specimen: r#"$ curl -s localhost:8080/ | head
<pre id="ascii-water"></pre>

$ curl -s localhost:8080/projects
~/projects
projects[04]
● live / 04
○ dreaming / 02

GET /            water, menu, easter eggs
GET /projects    field notes
GET /projects/*  one page per thing
GET /gallery     photographs
GET /socials     accounts and links
GET /music       a playlist kept on spotify
GET /blog        self-hosted writefreely
GET /about       one page, no cards
GET /faq         actual answers"#.to_string(),
            specimen_caption: "the route table, typed by hand like everything else.".to_string(),
            ..Project::new(
                "This website",
                "my corner of the internet, where I display my projects, socials, and my love for art.",
                &["Rust", "Axum", "Web"],
                "LIVE",
                "work",
                None,
                Some("https://github.com/peanutmow/website/"),
            )
        },
        Project {
            slug: "marx-knowledge-base".to_string(),
            summary: "a fully-local rag assistant over 125+ works of marx, engels and lenin. every answer resolves to the passage it came from.".to_string(),
            facts: vec![
                fact("year", "2026"),
                fact("role", "full build"),
                fact("stack", "python, llamaindex"),
                fact("retrieval", "chroma + bm25"),
                fact("model", "qwen3.5:9b"),
                fact("licence", "MIT"),
            ],
            links: vec![link("github", "https://github.com/peanutmow/marx-knowledge-base")],
            sections: vec![
                note(
                    "the pipeline",
                    &[
                        "the question is condensed with the chat history so follow-ups keep their subject. retrieval runs twice at once, vector search over chroma and bm25 keyword search, fused by reciprocal rank. an optional local cross-encoder re-scores the candidates before qwen3.5:9b reads them and writes the answer in streamlit.",
                        "the corpus, the embeddings, the re-ranker and the model all live on the machine. nothing is sent to a third party.",
                    ],
                    &[],
                    None,
                ),
                note(
                    "citations that resolve",
                    &[
                        "answers carry markers like [1] or [2, 4] that render as superscript links to a footnote with author, work, year, section and a link back to marxists internet archive. a marker that does not map to a retrieved passage is dropped and reported instead of being rendered as a dead link.",
                    ],
                    &[],
                    Some("an answer without a source is a rumour with better grammar."),
                ),
                note(
                    "when it does not know",
                    &[
                        "if the model reports that the passages are insufficient, the app widens retrieval once — more passages, a larger candidate pool, no similarity floor — and retries. if the second pass still fails, it says so instead of guessing.",
                    ],
                    &[],
                    None,
                ),
                note(
                    "measuring it",
                    &[
                        "25 curated questions drive the evaluation harness. hit@k asks whether the right work appeared at all; mrr@k asks how high it ranked. on this corpus hybrid retrieval lifts hit@6 from about 48% to 68%, and the re-ranker lifts mrr from 0.49 to 0.62.",
                        "a second mode runs the full retrieve-cite-answer pipeline against a judge model and reports faithfulness, answer relevancy, citation validity and retry rate.",
                    ],
                    &[
                        "scraper rate-limited to 0.6s between pages, with an identifying user-agent",
                        "corpus is public domain or cc by-sa, downloaded for research",
                        "a hardware wizard writes per-machine settings instead of guessing",
                        "streamlit on localhost, no hosted service",
                    ],
                    None,
                ),
            ],
            specimen: r#"your question
     │
     ▼
condensed with chat history
     │
     ▼
hybrid retrieval        chroma ⊕ bm25
     │
     ▼
cross-encoder re-ranker (optional)
     │
     ▼
qwen3.5:9b · ollama
     │
     ▼
streamlit · cited answer"#.to_string(),
            specimen_caption: "question to citation, with local models at every step.".to_string(),
            ..Project::new(
                "marx-knowledge-base",
                "a fully-local ai assistant over 125+ works of marx, engels & lenin. hybrid retrieval, cited answers, zero cloud.",
                &["Python", "RAG", "Ollama"],
                "LIVE",
                "work",
                None,
                Some("https://github.com/peanutmow/marx-knowledge-base"),
            )
        },
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
