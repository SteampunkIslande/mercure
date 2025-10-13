use rocket::FromForm;
use serde::{Deserialize, Serialize};
use sqlx;
use sqlx::{Row, SqlitePool};

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, FromForm)]
pub struct Group {
    pub id: i64,
    pub name: String,
}

impl Group {
    /// Retrieves all groups from the database with their IDs and names
    pub async fn get_groups_with_ids(pool: &SqlitePool) -> Result<Vec<Group>, sqlx::Error> {
        let rows = sqlx::query("SELECT group_id, group_name FROM Groups")
            .fetch_all(pool)
            .await?;

        let groups = rows
            .into_iter()
            .filter_map(|row| {
                Some(Group {
                    id: row.try_get("group_id").ok()?,
                    name: row.try_get("group_name").ok()?,
                })
            })
            .collect();

        Ok(groups)
    }

    pub async fn get_user_groups(
        pool: &SqlitePool,
        user_id: i64,
    ) -> Result<Vec<Group>, sqlx::Error> {
        Ok(sqlx::query(
            r#"SELECT g.group_name, gh.group_id, gh.user_id
FROM Groups g
JOIN GroupHasUser gh ON g.group_id = gh.group_id
WHERE gh.user_id = ?; "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .filter_map(|row| {
            Some(Group {
                id: row.try_get("group_id").ok()?,
                name: row.try_get("group_name").ok()?,
            })
        })
        .collect())
    }

    pub async fn set_user_groups(
        pool: &SqlitePool,
        user_id: i64,
        to_remove: Vec<Group>,
        to_add: Vec<Group>,
    ) -> Result<(), sqlx::Error> {
        for g in to_remove {
            sqlx::query(r#"DELETE FROM GroupHasUser WHERE group_id = ? AND user_id = ? "#)
                .bind(g.id)
                .bind(user_id)
                .execute(pool)
                .await?;
        }
        for g in to_add {
            sqlx::query(r#"INSERT INTO GroupHasUser (group_id,user_id) VALUES (?,?)"#)
                .bind(g.id)
                .bind(user_id)
                .execute(pool)
                .await?;
        }
        Ok(())
    }

    /// Adds a new group to the database and returns its ID
    pub async fn add_group(pool: &SqlitePool, group_name: &str) -> Result<i64, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO Groups (group_name)
            VALUES (?)
            RETURNING group_id
            "#,
        )
        .bind(group_name)
        .fetch_one(pool)
        .await?;

        let group_id: i64 = row.try_get("group_id")?;
        Ok(group_id)
    }
}
