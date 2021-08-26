use asoul_weekly::bilibili::tag_feed::Dynamic;
use biliapi::{requests::BiliResponseExt, Request};

#[derive(Debug, Deserialize)]
pub struct DynamicDetail {
    pub card: Dynamic<String>,
}

impl Request for DynamicDetail {
    type Args = String;
    fn request(
        client: &reqwest::Client,
        args: Self::Args,
    ) -> biliapi::requests::RequestResponse<Self> {
        let url = "https://api.vc.bilibili.com/dynamic_svr/v1/dynamic_svr/get_dynamic_detail";
        let req = client.get(url).query(&[("dynamic_id", &args)]).send();
        Box::pin(async move {
            let r = req.await?.bili_data().await?;
            Ok(r)
        })
    }
}
