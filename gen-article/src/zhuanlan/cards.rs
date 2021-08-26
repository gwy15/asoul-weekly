//! 获取视频信息

use std::collections::HashMap;

use biliapi::{requests::BiliResponseExt, Request};
use reqwest::header;

#[derive(Debug, Deserialize)]
pub struct Cards {
    #[serde(flatten)]
    pub map: HashMap<String, Card>,
}
#[derive(Debug, Deserialize)]
pub struct Card {
    pub aid: u64,
    pub pic: String,
}

impl Request for Cards {
    type Args = Vec<String>;
    fn request(
        client: &reqwest::Client,
        args: Self::Args,
    ) -> biliapi::requests::RequestResponse<Self> {
        let url = "https://api.bilibili.com/x/article/cards";
        let r = client
            .get(url)
            .query(&[
                ("ids", args.join(",")),
                ("cross_domain", "true".to_string()),
            ])
            .header(header::ORIGIN, "https://member.bilibili.com")
            .header(header::REFERER, "https://member.bilibili.com")
            .send();
        Box::pin(async move {
            let r = r.await?.bili_data().await?;
            Ok(r)
        })
    }
}
