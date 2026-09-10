use log::error;
use minijinja::{Environment, Value};
use rocket::fairing::AdHoc;
use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder, content::RawHtml};
use rust_embed::RustEmbed;

// Re-export de context
pub use minijinja::context;

/// Ces assets sont contenus dans le binaire final
#[derive(RustEmbed)]
#[folder = "../static/templates"]
struct TemplateAssets;

pub struct Template {
    name: String,
    context: Value,
}

impl Template {
    /// Rend le template `name` avec le contexte `context`.
    ///
    /// Le fichier `static/templates/{name}.html.j2` doit exister, sinon une erreur est renvoyée.
    pub fn render<S: Into<String>>(name: S, context: Value) -> Self {
        Self {
            name: name.into(),
            context,
        }
    }
}

/// Implémentation de Responder pour que Rocket sache comment renvoyer ce Template
impl<'r, 'o: 'r> Responder<'r, 'o> for Template {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'o> {
        // On récupère l'environnement Minijinja depuis l'état global de Rocket, via la requête
        let env = request.rocket().state::<Environment>().ok_or_else(|| {
            error!(
                "L'environnement Minijinja n'est pas géré par Rocket. As-tu oublié le fairing ?"
            );
            Status::InternalServerError
        })?;

        // On charge le template
        let tmpl = env
            .get_template(&format!("{}.html.j2", self.name))
            .map_err(|e| {
                error!("Erreur de chargement du template '{}': {}", self.name, e);
                Status::InternalServerError
            })?;

        // On exécute le rendu avec le contexte
        let rendered = tmpl.render(self.context).map_err(|e| {
            error!("Erreur lors du rendu du template '{}': {}", self.name, e);
            Status::InternalServerError
        })?;

        // On délègue la réponse à RawHtml pour s'assurer d'avoir le bon Content-Type (text/html)
        RawHtml(rendered).respond_to(request)
    }
}

pub fn minijinja_fairing() -> AdHoc {
    AdHoc::try_on_ignite("Minijinja Embedded Templates", |rocket| async {
        let mut env = Environment::new();

        for file_path in TemplateAssets::iter() {
            if let Some(file) = TemplateAssets::get(&file_path) {
                if let Ok(content) = std::str::from_utf8(&file.data) {
                    if let Err(e) =
                        env.add_template_owned(file_path.to_string(), content.to_string())
                    {
                        error!("Erreur lors de l'ajout d'un template: {}", e);
                        return Err(rocket);
                    }
                }
            }
        }

        let rocket = rocket.manage(env);
        Ok(rocket)
    })
}
