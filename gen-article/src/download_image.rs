use anyhow::*;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use reqwest::Client;

#[cfg(feature = "thumbnail")]
fn picture_path(
    uname: &str,
    dynamic_id: &str,
    src: &str,
    date: DateTime<Utc>,
) -> Result<(String, String)> {
    use regex::{Regex, RegexBuilder};
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

#[cfg(feature = "thumbnail")]
async fn load_local_picture(
    uname: &str,
    dynamic_id: &str,
    src: &str,
    date: DateTime<Utc>,
) -> Result<Bytes> {
    use tokio::io::AsyncReadExt;
    let (_path, fullpath) = picture_path(uname, dynamic_id, src, date)?;

    let mut f = tokio::fs::File::open(&fullpath).await?;
    let mut buf = vec![];
    f.read_to_end(&mut buf).await?;
    Ok(Bytes::copy_from_slice(&buf))
}

#[cfg(feature = "thumbnail")]
async fn save_picture(
    bytes: Bytes,
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

async fn download_and_save_picture(
    client: Client,
    #[allow(unused)] uname: String,
    #[allow(unused)] dynamic_id: String,
    pic_src: String,
    #[allow(unused)] date: DateTime<Utc>,
) -> Result<Bytes> {
    debug!("下载并保存图片链接：{}", pic_src);

    #[cfg(feature = "thumbnail")]
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

    #[cfg(feature = "thumbnail")]
    save_picture(pic_bytes.clone(), &uname, &dynamic_id, &pic_src, date).await?;

    Ok(pic_bytes)
}

/// return (pic_src, bytes)
pub async fn download_and_save_picture_with_retry(
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
