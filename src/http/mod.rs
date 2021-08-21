mod error;

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
    data: Json<CallbackData>,
    db_pool: web::Data<db::Pool>,
    feishu_client: web::Data<FeishuClient>,
) -> Result<Json<Value>> {
    let data = data.into_inner();

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
            .app_data(db_pool.clone())
            .app_data(feishu_client.clone())
    })
    .bind(addr)?
    .run()
    .await?;
    Ok(())
}
