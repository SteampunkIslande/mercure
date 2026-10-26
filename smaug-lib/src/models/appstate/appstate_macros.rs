use std::str::FromStr;

use rocket::serde::Serialize;
use serde::ser::SerializeSeq;

/// Core trait for all configuration items (both Enums and Dates)
pub trait ConfigItem:
    Sized + FromStr<Err = anyhow::Error> + std::fmt::Display + Send + Sync + 'static
{
    const DB_KEY: &'static str;
    const DESCRIPTION: &'static str;
    const DEFAULT_STR: &'static str;

    fn default_value() -> Self;
    fn value_input() -> ValueInput;
}

/// Input type for rendering HTML form elements in the Admin UI.
/// Only implements Serialize (for API responses), not Deserialize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueInput {
    /// Renders an HTML `<select>` with predefined options
    Combo(&'static [&'static str]),
    /// Renders an HTML `<input type="date">`
    Date,
}

// Manual Serialize implementation because &'static [&'static str] doesn't implement Deserialize
impl Serialize for ValueInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ValueInput::Combo(allowed) => {
                let mut seq = serializer.serialize_seq(Some(allowed.len()))?;
                for &item in *allowed {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            ValueInput::Date => serializer.serialize_str("date"),
        }
    }
}

impl ValueInput {
    /// Renders the appropriate HTML input for this config value.
    pub fn render_html(&self, name: &str, current_value: &str) -> String {
        match self {
            ValueInput::Combo(allowed) => {
                let mut html = format!(r#"<select class="config-select" name="{name}">"#);
                for &opt in *allowed {
                    let selected = if opt.eq_ignore_ascii_case(current_value) {
                        " selected"
                    } else {
                        ""
                    };
                    html.push_str(&format!(
                        r#"<option value="{opt}"{selected}>{opt}</option>"#
                    ));
                }
                html.push_str("</select>");
                html
            }
            ValueInput::Date => {
                format!(
                    r#"<input type="date" class="config-input" name="{name}" value="{}">"#,
                    html_escape::encode_text(current_value)
                )
            }
        }
    }
}
