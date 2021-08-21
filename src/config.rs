use anyhow::Result;
use std::collections::HashMap;
use std::{io::Read, path::Path};

lazy_static::lazy_static! {
    // SAFETY: 程序在启动的时候会载入配置，这里直接 unwrap 不会 panic
    pub static ref CONFIG: Config = Config::from_file("./config.toml").expect("载入配置文件失败");
}

#[derive(Debug, Deserialize)]
pub struct Config {
    /// 监听的 http 地址
    pub http_addr: String,
    /// sqlite 协议
    pub sqlite_url: String,
    /// 对视频的分类
    pub video_categories: Vec<String>,
    /// 监控的 tag，tag 名 => tag id
    pub watch_tags: HashMap<String, u64>,
    /// 飞书的配置
    pub feishu: FeishuConfig,
}

#[derive(Debug, Deserialize)]
pub struct FeishuConfig {
    pub app_id: String,
    pub app_secret: String,
    /// 建群拉人的时候的初始 user id
    pub init_user_ids: Vec<String>,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let mut f = std::fs::File::open(path.as_ref())?;

        let mut content = String::new();
        f.read_to_string(&mut content)?;

        let c = toml::from_str(&content)?;
        Ok(c)
    }
}
