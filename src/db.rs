use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::time::Duration;

use crate::config::get_mercure_config;

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
        CREATE TABLE IF NOT EXISTS Formdef (
            form_id INTEGER PRIMARY KEY AUTOINCREMENT,
            pipeline_name TEXT NOT NULL,
            launcher_name TEXT NOT NULL,
            form_name TEXT NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            version INTEGER NOT NULL DEFAULT 1
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS FormdefHasGroup (
            form_id INTEGER NOT NULL,
            group_id INTEGER NOT NULL,
            PRIMARY KEY (form_id, group_id),
            FOREIGN KEY (form_id) REFERENCES Formdef(form_id) ON DELETE CASCADE,
            FOREIGN KEY (group_id) REFERENCES Groups(group_id) ON DELETE CASCADE
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
        CREATE TABLE IF NOT EXISTS UDV (
            udv_id INTEGER PRIMARY KEY AUTOINCREMENT,
            form_id INTEGER NOT NULL,
            varname TEXT NOT NULL,
            default_values TEXT,
            type TEXT NOT NULL CHECK (type IN ('FromValuesList', 'Constant', 'RunDefined')),
            FOREIGN KEY (form_id) REFERENCES Formdef(form_id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS Runs (
            run_id INTEGER PRIMARY KEY AUTOINCREMENT,
            form_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            run_name TEXT NOT NULL,
            run_date TEXT NOT NULL,
            creation_date TEXT NOT NULL,
            run_sequencer TEXT NOT NULL,
            run_flowcellid TEXT NOT NULL,
            sample_sheet_adn_path TEXT NOT NULL,
            sample_sheet_arn_path TEXT NOT NULL,
            metadata_path TEXT NOT NULL,
            status TEXT NOT NULL,
            user_defined_vars TEXT NOT NULL,
            attempt_count INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (form_id) REFERENCES Formdef(form_id) ON DELETE CASCADE,
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
            run_date TEXT NOT NULL,
            run_sequencer TEXT NOT NULL,
            run_flowcellid TEXT NOT NULL,
            sample_sheet_adn_path TEXT NOT NULL,
            sample_sheet_arn_path TEXT NOT NULL,
            metadata_path TEXT NOT NULL,
            status TEXT NOT NULL,
            comment TEXT NOT NULL DEFAULT '',
            FOREIGN KEY (run_id) REFERENCES Runs(run_id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn init_db() -> Result<SqlitePool, sqlx::Error> {
    let database_url: String = get_mercure_config().mercure_db;
    init_db_from_url(&database_url).await
}

pub async fn init_db_from_url(url: &str) -> Result<SqlitePool, sqlx::Error> {
    let pool = create_pool(url).await?;
    run_migrations(&pool).await?;
    Ok(pool)
}
