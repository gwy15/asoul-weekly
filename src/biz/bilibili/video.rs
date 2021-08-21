use anyhow::Result;
use biliapi::{requests::Request, requests::VideoInfo};
use std::{collections::HashMap, time::Duration};
use tokio::time;

use crate::config::CONFIG;
use crate::{bilibili::tag_videos::TagVideos, biz, db, feishu::FeishuClient};

pub async fn fetch_forever(client: FeishuClient, db: db::Pool) -> ! {
    // 2分钟检查一次
    let mut interval = time::interval(Duration::from_secs(2 * 60));
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        info!("start fetch feed videos");

        if let Err(e) = run_once(&client, db.clone()).await {
            error!("failed to fetch feed videos: {:?}", e);
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
        let l = videos.len();
        videos.extend(
            tag_videos
                .news
                .archives
                .into_iter()
                // 直接筛掉转载
                .filter(|v| v.copyright != 2)
                // 筛选时长
                .filter(|v| v.duration.as_secs() >= 20)
                .map(|v| (v.bvid.clone(), v)),
        );
        info!("{} videos got for tag {}", videos.len() - l, tag_name);
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
    let group = biz::group::create_group("视频筛选", client).await?;

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
