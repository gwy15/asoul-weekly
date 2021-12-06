//! 并发下载图片
use crate::download_image::download_and_save_picture_with_retry;
use crate::zhuanlan::{dynamic_detail::DynamicDetail, items::Element};
use crate::MAX_SIZE;
use anyhow::*;
use biliapi::Request;
use bilibili::tag_feed::*;
use bytes::Bytes;
use chrono::{DateTime, Utc};

fn resize(w: usize, h: usize) -> (usize, usize) {
    let (w, h) = (w as f64, h as f64);
    // 宽高都最高 MAX_SIZE
    let ratio = (MAX_SIZE as f64 / w).min(MAX_SIZE as f64 / h).min(1.0);
    let ans = ((ratio * w) as usize, (ratio * h) as usize);
    debug!("ratio = {:.2}, render size: {:?}", ratio, ans);
    ans
}

async fn get_dynamic(
    dynamic_url: String,
    client: reqwest::Client,
    date: DateTime<Utc>,
) -> Result<(Vec<Element>, Vec<Bytes>)> {
    let dynamic_id = dynamic_url.replace("https://t.bilibili.com/", "");
    info!("获取动态信息 {}", dynamic_url);
    let info = match DynamicDetail::request(&client, dynamic_id.clone()).await {
        Ok(info) => info,
        Err(e) => {
            error!(
                "拉取动态信息失败，可能是已经删除了动态。动态链接: {}\n{:?}",
                dynamic_url, e
            );
            return Ok((vec![], vec![]));
        }
    };

    if info.card.desc.r#type != 2 {
        warn!("dynamic type != 2, but = {}", info.card.desc.r#type);
        return Ok((vec![], vec![]));
    }
    let picture_dynamic = match serde_json::from_str::<PictureDynamic>(&info.card.inner) {
        Ok(picture_dynamic) => Dynamic::<PictureDynamic> {
            desc: info.card.desc,
            inner: picture_dynamic,
        },
        Err(e) => {
            warn!("type = 2，但是解析动态错误：{:?}", e);
            return Ok((vec![], vec![]));
        }
    };
    info!(
        "获取动态信息完成 {}，开始下载图片以获取图片大小信息",
        dynamic_url
    );
    let uname = picture_dynamic.desc.user_profile.info.uname;
    let num_of_pictures = picture_dynamic.inner.pictures.len();

    let iter = picture_dynamic.inner.pictures.into_iter();
    #[cfg(not(feature = "thumbnail"))]
    let iter = iter.take(1);

    let futures: Vec<_> = iter
        // 下载全部图片/第一张图片
        .map(|pic| {
            download_and_save_picture_with_retry(
                client.clone(),
                uname.clone(),
                dynamic_id.clone(),
                pic.src,
                date,
            )
        })
        .collect();
    let pictures = futures::future::try_join_all(futures).await?;
    let first_pic = pictures[0].clone();
    #[cfg(feature = "thumbnail")]
    let picture_bytes = pictures.into_iter().map(|p| p.1).collect::<Vec<_>>();
    #[cfg(not(feature = "thumbnail"))]
    let picture_bytes = vec![];

    let (width, height) = match imagesize::blob_size(&first_pic.1) {
        Ok(dim) => {
            info!("动态 {} 图片大小：{:?}", dynamic_url, dim);
            resize(dim.width, dim.height)
        }
        Err(e) => {
            warn!("无法获取图片大小: {:?}", e);
            (MAX_SIZE, MAX_SIZE)
        }
    };

    let elements = vec![
        Element::figure(
            first_pic.0,
            width,
            height,
            first_pic.1.len(),
            "".to_string(),
        ),
        Element::raw(format!(
            "<p style=\"text-align: right;\"><a href=\"{}\"><span class=\"color-gray-02 font-size-12\">↑ @{}（{}图）点我跳转原作品动态  &gt</span></a></p>",
            dynamic_url,
            uname,
            num_of_pictures
        ))
    ];

    Ok((elements, picture_bytes))
}

pub async fn download_dynamics(
    dynamics: Vec<String>,
    client: &reqwest::Client,
    date: DateTime<Utc>,
) -> Result<(Vec<Element>, Vec<Bytes>)> {
    info!("共计 {} 动态，开始并发下载", dynamics.len());

    let tasks: Vec<_> = dynamics
        .into_iter()
        .map(move |url| async move { get_dynamic(url, client.clone(), date).await })
        .collect();

    let fut = futures::future::try_join_all(tasks);
    let all_elements: Vec<(Vec<Element>, Vec<Bytes>)> = fut.await?;
    let (elements, images) = all_elements.into_iter().fold(
        (vec![], vec![]),
        |(mut elements, mut images), (sub_elements, img)| {
            elements.extend(sub_elements);
            images.extend(img);
            (elements, images)
        },
    );
    Ok((elements, images))
}
