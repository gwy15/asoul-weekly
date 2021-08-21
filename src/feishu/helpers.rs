use anyhow::Result;

#[derive(Debug, Deserialize)]
pub struct FlatResponse<T> {
    code: i64,
    msg: String,
    #[serde(flatten)]
    data: Option<T>,
}
impl<T> FlatResponse<T> {
    pub fn ok(self) -> Result<T> {
        match (self.code, self.data) {
            (0, Some(data)) => Ok(data),
            (code, _) => {
                bail!("Feishu error ({}) {}", code, self.msg)
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DataResponse<T> {
    code: i64,
    msg: String,
    data: Option<T>,
}
impl<T> DataResponse<T> {
    pub fn ok(self) -> Result<T> {
        match (self.code, self.data) {
            (0, Some(data)) => Ok(data),
            (code, _) => {
                bail!("Feishu error ({}) {}", code, self.msg)
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page_token: Option<String>,
    pub has_more: bool,
}
impl<T> Page<T> {
    pub fn into_inner(self) -> Vec<T> {
        self.items
    }
}
