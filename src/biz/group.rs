//! 拉群
use crate::config::CONFIG;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use chrono_tz::Asia::Shanghai;

use crate::feishu::{FeishuClient, Group};

pub async fn create_group_with_time(
    name: &str,
    time: DateTime<Utc>,
    client: &FeishuClient,
) -> Result<Group> {
    let dev_mode = std::option_env!("DEV").is_some();

    let date_s = time.with_timezone(&Shanghai).format("%m-%d");
    let group_name = if dev_mode {
        format!("{} dev {}", name, date_s)
    } else {
        format!("{} {}", name, date_s)
    };
    let group = client.get_or_create_group(&group_name).await?;

    let user_ids = CONFIG.feishu.init_user_ids.clone();
    client
        .ensure_users_in_group(user_ids, &group.chat_id)
        .await
        .context("确保人在群里失败")?;

    debug!("群聊：{:?}", group);
    Ok(group)
}

/// 拉一个确保我在其中的群，如 动态筛选
pub async fn create_group(name: &str, client: &FeishuClient) -> Result<Group> {
    create_group_with_time(name, Utc::now(), client).await
}
