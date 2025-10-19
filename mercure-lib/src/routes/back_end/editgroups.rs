use rocket::{State, post, serde::json::Json};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{
    auth::Authenticated,
    models::{Group, HgFormDef},
    routes::ApiResponse,
};

#[derive(Deserialize)]
pub struct GroupUpdateForm {
    to_remove: Vec<Group>,
    to_add: Vec<Group>,
    user_id: i64,
}

#[post("/groups/update", data = "<data>")]
pub async fn update_groups(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    data: Json<GroupUpdateForm>,
) -> Json<ApiResponse<String>> {
    if !auth.user.is_admin {
        return Json(ApiResponse::error("Accès non autorisé"));
    }
    match Group::set_user_groups(pool, data.0.user_id, data.0.to_remove, data.0.to_add).await {
        Ok(()) => Json(ApiResponse::success(String::from(
            "Groupes mis à jour avec succès",
        ))),
        Err(e) => Json(ApiResponse::error(format!("{}", e))),
    }
}

#[derive(Deserialize)]
pub struct FormGroupEdit {
    form_id: i64,
    group_ids: Vec<i64>,
}

#[post("/formgroupedit", data = "<data>")]
pub async fn edit_form_groups(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    data: Json<FormGroupEdit>,
) -> Json<ApiResponse<String>> {
    if !auth.user.is_admin {
        return Json(ApiResponse::error("Accès non autorisé"));
    }
    match HgFormDef::set_form_groups(pool, data.form_id, data.group_ids.clone()).await {
        Ok(()) => Json(ApiResponse::success(String::from(
            "Groupes du formulaire mis à jour avec succès.",
        ))),
        Err(e) => Json(ApiResponse::error(format!("{}", e))),
    }
}
