use anyhow::Result;
use biliapi::{requests::Request, requests::VideoInfo};
use chrono::{Timelike, Utc};
use chrono_tz::Asia::Shanghai;
use std::{collections::HashMap, time::Duration};
use tokio::time;

use crate::config::CONFIG;
use crate::{bilibili::tag_videos::TagVideos, biz, db, feishu::FeishuClient};

pub async fn fetch_forever(client: FeishuClient, db: db::Pool) -> ! {
    loop {
        info!("开始拉取视频");
        if let Err(e) = run_once(&client, db.clone()).await {
            error!("failed to fetch feed videos: {:?}", e);
        }

        // 1~8点五分钟刷一次，其他时间3分钟一次
        if matches!(Utc::now().with_timezone(&Shanghai).hour(), 1..=8) {
            time::sleep(Duration::from_secs(5 * 60)).await;
        } else {
            time::sleep(Duration::from_secs(3 * 60)).await;
        }
    }
}

async fn get_all_tags(client: &FeishuClient) -> Result<Vec<VideoInfo>> {
    let mut videos = HashMap::new();
    let mut tick = tokio::time::interval(Duration::from_secs(2));
    for (tag_name, tag_id) in CONFIG.watch_tags.iter() {
        tick.tick().await;
        info!("getting videos for tag {}", tag_name);
        let tag_videos = TagVideos::request(&client.client, *tag_id).await?;
        debug!(
            "tag {} videos: {}",
            tag_name,
            tag_videos.news.archives.len()
        );
        let l = videos.len();
        videos.extend(
            tag_videos
                .news
                .archives
                .into_iter()
                // 直接筛掉转载
                .filter(|v| v.copyright != 2)
                // 筛选时长
                .filter(|v| v.duration.as_secs() >= 10)
                .map(|v| (v.bvid.clone(), v)),
        );
        info!("{} new videos got for tag {}", videos.len() - l, tag_name);
    }
    let mut videos: Vec<_> = videos.into_values().collect();
    videos.sort_unstable_by_key(|v| v.publish_at);
    Ok(videos)
}

async fn all_unsent_videos(pool: &db::Pool, videos: Vec<VideoInfo>) -> Vec<VideoInfo> {
    let mut ans = vec![];
    for v in videos {
        let sent: bool = db::Item::is_sent(&v.bvid, pool).await.unwrap_or(false);

        if !sent {
            ans.push(v);
        }
    }
    ans
}

async fn run_once(client: &FeishuClient, db: db::Pool) -> Result<()> {
    let group = biz::group::create_group("视频筛选", client, &db).await?;

    let videos = get_all_tags(client).await?;
    let videos = all_unsent_videos(&db, videos).await;
    info!("new videos: {}", videos.len());

    for videos in videos.chunks(10) {
        // 筛选
        let mut items = vec![];
        for video in videos {
            info!("新视频：[{}] {}", video.bvid, video.title);
            let body = biz::cards::video_info_to_card_body(video, client).await?;
            items.push((video, body));
        }

        if items.is_empty() {
            info!("没有新视频");
            return Ok(());
        }
        // 合并发送
        let bodies = items.iter().map(|(_, b)| b.clone()).collect();
        let card = biz::cards::wrap_card_body(biz::cards::merge_body(bodies));

        let sent = client.send_card(&group.chat_id, card.clone()).await?;
        let message_id = sent.message_id;
        debug!("message id = {}", message_id);
        info!("发送本批视频完毕，本批 {}", items.len());

        // 保存 message_id => bv 的映射
        for (video, body) in items {
            let item = db::Item {
                id: video.bvid.clone(),
                json: serde_json::to_string(&body)?,
                message_id: message_id.clone(),
                create_time: video.publish_at,
                category: None,
                author: video.owner.name.clone(),
            };
            item.insert(&db).await?;
        }
        info!("保存视频信息到 DB 完成");
    }

    Ok(())
}
