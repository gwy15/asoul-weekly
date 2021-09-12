//! 获取 tag 下的动态
use biliapi::{requests::BiliResponseExt, Request};
use chrono::{DateTime, Utc};
use serde::Deserialize;

/// 一个 tag 下的综合消息，这个会拉取“热门消息”，最后一条动态会是最新的一条消息，
/// 可以拿这个 id 去拉取按时间排序的消息（[`TagFeedHistory`] 接口）
///
/// https://api.vc.bilibili.com/topic_svr/v1/topic_svr/topic_new?topic_id=1712619
#[derive(Debug, Deserialize, Clone)]
pub struct TagFeedNew {
    pub cards: Vec<Dynamic<String>>,

    /// 最近一条动态的 offset，可以用来查询 topic_history 接口
    pub offset: String,
}

/// 按时间排序的动态接口
///
/// https://api.vc.bilibili.com/topic_svr/v1/topic_svr/topic_history?
/// ?topic_name=A-SOUL&offset_dynamic_id=555109649540124338
///
#[derive(Debug, Deserialize, Clone)]
pub struct TagFeedHistory {
    pub cards: Vec<Dynamic<String>>,

    /// 最后一条动态的 offset，可以用来查询下一批
    pub offset: String,
}

pub struct TagFeedHistoryArgs {
    pub topic_name: String,
    pub offset_dynamic_id: String,
}

/// 对应一个动态，注意这里 `Card` 初始是 string，需要根据 `desc.type` 来确定类型
#[derive(Debug, Deserialize, Clone)]
pub struct Dynamic<InnerType> {
    /// 动态的描述信息，包含 type 等
    pub desc: DynamicDesc,

    /// 内部 content，原始传过来的是 String，需要根据 desc.type 来判断转换
    #[serde(rename = "card")]
    pub inner: InnerType,
    // pub extend_json: String,
    // display:
}

/// 带有图片的动态，对应 type = 4
#[derive(Debug, Clone)]
pub struct PictureDynamic {
    pub category: String,
    /// 正文内容
    pub description: String,
    pub id: u64,
    pub pictures: Vec<DynamicPicture>,
    pub upload_time: DateTime<Utc>,
}

impl<'de> Deserialize<'de> for PictureDynamic {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Debug, Deserialize)]
        struct Helper {
            item: Inner,
        }
        #[derive(Debug, Deserialize)]
        struct Inner {
            pub category: String,
            /// 正文内容
            pub description: String,
            pub id: u64,
            pub pictures: Vec<DynamicPicture>,
            #[serde(with = "chrono::serde::ts_seconds")]
            pub upload_time: DateTime<Utc>,
        }
        let helper = Helper::deserialize(deserializer)?;
        Ok(PictureDynamic {
            category: helper.item.category,
            description: helper.item.description,
            id: helper.item.id,
            pictures: helper.item.pictures,
            upload_time: helper.item.upload_time,
        })
    }
}

/// 动态里面的图片
#[derive(Debug, Deserialize, Clone)]
pub struct DynamicPicture {
    #[serde(rename = "img_height")]
    pub height: u64,
    #[serde(rename = "img_width")]
    pub width: u64,
    #[serde(rename = "img_src")]
    pub src: String,
    // #[serde(rename = "img_size")]
    // pub size: f64,
}

/// 动态的 desc 部分，包含 uid 等
#[derive(Debug, Deserialize, Clone)]
pub struct DynamicDesc {
    pub uid: u64,
    /// type = 1: 转发视频
    /// type = 2: 图片动态
    /// type = 4: 纯文字动态
    /// type = 8: 视频动态
    /// type = 64: 文章专栏
    pub r#type: i64,

    /// 不知道干啥的
    // rid: u64,

    /// 猜测是阅读数
    pub view: u64,
    /// 转发数
    pub repost: u64,
    // comment: u64,
    pub like: u64,
    pub dynamic_id: u64,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: DateTime<Utc>,

    pub user_profile: UserProfile,
}

/// 动态发布人的信息
#[derive(Debug, Deserialize, Clone)]
pub struct UserProfile {
    pub info: UserInfo,
}

#[derive(Debug, Deserialize, Clone)]
pub struct UserInfo {
    pub uid: i64,
    pub uname: String,
    pub face: String,
}

impl Request for TagFeedNew {
    type Args = String;
    fn request(
        client: &reqwest::Client,
        args: Self::Args,
    ) -> biliapi::requests::RequestResponse<Self> {
        let req = client
            .get("https://api.vc.bilibili.com/topic_svr/v1/topic_svr/topic_new")
            .query(&[("topic_name", args)])
            .send();
        Box::pin(async move { req.await?.bili_data().await })
    }
}

impl Request for TagFeedHistory {
    type Args = TagFeedHistoryArgs;
    fn request(
        client: &reqwest::Client,
        args: Self::Args,
    ) -> biliapi::requests::RequestResponse<Self> {
        let req = client
            .get("https://api.vc.bilibili.com/topic_svr/v1/topic_svr/topic_history")
            .query(&[
                ("topic_name", args.topic_name),
                ("offset_dynamic_id", args.offset_dynamic_id),
            ])
            .send();
        Box::pin(async move { req.await?.bili_data().await })
    }
}

#[cfg(test)]
#[tokio::test]
async fn test_tag_feed() {
    let client = biliapi::connection::new_client().unwrap();
    let tag_feed = TagFeedNew::request(&client, "A-SOUL".to_string())
        .await
        .unwrap();
    println!("offset: {}", tag_feed.offset);
    // println!("tags: {:#?}", tag_feed);
    println!(
        "types: {:?}",
        tag_feed
            .cards
            .iter()
            .map(|t| t.desc.r#type)
            .collect::<Vec<_>>()
    );
    let non_video = tag_feed
        .cards
        .into_iter()
        .filter(|t| t.desc.r#type != 8)
        .filter_map(|t| {
            let card = t.inner;
            let card: PictureDynamic = match serde_json::from_str(&card) {
                Ok(e) => Some(e),
                Err(e) => {
                    log::error!("failed parse card to DynamicCard: {:?}", e);
                    None
                }
            }?;
            Some(Dynamic {
                desc: t.desc,
                inner: card,
                // extend_json: t.extend_json,
            })
        })
        .collect::<Vec<_>>();
    println!("{:#?}", non_video);
    println!("offset: {}", tag_feed.offset);
}
