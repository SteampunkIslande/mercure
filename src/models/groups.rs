use serde::{Deserialize, Serialize};
use sqlx;
use sqlx::{Row, SqlitePool};

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
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
}
