use anyhow::*;
use asoul_weekly as pkg;
use pkg::config;
use pkg::feishu::{self, FeishuClient};

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::Config::from_file("./config.toml").context("Config file not found")?;
    let token_manager =
        feishu::TokenManager::new(config.feishu.app_id, config.feishu.app_secret).await?;
    let token = token_manager.token();
    // println!("{}", token.read());
    let feishu_client = FeishuClient::new(token);

    let groups = feishu_client.get_groups().await?;
    for g in groups {
        println!("{:?}", g);
        let members = feishu_client.get_group_users(&g.chat_id).await?;
        for m in members {
            println!(" {:?}", m);
        }
        println!();
    }
    Ok(())
}
