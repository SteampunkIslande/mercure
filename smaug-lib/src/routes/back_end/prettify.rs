use anyhow::Context;
use rocket::{State, get};
use sqlx::{Row, SqlitePool};

use crate::auth::AuthError;

#[get("/prettify/seqname/<seqname>")]
pub async fn prettify_seqname(
    pool: &State<SqlitePool>,
    seqname: &str,
) -> Result<String, AuthError> {
    let pretty_name = sqlx::query(
        "SELECT sequencer_pretty_name FROM SequencerPrettyName WHERE sequencer_name = ?",
    )
    .bind(seqname)
    .fetch_one(pool as &SqlitePool)
    .await;

    match pretty_name {
        Ok(row) => Ok(row
            .try_get::<String, &str>("sequencer_pretty_name")
            .unwrap_or_else(|_| seqname.to_string())),
        // An unmapped sequencer is not an error: the caller displays its raw name.
        Err(sqlx::Error::RowNotFound) => Ok(seqname.to_string()),
        Err(e) => {
            Err(e).with_context(|| format!("Impossible de traduire le séquenceur `{seqname}`"))?
        }
    }
}
