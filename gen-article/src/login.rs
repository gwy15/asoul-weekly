//! 这个实例进行登录并查询一个需要登录的接口
use std::{path::Path, sync::Arc, time::Duration};

use anyhow::*;
use biliapi::Request;
use cookie_store::CookieStore;
use qrcode::{render::unicode, QrCode};
use reqwest::Client;
use reqwest_cookie_store::CookieStoreMutex;
use tokio::{fs, io::AsyncWriteExt, time::sleep};

/// 从文件解析出 CookieStore
async fn load_persisted_cookie_store_from_file(path: impl AsRef<Path>) -> Result<CookieStore> {
    let cookies = fs::read_to_string(path.as_ref()).await?;
    CookieStore::load_json(cookies.as_bytes()).map_err(|e| anyhow!(e))
}

/// 从 cookie 文件获取持久化的客户端
pub async fn persisted_client(
    cookie_json: impl AsRef<Path>,
) -> Result<(Client, Arc<CookieStoreMutex>)> {
    let cookies = load_persisted_cookie_store_from_file(cookie_json)
        .await
        .unwrap_or_else(|e| {
            warn!("failed to load cookies: {:?}", e);
            CookieStore::default()
        });

    let cookies = Arc::new(CookieStoreMutex::new(cookies));

    let client = reqwest::ClientBuilder::new()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.164 Safari/537.36")
        .cookie_provider(Arc::clone(&cookies))
        .build()?;

    Ok((client, cookies))
}

/// 通过二维码登录一个客户端
pub async fn login(client: &Client, timeout_seconds: u32) -> Result<()> {
    let request = biliapi::requests::QrLoginRequest::request(client, ()).await?;

    let code = QrCode::new(request.url.as_bytes())?;
    let string = code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .build();
    info!("请使用哔哩哔哩手机客户端扫描下面的二维码以登录");
    println!("{}", string);

    // 先等5秒
    sleep(Duration::from_secs(5)).await;
    // 每隔3秒轮询一次，最多轮询20次
    let mut retry: u32 = 0;
    loop {
        sleep(Duration::from_secs(3)).await;
        let resp =
            biliapi::requests::CheckQrLogin::request(client, request.oauth_key.clone()).await?;
        match resp.is_success() {
            Some(true) => {
                info!("登录成功！");
                break;
            }
            Some(false) => {
                bail!("二维码登录失败");
            }
            None => {
                if retry > timeout_seconds / 3 {
                    bail!("二维码登录超时了");
                }
            }
        }
        retry += 1;
    }
    Ok(())
}

/// 将 cookies store 持久化
pub async fn save_cookies(cookies: Arc<CookieStoreMutex>, path: impl AsRef<Path>) -> Result<()> {
    // let mut f = std::fs::File::create("persisted_cookies.json")?;
    let mut buffer = &mut vec![];
    {
        let c = cookies.lock().unwrap();
        c.save_json(&mut buffer).map_err(|e| anyhow!(e))?;
    }
    let mut f = tokio::fs::File::create(path.as_ref()).await?;
    f.write_all(buffer).await?;
    Ok(())
}
