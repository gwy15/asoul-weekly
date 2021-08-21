use anyhow::Result;
use parking_lot::RwLock;
use reqwest::Client;
use std::{sync::Arc, time::Duration};

use super::FlatResponse;

pub struct TokenManager {
    app_id: String,
    app_secret: String,
    client: Client,
    token: Arc<RwLock<String>>,
}
impl TokenManager {
    async fn force_refresh_token(&self) -> Result<()> {
        const URL: &str = "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal/";
        let r = self
            .client
            .post(URL)
            .json(&json!({
                "app_id": self.app_id,
                "app_secret": self.app_secret
            }))
            .send()
            .await?;
        #[derive(Debug, Deserialize)]
        struct Response {
            tenant_access_token: String,
        }
        let data: FlatResponse<Response> = r.json().await?;
        let r = data.ok()?;
        // info!("{}", r.tenant_access_token);
        if r.tenant_access_token == self.token.read().as_ref() {
            debug!("access token not changed.");
        } else {
            info!(
                "update access token, length = {}",
                r.tenant_access_token.len()
            );
            *self.token.write() = r.tenant_access_token;
        }
        Ok(())
    }
    pub async fn new(app_id: impl Into<String>, app_secret: impl Into<String>) -> Result<Self> {
        let this = TokenManager {
            app_id: app_id.into(),
            app_secret: app_secret.into(),
            client: Client::new(),
            token: Default::default(),
        };
        info!("initiate access token");
        this.force_refresh_token().await?;
        Ok(this)
    }
    pub async fn auto_refresh(self) -> ! {
        // 十分钟刷新一次
        let mut interval = tokio::time::interval(Duration::from_secs(10 * 60));

        loop {
            interval.tick().await;
            if let Err(e) = self.force_refresh_token().await {
                // 忽略错误
                error!("Failed to refresh token: {:?}", e);
            }
        }
    }
    pub fn token(&self) -> Arc<RwLock<String>> {
        self.token.clone()
    }
}
