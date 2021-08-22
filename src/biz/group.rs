//! 拉群
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use chrono_tz::Asia::Shanghai;
use tokio::sync::Mutex;

use crate::config::CONFIG;
use crate::{db, feishu::FeishuClient};

lazy_static::lazy_static! {
    /// 代表对 group 表的写访问权限
    static ref LOCK: Mutex<()> = Mutex::new(());
}

pub async fn create_group_with_time(
    name: &str,
    time: DateTime<Utc>,
    pool: &db::Pool,
    client: &FeishuClient,
) -> Result<db::Group> {
    let dev_mode = std::option_env!("DEV").is_some();

    let date_s = time.with_timezone(&Shanghai).format("%m-%d");
    let group_name = if dev_mode {
        format!("{} dev {}", name, date_s)
    } else {
        format!("{} {}", name, date_s)
    };

    // 不加锁也可以直接读
    if let Some(group) = db::Group::from_name(&group_name, pool).await? {
        info!("从 DB 中查到群 {} 的信息，不再新拉群", group_name);
        return Ok(group);
    }
    // 不存在，创建
    let _guard = LOCK.lock().await;
    // 现在有唯一的对 group 表的访问权限了
    // TODO: 把 get_or_create_group 替换成 create_group，前者总会失败
    let feishu_group = client.get_or_create_group(&group_name).await?;
    let chat_id = feishu_group.chat_id;
    let user_ids = CONFIG.feishu.init_user_ids.clone();
    client
        .ensure_users_in_group(user_ids, &chat_id)
        .await
        .context("确保人在群里失败")?;
    // 现在插入表
    let group = db::Group::insert(&group_name, &chat_id, pool).await?;

    debug!("群聊：{:?}", group);
    Ok(group)
}

/// 拉一个确保我在其中的群，如 动态筛选
pub async fn create_group(name: &str, client: &FeishuClient, pool: &db::Pool) -> Result<db::Group> {
    create_group_with_time(name, Utc::now(), pool, client).await
}
