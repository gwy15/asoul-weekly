use std::collections::HashMap;

use crate::{
    biz::{self, cards::wrap_card_body},
    db, FeishuClient,
};
use actix_web::web;
use anyhow::*;
use chrono::{DateTime, Utc};
use serde_json::Value;

/// 飞书对 bind 传回来的 data
#[derive(Debug, Deserialize)]
pub struct BindData {
    pub challenge: String,
    token: String,
    r#type: String,
}

/// 飞书对 action 传过来的数据
#[derive(Debug, Deserialize)]
pub struct ActionData {
    open_message_id: String,
    token: String,
    action: Action,
}

/// action 细分
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Action {
    Select(SelectAction),
    Button(ButtonAction),
}

/// 多选的 action
#[derive(Debug, Deserialize)]
struct SelectAction {
    option: String,
    tag: String,
    value: HashMap<String, String>,
}

/// 按钮的 action
#[derive(Debug, Deserialize)]
struct ButtonAction {
    tag: String,
    value: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CallbackData {
    Bind(BindData),
    Action(ActionData),
}

pub async fn new_body(
    action: ActionData,
    pool: &db::Pool,
    feishu_client: web::Data<crate::FeishuClient>,
) -> Result<Vec<Value>> {
    let (item_new_body, item, is_video) = match action.action {
        // 选择类型一定是视频类的
        Action::Select(s) => {
            // 标记类型
            let bvid = s.value["bvid"].to_string();
            let category = s.option;
            db::Item::set_category(&bvid, &category, pool).await?;
            // 修改
            info!("getting card body {} from redis", bvid);
            let item = db::Item::from_id(&bvid, pool).await?;
            let v: Vec<Value> = serde_json::from_str(&item.json)?;
            info!("card body {} got.", bvid);

            (biz::cards::video_to_accepted(v, category), item, true)
        }
        // 按键可能是动态的通过，但是历史遗留，也可能是视频的
        Action::Button(b) => {
            if b.value.get("type").map(|s| s.as_str()) == Some("dynamic") {
                let dynamic_id = b.value["dynamic_id"].to_string();
                db::Item::set_category(&dynamic_id, "ok", pool).await?;

                // 修改
                let item = db::Item::from_id(&dynamic_id, pool).await?;
                let v: Vec<Value> = serde_json::from_str(&item.json)?;
                info!("card body of dynamic {} got.", dynamic_id);

                (biz::cards::dynamic_to_ok(v), item, false)
            } else {
                // 历史遗留代码，对视频的拒绝
                let bvid = b.value["bvid"].to_string();

                db::Item::set_category(&bvid, "deny", pool).await?;
                // 修改
                info!("getting card body {} from redis", bvid);
                let item = db::Item::from_id(&bvid, pool).await?;
                let v: Vec<Value> = serde_json::from_str(&item.json)?;
                info!("card body {} got.", bvid);

                (biz::cards::video_to_denied(v), item, true)
            }
        }
    };
    let card_body = serde_json::to_string(&item_new_body)?;
    db::Item::set_json(&item.id, &card_body, pool).await?;
    info!("new card body set.");

    // 把归档结果发送到归档群
    let _pool = pool.clone();
    tokio::spawn(async move {
        if let Err(e) = send_notice(
            is_video,
            _pool,
            &feishu_client,
            item_new_body,
            item.create_time,
        )
        .await
        {
            error!("发送归档信息失败：{:?}", e);
        }
    });

    // 返回新的卡片
    let mut bodies = vec![];
    let all_json = db::Item::all_item_json(&action.open_message_id, pool).await?;
    for json in all_json {
        let body: Vec<Value> = serde_json::from_str(&json)?;
        bodies.push(body);
    }
    Ok(biz::cards::merge_body(bodies))
}

async fn send_notice(
    is_video: bool,
    pool: db::Pool,
    feishu_client: &FeishuClient,
    body: Vec<Value>,
    time: DateTime<Utc>,
) -> Result<()> {
    let group_name = if is_video {
        "视频归档"
    } else {
        "动态归档"
    };
    let group = biz::group::create_group_with_time(group_name, time, &pool, feishu_client).await?;
    feishu_client
        .send_card(&group.chat_id, wrap_card_body(body))
        .await?;
    Ok(())
}

// /// 异步接口
// #[allow(unused)]
// async fn update(
//     action: ActionData,
//     redis_pool: web::Data<crate::RedisPool>,
//     feishu_client: web::Data<crate::FeishuClient>,
// ) -> Result<()> {
//     let token = action.token.clone();
//     let body = new_body(action, &redis_pool, feishu_client.clone()).await?;
//     let data = json!({
//         "token": token,
//         "card": {
//             "open_ids": [],
//             "elements": body
//         }
//     });
//     feishu_client.update_card(data).await?;

//     info!("延迟更新卡片完成");
//     Ok(())
// }
