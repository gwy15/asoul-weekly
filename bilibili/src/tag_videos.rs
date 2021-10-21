use biliapi::requests::{self, BiliResponseExt, Request, VideoInfo};

/// 一个 tag 下的视频
#[derive(Debug, Deserialize, Clone)]
pub struct TagVideos {
    // pub info
    pub news: TagNews,
    // pub similar:
}

#[derive(Debug, Deserialize, Clone)]
pub struct TagNews {
    pub archives: Vec<VideoInfo>,
    // count: u64,
}

impl Request for TagVideos {
    type Args = u64;
    fn request(client: &reqwest::Client, args: Self::Args) -> requests::RequestResponse<Self> {
        let req = client
            .get("https://api.bilibili.com/x/tag/detail")
            .query(&[("pn", 1), ("ps", 20), ("tag_id", args)])
            .send();
        Box::pin(async move { req.await?.bili_data().await })
    }
}

#[cfg(test)]
#[tokio::test]
async fn test_tag() {
    let tag_id = 1712619;
    let client = biliapi::connection::new_client().unwrap();
    let tag_info = TagVideos::request(&client, tag_id).await.unwrap();
    assert_eq!(tag_info.news.archives.len(), 20);
}
