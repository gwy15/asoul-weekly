//! 生成各种卡片
use anyhow::Result;
use biliapi::requests::VideoInfo;
use chrono_tz::Asia::Shanghai;
use regex::Regex;
use serde_json::Value;
use std::borrow::Cow;

use bilibili::tag_feed::{Dynamic, PictureDynamic};

use crate::{biz, config::CONFIG, feishu::FeishuClient};

type CardBody = Vec<Value>;

fn markdown_escape(s: &str) -> Cow<str> {
    lazy_static::lazy_static! {
        static ref R: Regex = Regex::new("\n\\s+").unwrap();
    }
    let s = R.replace_all(s, "\n");
    if s.contains(|ch: char| matches!(ch, '[' | ']' | '(' | ')')) {
        let mut ans = String::new();
        for ch in s.chars() {
            ans.push(match ch {
                '[' => '【',
                ']' => '】',
                '(' => '（',
                ')' => '）',
                _ => ch,
            })
        }
        Cow::from(ans)
    } else {
        s
    }
}

/// 基础信息分行，会上传封面
async fn video_basic_info(info: &VideoInfo, client: &FeishuClient) -> Result<Value> {
    let url = format!("https://www.bilibili.com/video/{}", info.bvid);
    let img_key = client.upload_image_url(&info.cover_url).await?;
    let up = &info.owner.name;

    let intro = format!(
        "[▷{title}]({url})\n{bvid} UP：{up}\n播放 {play}  评论 {comment}  弹幕 {danmaku}  长度 {m}:{s:02}",
        title = markdown_escape(&info.title),
        bvid = info.bvid,
        url = url,
        up = up,
        play = info.stat.view,
        comment = info.stat.reply,
        danmaku = info.stat.danmaku,
        m = info.duration.as_secs() / 60,
        s = info.duration.as_secs() % 60,
    );
    Ok(json!({
        "tag": "div",
        "text": {
            "tag": "lark_md",
            "content": intro
        },
        "extra": {
            "tag": "img",
            "img_key": img_key,
            "alt": {
                "tag": "plain_text",
                "content": "视频封面"
            }
        }
    }))
}

/// 视频的页脚，发布于
fn video_footnote(info: &VideoInfo) -> Value {
    let t = info
        .publish_at
        .with_timezone(&Shanghai)
        .format("%Y-%m-%d %H:%M:%S");
    json!({
        "tag": "note",
        "elements": [
            {
                "tag": "plain_text",
                "content": format!("发布于 {}", t)
            }
        ]
    })
}

/// 按钮
fn video_action(info: &VideoInfo) -> Value {
    let select_options: Vec<Value> = CONFIG
        .video_categories
        .iter()
        .map(|t| {
            json!({
                "text": {
                    "tag": "plain_text",
                    "content": t
                },
                "value": t
            })
        })
        .collect();

    json!({
        "tag": "action",
        "actions": [
            {
                "tag": "select_static",
                "placeholder": {
                    "tag": "plain_text",
                    "content": "选择分类"
                },
                "value": {
                    "type": "video",
                    "bvid": info.bvid
                },
                "options": select_options
            },
            // {
            //     "tag": "button",
            //     "text": {
            //         "tag": "plain_text",
            //         "content": "不通过筛选"
            //     },
            //     "type": "default",
            //     "value": {
            //         "bvid": info.bvid,
            //         "action": "deny"
            //     }
            // }
        ]
    })
}

pub fn video_to_accepted(body: CardBody, category: String) -> CardBody {
    vec![
        body[0].clone(),
        json!({
          "tag": "markdown",
          "content": format!("✔️ 已接受，分类：{}", category)
        }),
        body[2].clone(),
    ]
}

pub fn dynamic_to_ok(body: CardBody) -> CardBody {
    // 一共三段，修改中间的
    vec![
        body[0].clone(),
        json!({
            "tag": "markdown",
            "content": "✔️ 已接受"
        }),
        body[2].clone(),
    ]
}

pub fn wrap_card_body(body: CardBody) -> Value {
    json!({
        "config": { "wide_screen_mode": true },
        "i18n_elements": { "zh_cn": body }
    })
}

/// 动态卡片，会上传封面
pub async fn dynamic_card(
    dynamic: &Dynamic<PictureDynamic>,
    client: &FeishuClient,
) -> Result<CardBody> {
    let content_md = format!(
        "[{} ({} 图)](https://t.bilibili.com/{})\n{}",
        dynamic.desc.user_profile.info.uname,
        dynamic.inner.pictures.len(),
        dynamic.desc.dynamic_id,
        markdown_escape(&dynamic.inner.description)
    );
    debug!("dynamic content card markdown = {}", content_md);

    let img_key = get_dynamic_thumbnail_image_key(dynamic, client).await?;

    let t = dynamic
        .desc
        .timestamp
        .with_timezone(&Shanghai)
        .format("%Y-%m-%d %H:%M:%S");

    let b = vec![
        json!({
          "tag": "div",
          "text": {
            "tag": "lark_md",
            "content": content_md,
          },
          "extra": {
            "tag": "img",
            "img_key": img_key,
            "alt": {
              "tag": "plain_text",
              "content": "第一张图"
            }
          }
        }),
        json!({
            "tag": "action",
            "actions": [
                {
                    "tag": "button",
                    "text": {
                        "tag": "plain_text",
                        "content": "选入今日二创"
                    },
                    "type": "default",
                    "value": {
                        "type": "dynamic",
                        "dynamic_id": dynamic.desc.dynamic_id.to_string()
                    }
                },
            ]
        }),
        json!({
          "tag": "note",
          "elements": [
            {
              "tag": "plain_text",
              "content":  format!("发布于 {}", t)
            }
          ]
        }),
    ];
    debug!("card = {}", serde_json::to_string(&b).unwrap());
    Ok(b)
}

async fn get_dynamic_thumbnail_image_key(
    dynamic: &Dynamic<PictureDynamic>,
    client: &FeishuClient,
) -> Result<String> {
    // 进行一个贴图的上传
    let mut image_download_futures = vec![];
    async fn download_image(client: &FeishuClient, mut url: String) -> Result<Vec<u8>> {
        if !url.ends_with(".@512w.jpg") {
            url.push_str(".@512w.jpg");
        }
        let bytes = client.client.get(url).send().await?.bytes().await?;
        Ok(bytes.to_vec())
    }
    for pic in dynamic.inner.pictures.iter() {
        let url = pic.src.clone();
        image_download_futures.push(download_image(client, url));
    }
    let image_bytes = match futures::future::try_join_all(image_download_futures).await {
        Ok(result) => result,
        Err(e) => {
            error!("动态的某张图片下载失败了：{:?}", e);
            return Ok("img_v2_1f156161-3ffa-40f7-9d28-9621cc5ed2cg".to_string());
        }
    };
    debug!(
        "图片下载完毕，一共下载了 {} 图，大小 {:.2} MiB",
        image_bytes.len(),
        image_bytes.iter().map(|i| i.len()).sum::<usize>() as f64 / 1024. / 1024.
    );

    let merged_image_bytes = match merge_images::merge(&image_bytes) {
        Ok(bytes) => bytes,
        Err(e) => {
            warn!("合并图片失败,使用fallback图片：{:?}", e);
            return Ok("img_v2_1f156161-3ffa-40f7-9d28-9621cc5ed2cg".to_string());
        }
    };
    debug!("图片合并成功");

    let r = client
        .upload_image_bytes(merged_image_bytes)
        .await
        .unwrap_or_else(|e| {
            warn!("上传图片失败，可能是过大：{:?}", e);
            debug!("使用默认图");
            "img_v2_1f156161-3ffa-40f7-9d28-9621cc5ed2cg".to_string()
        });

    debug!("图片上传完成");
    Ok(r)
}

pub async fn video_info_to_card_body(info: &VideoInfo, client: &FeishuClient) -> Result<CardBody> {
    let basic_info_block = biz::cards::video_basic_info(info, client).await?;
    let footnote_block = biz::cards::video_footnote(info);
    let action_block = biz::cards::video_action(info);

    let body = vec![basic_info_block, action_block, footnote_block];
    Ok(body)
}

pub fn merge_body(bodies: Vec<CardBody>) -> CardBody {
    let mut combined_body = vec![];
    for (idx, body) in bodies.into_iter().enumerate() {
        if idx != 0 {
            combined_body.push(json!({ "tag": "hr" }));
        }
        combined_body.extend(body);
    }
    combined_body
}
