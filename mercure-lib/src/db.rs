use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::time::Duration;

pub async fn create_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    SqlitePoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(database_url)
        .await
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS Users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            usermail TEXT NOT NULL UNIQUE,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            created_at DATETIME NOT NULL,
            last_login DATETIME,
            is_admin BOOLEAN NOT NULL DEFAULT 0
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS Groups (
            group_id INTEGER PRIMARY KEY AUTOINCREMENT,
            group_name TEXT NOT NULL UNIQUE
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS GroupHasUser (
            group_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            PRIMARY KEY (group_id, user_id),
            FOREIGN KEY (group_id) REFERENCES Groups(group_id) ON DELETE CASCADE,
            FOREIGN KEY (user_id) REFERENCES Users(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS Runs (
            run_id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            run_name TEXT NOT NULL,
            creation_date TEXT NOT NULL,
            status TEXT NOT NULL,
            attempt_count INTEGER NOT NULL DEFAULT 0,
            branch_name TEXT NOT NULL,
            user_defined_vars TEXT NOT NULL,
            form_path TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES Users(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS Attempts (
            attempt_number INTEGER NOT NULL,
            run_id INTEGER NOT NULL,
            attempt_date TEXT NOT NULL,
            user_defined_vars TEXT NOT NULL,
            commit_hash TEXT NOT NULL,
            status TEXT NOT NULL,
            comment TEXT NOT NULL DEFAULT '',
            FOREIGN KEY (run_id) REFERENCES Runs(run_id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS SequencerPrettyName (
            sequencer_name TEXT PRIMARY KEY,
            sequencer_pretty_name TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn init_db_from_url(url: &str) -> Result<SqlitePool, sqlx::Error> {
    let pool = create_pool(url).await?;
    run_migrations(&pool).await?;
    Ok(pool)
}
