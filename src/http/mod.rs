mod error;

use crate::{biz, db, FeishuClient};
use actix_web::{
    delete, get, patch, post,
    web::{self, Json},
    App, HttpServer,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;

use biz::callback::CallbackData;
use error::*;

#[post("/callback")]
async fn callback(
    // data: Json<CallbackData>,
    data: Json<Value>,
    db_pool: web::Data<db::Pool>,
    feishu_client: web::Data<FeishuClient>,
) -> Result<Json<Value>> {
    let data = data.into_inner();

    debug!("callback data: {}", data.to_string());
    let data: CallbackData = serde_json::from_value(data).map_err(anyhow::Error::from)?;

    let j = match data {
        CallbackData::Bind(b) => json!({
            "challenge": b.challenge
        }),
        CallbackData::Action(action) => {
            info!("action: {:?}", action);

            // 同步更新
            match biz::callback::new_body(action, &db_pool, feishu_client).await {
                Ok(new_body) => json!({ "elements": new_body }),
                Err(e) => {
                    error!("获取新卡片失败：{:?}", e);
                    return Err(e.into());
                }
            }
        }
    };
    Ok(Json(j))
}

#[derive(Debug, Deserialize)]
struct SummaryQuery {
    t: DateTime<Utc>,
}

#[get("/summary")]
async fn summary(
    data: web::Query<SummaryQuery>,
    db: web::Data<db::Pool>,
) -> Result<Json<impl Serialize>> {
    let t = data.into_inner().t;
    let map = biz::summary::categorized(t, &db).await?;
    Ok(Json(map))
}

#[derive(Debug, Deserialize)]
struct Category {
    category: String,
}

#[patch("/items/{id}/category")]
async fn set_category(
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

pub async fn main(
    addr: impl std::net::ToSocketAddrs,
    feishu_client: crate::FeishuClient,
    db_pool: db::Pool,
) -> anyhow::Result<()> {
    let db_pool = web::Data::new(db_pool);
    let feishu_client = web::Data::new(feishu_client);
    HttpServer::new(move || {
        App::new()
            .service(callback)
            .service(summary)
            .service(set_category)
            .service(remove_category)
            .app_data(db_pool.clone())
            .app_data(feishu_client.clone())
    })
    .bind(addr)?
    .run()
    .await?;
    Ok(())
}
