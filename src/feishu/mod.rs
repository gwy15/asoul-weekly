use std::{collections::HashSet, sync::Arc};

use anyhow::{Context, Result};
use parking_lot::RwLock;
use reqwest::Client;
use serde_json::Value;

mod token_manager;
pub use token_manager::TokenManager;

mod helpers;
pub use helpers::*;

const AUTHORIZATION: &str = "Authorization";

#[derive(Debug, Deserialize)]
pub struct Group {
    pub chat_id: String,
    pub avatar: String,
    pub name: String,
    pub description: String,
    // pub owner_id: String,
    // pub owner_id_type: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GroupUser {
    pub name: String,
    pub member_id: String,
    pub member_id_type: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct User {
    pub name: String,
    pub open_id: String,
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SentMessage {
    pub message_id: String,
    // pub root_id: String,
    // pub parent_id: String,
    // pub msg_type: String,
    // pub chat_id: String,
}

#[derive(Clone)]
pub struct FeishuClient {
    pub client: Client,
    token: Arc<RwLock<String>>,
}
impl FeishuClient {
    pub fn new(token: Arc<RwLock<String>>) -> Self {
        Self {
            client: Client::new(),
            token,
        }
    }
    fn token(&self) -> String {
        format!("Bearer {}", self.token.read())
    }
    pub async fn get_groups(&self) -> Result<Vec<Group>> {
        // FIXME: 这里需要处理翻页
        let url = "https://open.feishu.cn/open-apis/im/v1/chats?page_size=100";
        let r = self
            .client
            .get(url)
            .header(AUTHORIZATION, self.token())
            .send()
            .await?;
        let groups: DataResponse<Page<Group>> = r.json().await?;
        let groups = groups.ok()?;
        Ok(groups.into_inner())
    }

    pub async fn create_group(&self, name: &str) -> Result<Group> {
        info!("新建群组 {}", name);
        let url = "https://open.feishu.cn/open-apis/im/v1/chats";
        let r = self
            .client
            .post(url)
            .header(AUTHORIZATION, self.token())
            .json(&json!({
                "name": name,
                "chat_mode": "group",
                "chat_type": "public",
                "membership_approval": "no_approval_required",
                "add_member_permission": "all_members"
            }))
            .send()
            .await?;
        let g: DataResponse<Group> = r.json().await?;
        let g = g.ok()?;
        Ok(g)
    }

    pub async fn get_or_create_group(&self, name: &str) -> Result<Group> {
        let groups = self.get_groups().await?;
        for g in groups {
            if g.name == name {
                return Ok(g);
            }
        }
        debug!("没找到群名为 {} 的群聊，新建群聊。", name);
        self.create_group(name).await
    }

    pub async fn add_user_to_group(&self, user_ids: &[String], chat_id: &str) -> Result<()> {
        debug!("adding users {:?} to chat {}", user_ids, chat_id);
        let url = format!(
            "https://open.feishu.cn/open-apis/im/v1/chats/{}/members",
            chat_id
        );
        let r = self
            .client
            .post(url)
            .header(AUTHORIZATION, self.token())
            .query(&[("member_id_type", "user_id")])
            .json(&json!({ "id_list": user_ids }))
            .send()
            .await?;
        let r: DataResponse<Value> = r.json().await?;
        r.ok()?;
        Ok(())
    }

    pub async fn get_group_users(&self, chat_id: &str) -> Result<Vec<GroupUser>> {
        let url = format!(
            "https://open.feishu.cn/open-apis/im/v1/chats/{}/members",
            chat_id
        );
        let r = self
            .client
            .get(url)
            .header(AUTHORIZATION, self.token())
            .query(&[("member_id_type", "user_id"), ("page_size", "100")])
            .send()
            .await?;
        let r: DataResponse<Page<GroupUser>> = r.json().await?;
        let r = r.ok()?.into_inner();
        Ok(r)
    }

    pub async fn ensure_users_in_group(&self, user_ids: Vec<String>, chat_id: &str) -> Result<()> {
        let target: HashSet<String> = user_ids.into_iter().collect();

        let users = self.get_group_users(chat_id).await?;
        debug!("已经在群的用户：{:?}", users);
        let current: HashSet<String> = users.into_iter().map(|u| u.member_id).collect();

        let desired: Vec<_> = target.difference(&current).cloned().collect();
        if !desired.is_empty() {
            info!("把 {:?} 拉进群 {}", desired, chat_id);
            self.add_user_to_group(&desired, chat_id)
                .await
                .context("拉人进群失败")?;
        }
        Ok(())
    }

    #[allow(unused)]
    pub async fn get_all_users(&self) -> Result<Vec<User>> {
        let url = "https://open.feishu.cn/open-apis/contact/v3/users";
        let r = self
            .client
            .get(url)
            .header(AUTHORIZATION, self.token())
            .query(&[("user_id_type", "user_id"), ("page_size", "100")])
            .send()
            .await?;
        let r: DataResponse<Page<User>> = r.json().await?;
        let r = r.ok()?.into_inner();
        Ok(r)
    }

    pub async fn send_card(&self, chat_id: &str, card: Value) -> Result<SentMessage> {
        // let url = "https://open.feishu.cn/open-apis/im/v1/messages?receive_id_type=chat_id";
        let url = "https://open.feishu.cn/open-apis/message/v4/send/";
        let r = self
            .client
            .post(url)
            .header(AUTHORIZATION, self.token())
            .json(&json!({
                // "receive_id": receiver,
                "chat_id": chat_id,
                "msg_type": "interactive",
                "card": card,
                "update_multi": true,
            }))
            .send()
            .await?;
        let r: DataResponse<SentMessage> = r.json().await?;
        let r = r.ok()?;
        Ok(r)
    }

    /// 返回 img key
    pub async fn upload_image(&self, url: &str) -> Result<String> {
        use reqwest::multipart;
        // download to mem
        let bytes = self.client.get(url).send().await?.bytes().await?;
        debug!(
            "image downloaded, size = {:.2} MiB",
            bytes.len() as f64 / 1024. / 1024.
        );

        let form = multipart::Form::new()
            .text("image_type", "message")
            .part("image", multipart::Part::bytes(bytes.to_vec()));

        let url = "https://open.feishu.cn/open-apis/im/v1/images";
        let r = self
            .client
            .post(url)
            .header(AUTHORIZATION, self.token())
            .multipart(form)
            .send()
            .await?;

        #[derive(Debug, Deserialize)]
        struct R {
            image_key: String,
        }
        let r: DataResponse<R> = r.json().await?;
        let r = r.ok().context("upload image to feishu failed.")?;
        debug!("image {} uploaded, img key = {}", url, r.image_key);
        Ok(r.image_key)
    }

    #[allow(unused)]
    pub async fn update_card(&self, data: Value) -> Result<()> {
        info!("calling update card API");
        let url = "https://open.feishu.cn/open-apis/interactive/v1/card/update";
        let r = self
            .client
            .post(url)
            .header(AUTHORIZATION, self.token())
            .json(&data)
            .send()
            .await?
            .text()
            .await?;
        info!("response: {:?}", r);
        Ok(())
    }
}
