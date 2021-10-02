//! 并发下载图片
use crate::zhuanlan::{dynamic_detail::DynamicDetail, items::Element};
use crate::MAX_SIZE;
use anyhow::*;
use biliapi::Request;
use bilibili::tag_feed::*;

fn get_size(w: usize, h: usize) -> (usize, usize) {
    let (w, h) = (w as f64, h as f64);
    // 宽高都最高 MAX_SIZE
    let ratio = (MAX_SIZE as f64 / w).min(MAX_SIZE as f64 / h).min(1.0);
    let ans = ((ratio * w) as usize, (ratio * h) as usize);
    debug!("ratio = {:.2}, render size: {:?}", ratio, ans);
    ans
}

async fn get_dynamic(dynamic_url: String, client: reqwest::Client) -> Result<Vec<Element>> {
    let dynamic_id = dynamic_url.replace("https://t.bilibili.com/", "");
    info!("获取动态信息 {}", dynamic_url);
    let info = match DynamicDetail::request(&client, dynamic_id).await {
        Ok(info) => info,
        Err(e) => {
            error!(
                "拉取动态信息失败，可能是已经删除了动态。动态链接: {}\n{:?}",
                dynamic_url, e
            );
            return Ok(vec![]);
        }
    };

    if info.card.desc.r#type != 2 {
        warn!("dynamic type != 2, but = {}", info.card.desc.r#type);
        return Ok(vec![]);
    }
    let picture_dynamic = match serde_json::from_str::<PictureDynamic>(&info.card.inner) {
        Ok(picture_dynamic) => Dynamic::<PictureDynamic> {
            desc: info.card.desc,
            inner: picture_dynamic,
        },
        Err(e) => {
            warn!("type = 2，但是解析动态错误：{:?}", e);
            return Ok(vec![]);
        }
    };
    info!(
        "获取动态信息完成 {}，开始下载图片以获取图片大小信息",
        dynamic_url
    );
    let uname = picture_dynamic.desc.user_profile.info.uname;
    let pic_src = picture_dynamic.inner.pictures[0].src.clone();
    debug!("图片链接：{}", pic_src);
    // 获取图片大小
    let r = client.get(&pic_src).send().await?;
    if let Some(size) = r.content_length() {
        if size > 1_000_000 {
            info!(
                "图片大小：{:.2} MiB，可能需要下载一会儿",
                size as f64 / 1024. / 1024.
            );
        }
    }
    let pic = r.bytes().await?;
    let (width, height) = match imagesize::blob_size(&pic) {
        Ok(dim) => {
            info!("动态 {} 图片大小：{:?}", dynamic_url, dim);
            get_size(dim.width, dim.height)
        }
        Err(e) => {
            warn!("无法获取图片大小: {:?}", e);
            (MAX_SIZE, MAX_SIZE)
        }
    };

    Ok(vec![
        Element::figure(
            pic_src,
            width,
            height,
            pic.len(),
            "".to_string(),
        ),
        Element::raw(format!(
            "<p style=\"text-align: right;\"><a href=\"{}\"><span class=\"color-gray-02 font-size-12\">↑ @{}（{}图）点我跳转原作品动态  &gt</span></a></p>",
            dynamic_url,
            uname,
            picture_dynamic.inner.pictures.len()
        ))
    ])
}

pub async fn download_dynamics(
    dynamics: Vec<String>,
    client: &reqwest::Client,
) -> Result<Vec<Element>> {
    info!("共计 {} 动态，开始并发下载", dynamics.len());

    let tasks: Vec<_> = dynamics
        .into_iter()
        .map(move |url| async move { get_dynamic(url, client.clone()).await })
        .collect();

    let fut = futures::future::try_join_all(tasks);
    let all_elements: Vec<Vec<_>> = fut.await?;
    let elements = all_elements.into_iter().fold(vec![], |mut a, b| {
        a.extend(b);
        a
    });
    Ok(elements)
}
