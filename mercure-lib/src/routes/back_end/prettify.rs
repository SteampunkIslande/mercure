use rocket::{State, get};
use sqlx::{Row, SqlitePool};

#[get("/prettify/seqname/<seqname>")]
pub async fn prettify_seqname(pool: &State<SqlitePool>, seqname: &str) -> String {
    sqlx::query("SELECT sequencer_pretty_name FROM SequencerPrettyName WHERE sequencer_name = ?")
        .bind(seqname)
        .fetch_one(pool as &SqlitePool)
        .await
        .and_then(|row| row.try_get::<String, &str>("sequencer_pretty_name"))
        .unwrap_or(seqname.to_string())
}
