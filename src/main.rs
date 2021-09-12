#[macro_use]
extern crate serde;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate anyhow;

use anyhow::*;
use feishu::FeishuClient;

mod biz;
mod config;
pub mod db;
mod feishu;
mod http;

// #[tokio::main]
#[actix_web::main]
async fn main() -> Result<()> {
    if std::option_env!("DEV").is_some() {
        info!("running in dev mode.");
    }
    log4rs::init_file("./log4rs.yml", Default::default()).context("Missing log4rs.yml")?;
    let config = config::Config::from_file("./config.toml").context("Config file not found")?;

    // 连 db
    let db_pool = db::init(&config.sqlite_url).await?;
    sqlx::migrate!().run(&db_pool).await?;
    debug!("db migration ok");

    // 刷新 token
    let token_manager = feishu::TokenManager::new(config.feishu.app_id, config.feishu.app_secret)
        .await
        .context("Init token manger failed")?;
    let token = token_manager.token();
    tokio::spawn(async move { token_manager.auto_refresh().await });

    let feishu_client = FeishuClient::new(token);

    // 拉 feed 下的视频
    let _feishu = feishu_client.clone();
    let _db_pool = db_pool.clone();
    tokio::spawn(async move { biz::bilibili::video::fetch_forever(_feishu, _db_pool).await });

    // 拉 feed 动态
    let _feishu = feishu_client.clone();
    let _db_pool = db_pool.clone();
    tokio::spawn(async move { biz::bilibili::dynamic::fetch_forever(_feishu, _db_pool).await });

    http::main(config.http_addr, feishu_client, db_pool).await?;

    Ok(())
}
