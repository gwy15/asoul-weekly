use super::error::*;
use crate::db;
use actix_web::{
    delete, get, patch, post,
    web::{self, Json},
};
use anyhow::anyhow;
use log::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Category {
    category: String,
}

#[post("/items/{id}/category")]
async fn post_category(
    id: web::Path<(String,)>,
    data: web::Json<Category>,
    db: web::Data<db::Pool>,
) -> Result<Json<impl Serialize>> {
    let id = id.into_inner().0;
    info!("id = {}", id);
    let category = data.into_inner().category;
    info!("set category = {}", category);
    if db::Item::from_id(&id, &db).await?.is_some() {
        return Err(Error(anyhow!("数据库已经存在 id 为 {} 的条目", id)));
    }
    // create time 猜一个当天
    let create_time = chrono::Utc::now();
    let item = db::Item {
        id: id.clone(),
        json: "".to_string(),
        message_id: "".to_string(),
        create_time,
        category: Some(category.clone()),
        author: "unknown".to_string(),
    };
    item.insert(&db).await?;
    db::Item::set_category(&id, &category, "HTTP API", &db).await?;
    Ok(Json(json!({
        "msg": "ok"
    })))
}

#[get("/items/{id}/category")]
async fn get_category(
    id: web::Path<(String,)>,
    pool: web::Data<db::Pool>,
) -> Result<Json<impl Serialize>> {
    let id = id.into_inner().0;
    info!("get category: id = {}", id);
    match db::Item::from_id(&id, &pool).await? {
        Some(item) => Ok(Json(json!({
            "category": item.category
        }))),
        None => Err(anyhow!("数据库不存在该条目").into()),
    }
}

#[patch("/items/{id}/category")]
async fn patch_category(
    id: web::Path<(String,)>,
    data: web::Json<Category>,
    db: web::Data<db::Pool>,
) -> Result<Json<impl Serialize>> {
    let id = id.into_inner().0;
    info!("id = {}", id);
    let category = data.into_inner().category;
    info!("set category = {}", category);
    if db::Item::from_id(&id, &db).await?.is_none() {
        return Err(Error(anyhow!("数据库不存在 {} 的条目", id)));
    }
    db::Item::set_category(&id, &category, "HTTP API", &db).await?;
    Ok(Json(json!({
        "msg": "ok"
    })))
}

#[delete("/items/{id}/category")]
async fn remove_category(
    id: web::Path<(String,)>,
    db: web::Data<db::Pool>,
) -> Result<Json<impl Serialize>> {
    let id = id.into_inner().0;
    info!("id = {}, remove category", id);
    if db::Item::from_id(&id, &db).await?.is_none() {
        return Err(Error(anyhow!("数据库不存在 {} 的条目", id)));
    }
    db::Item::remove_category(&id, &db).await?;
    Ok(Json(json!({
        "msg": "ok"
    })))
}
