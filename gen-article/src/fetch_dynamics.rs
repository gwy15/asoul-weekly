//! 并发下载图片
use crate::zhuanlan::{dynamic_detail::DynamicDetail, items::Element};
use crate::MAX_SIZE;
use anyhow::*;
use biliapi::Request;
use bilibili::tag_feed::*;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use regex::{Regex, RegexBuilder};
use reqwest::Client;
use tokio::io::AsyncReadExt;

fn get_size(w: usize, h: usize) -> (usize, usize) {
    let (w, h) = (w as f64, h as f64);
    // 宽高都最高 MAX_SIZE
    let ratio = (MAX_SIZE as f64 / w).min(MAX_SIZE as f64 / h).min(1.0);
    let ans = ((ratio * w) as usize, (ratio * h) as usize);
    debug!("ratio = {:.2}, render size: {:?}", ratio, ans);
    ans
}

fn picture_path(
    uname: &str,
    dynamic_id: &str,
    src: &str,
    date: DateTime<Utc>,
) -> Result<(String, String)> {
    lazy_static::lazy_static! {
        static ref EXT_PATTERN: Regex = RegexBuilder::new(r"\.(jpg|jpeg|bmp|webp|png|gif)$")
            .case_insensitive(true)
            .build().unwrap();
    }
    let ext = EXT_PATTERN
        .captures(src)
        .and_then(|cap| cap.get(1))
        .map(|mat| mat.as_str())
        .unwrap_or_else(|| {
            info!("ext not found from src {}, using default jpg", src);
            "jpg"
        });

    let filename = format!("{}.{}", dynamic_id, ext);
    let path = format!("动态图片/{}/{}", date.format("%Y-%m-%d"), uname);
    debug!("保存文件：path = {}, filename = {}", path, filename);
    let fullpath = format!("{}/{}", path, filename);
    Ok((path, fullpath))
}

async fn load_local_picture(
    uname: &str,
    dynamic_id: &str,
    src: &str,
    date: DateTime<Utc>,
) -> Result<Bytes> {
    let (_path, fullpath) = picture_path(uname, dynamic_id, src, date)?;

    let mut f = tokio::fs::File::open(&fullpath).await?;
    let mut buf = vec![];
    f.read_to_end(&mut buf).await?;
    Ok(bytes::Bytes::copy_from_slice(&buf))
}

async fn save_picture(
    bytes: bytes::Bytes,
    uname: &str,
    dynamic_id: &str,
    src: &str,
    date: DateTime<Utc>,
) -> Result<()> {
    use tokio::io::AsyncWriteExt;
    let (path, fullpath) = picture_path(uname, dynamic_id, src, date)?;

    tokio::fs::create_dir_all(&path).await?;
    let mut f = tokio::fs::File::create(&fullpath).await?;
    f.write_all(&bytes).await?;
    info!("文件 {} 写入完成", fullpath);
    Ok(())
}

/// return (pic_src, bytes)
async fn download_and_save_picture_with_retry(
    client: Client,
    uname: String,
    dynamic_id: String,
    pic_src: String,
    date: DateTime<Utc>,
) -> Result<(String, Bytes)> {
    for i in 0..3 {
        let r = download_and_save_picture(
            client.clone(),
            uname.clone(),
            dynamic_id.clone(),
            pic_src.clone(),
            date,
        )
        .await;
        match r {
            Ok(r) => return Ok((pic_src, r)),
            Err(e) => {
                warn!("下载失败，重试，已经失败 {} 次: {}", i, e);
                if i == 2 {
                    error!("下载失败");
                    return Err(e);
                }
            }
        }
    }
    unreachable!();
}

async fn download_and_save_picture(
    client: Client,
    uname: String,
    dynamic_id: String,
    pic_src: String,
    date: DateTime<Utc>,
) -> Result<Bytes> {
    debug!("下载并保存图片链接：{}", pic_src);

    if let Ok(bytes) = load_local_picture(&uname, &dynamic_id, &pic_src, date).await {
        info!("图片已在本地存在，跳过下载");
        return Ok(bytes);
    }

    // 获取图片大小
    let r = client
        .get(&pic_src)
        .send()
        .await
        .with_context(|| format!("error fetch header of the image from url {}", pic_src))?;
    if let Some(size) = r.content_length() {
        if size > 1_000_000 {
            info!(
                "图片大小：{:.2} MiB，可能需要下载一会儿",
                size as f64 / 1024. / 1024.
            );
        }
    }
    let pic_bytes = r
        .bytes()
        .await
        .with_context(|| format!("error downloading the image from url {}", pic_src))?;
    save_picture(pic_bytes.clone(), &uname, &dynamic_id, &pic_src, date).await?;
    Ok(pic_bytes)
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

    let futures: Vec<_> = picture_dynamic
        .inner
        .pictures
        .into_iter()
        // 下载全部图片/第一张图片
        .map(|pic| {
            download_and_save_picture_with_retry(
                client.clone(),
                uname.clone(),
                dynamic_id.clone(),
                pic.src.clone(),
                date,
            )
        })
        .collect();
    let pictures = futures::future::try_join_all(futures).await?;
    let first_pic = pictures[0].clone();
    let picture_bytes = pictures.into_iter().map(|p| p.1).collect::<Vec<_>>();

    let (width, height) = match imagesize::blob_size(&first_pic.1) {
        Ok(dim) => {
            info!("动态 {} 图片大小：{:?}", dynamic_url, dim);
            get_size(dim.width, dim.height)
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
            picture_bytes.len()
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
