use anyhow::*;
use biliapi::Request;
use chrono::{Timelike, Utc};
use chrono_tz::Asia::Shanghai;
use std::{collections::HashMap, time::Duration};
use tokio::time;

use crate::config::CONFIG;
use crate::{bilibili::tag_feed::*, biz, db, feishu::FeishuClient};

pub async fn fetch_forever(client: FeishuClient, pool: db::Pool) -> ! {
    loop {
        info!("开始拉取动态");
        if let Err(e) = run_once(&client, pool.clone()).await {
            error!("拉取动态失败: {:?}", e);
        }

        // 1~8点五分钟刷一次，其他时间3分钟一次
        if matches!(Utc::now().with_timezone(&Shanghai).hour(), 1..=8) {
            time::sleep(Duration::from_secs(5 * 60)).await;
        } else {
            time::sleep(Duration::from_secs(3 * 60)).await;
        }
    }
}

fn filter_map(card: Dynamic<String>) -> Option<Dynamic<PictureDynamic>> {
    if card.inner.contains("解锁专属粉丝卡片，使用专属粉丝装扮") {
        return None;
    }
    // 图片
    if card.desc.r#type != 2 {
        return None;
    }
    let picture_dynamic = match serde_json::from_str::<PictureDynamic>(&card.inner) {
        Ok(picture_dynamic) => Dynamic::<PictureDynamic> {
            desc: card.desc,
            inner: picture_dynamic,
        },
        Err(e) => {
            warn!("type = 2，但是解析动态错误：{:?}", e);
            return None;
        }
    };
    Some(picture_dynamic)
}

async fn get_all_tags(client: &FeishuClient) -> Result<Vec<Dynamic<PictureDynamic>>> {
    let mut dynamics = HashMap::new();

    let mut tick = tokio::time::interval(Duration::from_secs(1));
    for (tag_name, _tag_id) in CONFIG.watch_tags.iter() {
        let original_size = dynamics.len();
        // tick.tick().await;
        info!("获取 tag {} 下动态", tag_name);
        // // 拿热门线
        // let mut tag_dynamics = TagFeedNew::request(&client.client, tag_name.to_string()).await?;
        // // 这里面的没用，都是叔叔选的热门消息，要拿 offset 去拿新的
        // // 反转了，最后一条有用
        // if let Some(last) = tag_dynamics.cards.pop() {
        //     if let Some(d) = filter_map(last) {
        //         dynamics.insert(d.desc.dynamic_id, d);
        //     }
        // }

        // let offset = tag_dynamics.offset;
        // 反转了，这里可以直接用 0 去拿
        let offset = "0".to_string();
        tick.tick().await;
        // 拿时间线
        let tag_dynamics = TagFeedHistory::request(
            &client.client,
            TagFeedHistoryArgs {
                topic_name: tag_name.to_string(),
                offset_dynamic_id: offset,
            },
        )
        .await?;

        for card in tag_dynamics.cards.into_iter().filter_map(filter_map) {
            dynamics.insert(card.desc.dynamic_id, card);
        }
        info!(
            "tag {} got {} new dynamics.",
            tag_name,
            dynamics.len() - original_size
        );
    }
    let mut dynamics: Vec<_> = dynamics.into_values().collect();
    info!("got {} dynamics from all tags.", dynamics.len());
    dynamics.sort_unstable_by_key(|d| d.desc.timestamp);

    Ok(dynamics)
}

async fn filter_new_dynamics(
    pool: &db::Pool,
    dynamics: Vec<Dynamic<PictureDynamic>>,
) -> Vec<Dynamic<PictureDynamic>> {
    let mut ans = vec![];
    for d in dynamics {
        let sent: bool = db::Item::is_sent(&d.desc.dynamic_id.to_string(), pool)
            .await
            .unwrap_or(false);
        if !sent {
            ans.push(d);
        }
    }
    ans
}

async fn run_once(client: &FeishuClient, pool: db::Pool) -> Result<()> {
    let group = biz::group::create_group("动态筛选", client, &pool).await?;

    // 拉动态
    let dynamics = get_all_tags(client).await?;
    let dynamics = filter_new_dynamics(&pool, dynamics).await;
    info!("new dynamics: {}", dynamics.len());

    // 发送到飞书
    for dynamics in dynamics.chunks(10) {
        // 按批发送
        let mut items = vec![];
        for dynamic in dynamics {
            info!("新动态：{}", dynamic.desc.dynamic_id);
            let body = biz::cards::dynamic_card(dynamic, client).await?;
            items.push((dynamic, body));
        }

        if items.is_empty() {
            info!("没有新动态");
            return Ok(());
        }
        // 发送
        let bodies = items.iter().map(|(_, b)| b.clone()).collect();
        let card = biz::cards::wrap_card_body(biz::cards::merge_body(bodies));

        let sent = client.send_card(&group.chat_id, card.clone()).await?;
        let message_id = sent.message_id;
        info!("message id = {}", message_id);

        info!("发送批动态完毕，本批 {} 动态", items.len());

        for (dynamic, body) in items {
            let item = db::Item {
                id: dynamic.desc.dynamic_id.to_string(),
                json: serde_json::to_string(&body)?,
                message_id: message_id.clone(),
                create_time: dynamic.desc.timestamp,
                category: None,
                author: dynamic.desc.user_profile.info.uname.clone(),
            };
            item.insert(&pool).await?;
        }
        info!("保存动态信息到 DB 完成");
    }

    Ok(())
}
