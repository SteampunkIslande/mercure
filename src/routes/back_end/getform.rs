use rocket::{State, get, serde::json::Json};
use sqlx::SqlitePool;

use crate::{
    models::{HgFormDef, HgFormListItem},
    routes::ApiResponse,
};

// #[get("/forms")]
// pub async fn get_all_forms(pool: &State<SqlitePool>) -> Json<ApiResponse<Vec<HgFormDef>>> {
//     match HgFormDef::get_all_form_defs(pool).await {
//         Ok(alldefs) => Json(ApiResponse::success(alldefs)),
//         Err(e) => Json(ApiResponse::error(format!("{:?}", e))),
//     }
// }

#[get("/forms/groups/<group_id>")]
pub async fn get_all_forms_for_group(
    pool: &State<SqlitePool>,
    group_id: i64,
) -> Json<ApiResponse<Vec<HgFormListItem>>> {
    match HgFormDef::get_form_list_items_for_group(pool, group_id).await {
        Ok(alldefs) => Json(ApiResponse::success(alldefs)),
        Err(e) => Json(ApiResponse::error(format!("{:?}", e))),
    }
}

#[get("/forms/<id>")]
pub async fn get_form_from_id(pool: &State<SqlitePool>, id: i64) -> Json<ApiResponse<HgFormDef>> {
    match HgFormDef::get_formdef_from_id(pool, id).await {
        Ok(formdef) => Json(ApiResponse::success(formdef)),
        Err(e) => Json(ApiResponse::error(format!("{:?}", e))),
    }
}
