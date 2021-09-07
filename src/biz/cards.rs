//! 生成各种卡片
use anyhow::Result;
use biliapi::requests::VideoInfo;
use chrono_tz::Asia::Shanghai;
use serde_json::Value;
use std::borrow::Cow;

use crate::{
    bilibili::tag_feed::{Dynamic, PictureDynamic},
    biz,
    config::CONFIG,
    feishu::FeishuClient,
};

type CardBody = Vec<Value>;

fn markdown_escape(s: &str) -> Cow<str> {
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
        Cow::from(s)
    }
}

/// 基础信息分行，会上传封面
async fn video_basic_info(info: &VideoInfo, client: &FeishuClient) -> Result<Value> {
    let url = format!("https://www.bilibili.com/video/{}", info.bvid);
    let img_key = client.upload_image(&info.cover_url).await?;
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
    // NOTE: 这里只上传第一张图片
    let image_url = dynamic
        .inner
        .pictures
        .first()
        .map(|p| p.src.as_str())
        .ok_or_else(|| anyhow!("动态 {} 没有图片", dynamic.desc.dynamic_id))?;

    let img_key = client.upload_image(image_url).await.unwrap_or_else(|e| {
        error!(
            "动态 {} 图片({})上传失败:{:?}",
            dynamic.desc.dynamic_id, image_url, e
        );
        // 替换成上传图片失败的 fallback
        "img_v2_1f156161-3ffa-40f7-9d28-9621cc5ed2cg".to_string()
    });

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
    Ok(b)
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
