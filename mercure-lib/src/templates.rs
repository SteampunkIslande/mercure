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

#[cfg(test)]
mod tests {
    use super::*;
    use minijinja::context;

    fn load_env() -> Environment<'static> {
        let mut env = Environment::new();
        for file_path in TemplateAssets::iter() {
            let file = TemplateAssets::get(&file_path).expect("template asset");
            let content = std::str::from_utf8(&file.data).expect("utf8 template");
            env.add_template_owned(file_path.to_string(), content.to_string())
                .unwrap_or_else(|e| panic!("parse {}: {e}", file_path));
        }
        env
    }

    #[test]
    fn all_templates_parse() {
        let env = load_env();
        assert!(env.get_template("base.html.j2").is_ok());
        assert!(env.get_template("common/run_base.html.j2").is_ok());
        assert!(env.get_template("common/welcome.html.j2").is_ok());
        assert!(env.get_template("admin/dashboard.html.j2").is_ok());
        assert!(env.get_template("common/runningrun.html.j2").is_ok());
    }

    #[test]
    fn welcome_renders_from_base() {
        let env = load_env();
        let html = env
            .get_template("common/welcome.html.j2")
            .unwrap()
            .render(context! {})
            .unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Bienvenue sur Mercure"));
        assert!(html.contains("/mercure/static/styles/default.css"));
        assert!(!html.contains("Déconnexion"));
    }

    #[test]
    fn login_renders_without_banner() {
        let env = load_env();
        let html = env
            .get_template("common/login.html.j2")
            .unwrap()
            .render(context! {})
            .unwrap();
        assert!(html.contains("Connexion - Mercure"));
        assert!(html.contains("show_message.js"));
        assert!(!html.contains("banner-text"));
    }

    #[test]
    fn error_pages_render() {
        let env = load_env();
        let unauthorized = env
            .get_template("errors/unauthorized.html.j2")
            .unwrap()
            .render(context! { title => "x", h2 => "y", message => "z" })
            .unwrap();
        assert!(unauthorized.contains("class=\"bg-light flex-center vh-100\""));
        assert!(unauthorized.contains("Se connecter"));

        let admin_only = env
            .get_template("errors/admin_only.html.j2")
            .unwrap()
            .render(context! {})
            .unwrap();
        assert!(admin_only.contains("Accès protégé"));
    }

    #[test]
    fn inherited_pages_render_with_minimal_context() {
        let env = load_env();
        let pages = [
            "admin/register.html.j2",
            "admin/dashboard.html.j2",
            "admin/users_list.html.j2",
            "common/home.html.j2",
            "common/listruns.html.j2",
            "common/searchrun.html.j2",
            "common/redirect.html.j2",
            "common/error.html.j2",
            "common/pendingrun.html.j2",
            "common/successrun.html.j2",
            "common/failurerun.html.j2",
            "common/idlerun.html.j2",
            "common/runningrun.html.j2",
            "common/editrun.html.j2",
            "admin/groups_list.html.j2",
            "admin/passedit.html.j2",
        ];
        let ctx = context! {
            title => "t",
            h2 => "h",
            message => "m",
            seconds => 5,
            target_url => "/",
            user => context! { username => "u", id => 1 },
            edited_user => context! { username => "u", id => 1 },
            auth_user => context! { is_admin => false },
            user_groups => Vec::<minijinja::Value>::new(),
            is_admin => false,
            pipelines_struct => context! {},
            edit_mode => false,
            form => context! { name => "f", version => 1, indir_type => "BclDir" },
            run => context! {
                run_id => 1,
                run_name => "r",
                status => "Idle",
                user_defined_vars => context! {},
                sample_sheet_adn_path => "",
                sample_sheet_arn_path => "",
                metadata_file_path => "",
            },
            attempt => context! {
                attempt_number => 1,
                run_date => "",
                run_sequencer => "",
                run_flowcellid => "",
                indir => "",
                user_defined_vars => context! {},
            },
            history => Vec::<minijinja::Value>::new(),
            user_defined_vars_json => "{}",
            fail_reason => "",
        };
        for page in pages {
            env.get_template(page)
                .unwrap_or_else(|e| panic!("load {page}: {e}"))
                .render(ctx.clone())
                .unwrap_or_else(|e| panic!("render {page}: {e}"));
        }
    }
}
