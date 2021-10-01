mod category;
mod error;

use std::collections::HashMap;

use crate::{biz, db, FeishuClient};
use actix_web::{
    get, post,
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
struct DateQuery {
    t: DateTime<Utc>,
}

#[get("/summary")]
async fn summary(
    data: web::Query<DateQuery>,
    db: web::Data<db::Pool>,
) -> Result<Json<impl Serialize>> {
    let t = data.into_inner().t;
    let map = biz::summary::categorized(t, &db).await?;
    Ok(Json(map))
}

#[get("/kpi")]
async fn get_kpi(
    data: web::Query<DateQuery>,
    db: web::Data<db::Pool>,
    feishu_client: web::Data<FeishuClient>,
) -> Result<Json<impl Serialize>> {
    let date = data.into_inner().t;

    let kpi = db::Item::get_kpi(date, &db).await?;
    let users = feishu_client.get_users_in_tenant().await?;
    let user_id_to_name: HashMap<String, String> =
        users.into_iter().map(|u| (u.user_id, u.name)).collect();

    let mut result = vec![];
    for (user_id, times) in kpi {
        match user_id_to_name.get(&user_id) {
            Some(name) => result.push(json!({
                "name": name,
                "times": times,
            })),
            None => result.push(json!({
                "name": "？？？",
                "times": times
            })),
        }
    }

    Ok(Json(result))
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
            .service(get_kpi)
            .service(category::post_category)
            .service(category::patch_category)
            .service(category::remove_category)
            .service(category::get_category)
            .app_data(db_pool.clone())
            .app_data(feishu_client.clone())
    })
    .bind(addr)?
    .run()
    .await?;
    Ok(())
}
