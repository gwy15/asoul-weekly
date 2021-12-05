//! 好像并不需要上传图片

use biliapi::{requests::BiliResponseExt, Request};
use reqwest::multipart;

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct UploadImage {
    size: usize,
    url: String,
}

pub struct UploadImageArgs {
    file: Vec<u8>,
    csrf: String,
}

impl Request for UploadImage {
    type Args = UploadImageArgs;
    fn request(
        client: &reqwest::Client,
        args: Self::Args,
    ) -> biliapi::requests::RequestResponse<Self> {
        let url = "https://api.bilibili.com/x/article/creative/article/upcover";
        let form = multipart::Form::new()
            .part("binary", multipart::Part::bytes(args.file))
            .part("csrf", multipart::Part::text(args.csrf));
        let r = client.post(url).multipart(form).send();
        Box::pin(async move {
            let r = r.await?.bili_data().await?;
            Ok(r)
        })
    }
}
