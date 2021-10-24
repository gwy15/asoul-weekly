use anyhow::*;
use log::*;

const URLS: &[&'static str] = &[
    "https://i0.hdslb.com/bfs/album/e09ca4a57584181ce573e45079b524ff3859b9fb.jpg",
    "https://i0.hdslb.com/bfs/album/7f9c77bf727dd9f3be37bbe90dde40b6b14c208a.jpg",
    "https://i0.hdslb.com/bfs/album/c27261c312df1187339795ef744c70850476a516.jpg",
    "https://i0.hdslb.com/bfs/album/04950b09e5654eafa7a9b2285448d281d4008038.jpg",
    "https://i0.hdslb.com/bfs/album/e005114d1b88af00493c28aaf4c16a6a7ae7e23c.jpg",
    "https://i0.hdslb.com/bfs/album/3e2bf0e4c288f90d641030306a4f2535888ded8f.jpg",
    "https://i0.hdslb.com/bfs/album/4f9553b65c15a5ae2392183ece0fee652d688463.jpg",
    "https://i0.hdslb.com/bfs/album/0bad4891ec21aec2e889ddbc0b8d84df63416216.jpg",
    "https://i0.hdslb.com/bfs/album/fe8fc133a40abb93138e5b8c794faf64ba41e0ef.jpg",
];

#[tokio::test]
async fn test_584996793651183014() -> Result<()> {
    pretty_env_logger::try_init().ok();
    let mut image_download_futures = vec![];

    async fn download_image(url: String) -> Result<Vec<u8>> {
        let bytes = reqwest::Client::new()
            .get(url)
            .send()
            .await?
            .bytes()
            .await?;
        Ok(bytes.to_vec())
    }

    // let client = biliapi::connection::new_client().unwrap();

    for url in URLS {
        image_download_futures.push(download_image(url.to_string()));
    }
    let image_bytes = match futures::future::try_join_all(image_download_futures).await {
        Ok(result) => result,
        Err(e) => {
            error!("动态的某张图片下载失败了：{:?}", e);
            return Ok(());
        }
    };
    debug!(
        "图片下载完毕，一共下载了 {} 图，大小 {:.2} MiB",
        image_bytes.len(),
        image_bytes.iter().map(|i| i.len()).sum::<usize>() as f64 / 1024. / 1024.
    );

    let _merged_image_bytes = match merge_images::merge(&image_bytes) {
        Ok(bytes) => bytes,
        Err(e) => {
            warn!("合并图片失败,使用fallback图片：{:?}", e);
            return Ok(());
        }
    };
    debug!("图片合并成功");
    Ok(())
}
