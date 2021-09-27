use biliapi::requests::{BiliResponseExt, Request};

use super::items::Element;

#[derive(Debug, Deserialize)]
pub struct SaveDraft {
    pub aid: i64,
}

pub struct Draft {
    pub title: String,
    /// 标题图，url，默认为空
    pub banner_url: String,
    /// xml 内容
    pub content: Vec<Element>,
    /// 简写的简介
    pub summary: String,
    /// 需要额外的 csrf 参数，因为 reqwest 拿不到这个参数
    pub csrf: String,
}
impl Draft {
    pub fn content_string(&self) -> String {
        let mut s = String::new();
        for e in self.content.iter() {
            s += &e.to_string();
        }
        s
    }
}

impl Request for SaveDraft {
    type Args = Draft;
    fn request(
        client: &reqwest::Client,
        args: Self::Args,
    ) -> biliapi::requests::RequestResponse<Self> {
        let url = "https://api.bilibili.com/x/article/creative/draft/addupdate";

        let content = args.content_string();

        let r = client
            .post(url)
            .form(&json!({
                "title": args.title,
                "banner_url": args.banner_url,
                "content": content,
                "summary": args.summary,
                "words": content.len(),
                "category": 15, // 生活类
                "list_id": 454181,// 枝江日报
                "tid": 4,
                "reprint": 0,
                "tags": "A-SOUL,ASOUL,向晚,贝拉,珈乐,乃琳,嘉然,虚拟偶像,虚拟主播",
                "image_urls": "//i0.hdslb.com/bfs/article/8359893082773ac25ead9765a5ef5e913d0a7eb9.png",
                "origin_image_urls": "http://article.biliimg.com/bfs/article/8a4c5b2a8bc917c374252adc8ac51d2ddaf22c68.png",
                "dynamic_intro": "",
                "media_id": 0,
                "spoiler": 0,
                "original": 1, // 原创
                "top_video_bvid": "",
                "csrf": args.csrf,
            }))
            .send();
        Box::pin(async move {
            let r = r.await?.bili_data().await?;
            Ok(r)
        })
    }
}
