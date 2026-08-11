use axum::response::{Html, IntoResponse, Response};
use minijinja::Environment;
use serde::Serialize;

pub struct TemplateEngine {
    env: Environment<'static>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        let mut env = Environment::new();
        env.add_template("index.html", include_str!("../templates/index.html")).unwrap();
        env.add_template("gallery.html", include_str!("../templates/gallery.html")).unwrap();
        env.add_template("socials.html", include_str!("../templates/socials.html")).unwrap();
        env.add_template("projects.html", include_str!("../templates/projects.html")).unwrap();
        env.add_template("blog_post.html", include_str!("../templates/blog_post.html")).unwrap();
        env.add_template("etchosts.html", include_str!("../templates/etchosts.html")).unwrap();
        env.add_template("changelog.html", include_str!("../templates/changelog.html")).unwrap();
        env.add_template("man.html", include_str!("../templates/man.html")).unwrap();
        env.add_template("rawtext.html", include_str!("../templates/rawtext.html")).unwrap();
        TemplateEngine { env }
    }

    pub fn render(&self, name: &str, ctx: &impl Serialize) -> String {
        self.env.get_template(name).unwrap().render(ctx).unwrap()
    }

    pub fn render_response(&self, name: &str, ctx: &impl Serialize) -> Response {
        match self.env.get_template(name).unwrap().render(ctx) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Template error: {}", e);
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response()
            }
        }
    }
}
