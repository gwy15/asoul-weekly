//!

use std::convert::{TryFrom, TryInto};

use anyhow::*;
use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_tz::Asia::Shanghai;
use sqlx::{sqlite::SqlitePoolOptions, Sqlite};

pub type Pool = sqlx::Pool<Sqlite>;

pub async fn init(database_url: &str) -> Result<Pool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    Ok(pool)
}

#[derive(Clone)]
pub struct RawItem<T> {
    pub id: String,
    pub json: String,
    pub message_id: String,
    pub create_time: T,
    pub category: Option<String>,
    pub author: String,
}

/// 动态或者视频
pub type Item = RawItem<DateTime<Utc>>;

impl TryFrom<RawItem<String>> for Item {
    type Error = chrono::ParseError;
    fn try_from(item: RawItem<String>) -> Result<Self, Self::Error> {
        let t = &item.create_time;
        let create_time = NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S%.f"))
            .map(|t| DateTime::from_utc(t, Utc))?;
        Ok(Self {
            id: item.id,
            json: item.json,
            message_id: item.message_id,
            create_time,
            category: item.category,
            author: item.author,
        })
    }
}

impl Item {
    pub async fn is_sent(id: &str, pool: &Pool) -> Result<bool> {
        let cnt = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)
            FROM `item`
            WHERE
                `id` = ?;
        "#,
            id
        )
        .fetch_one(&*pool)
        .await?;

        Ok(cnt > 0)
    }

    pub async fn insert(self, pool: &Pool) -> Result<()> {
        sqlx::query!(
            r"
            INSERT INTO `item`
            (`id`, `json`, `message_id`, `create_time`, `category`, `author`)
            VALUES
            (?, ?, ?, ?, ?, ?);
            ",
            self.id,
            self.json,
            self.message_id,
            self.create_time,
            self.category,
            self.author
        )
        .execute(&*pool)
        .await?;
        Ok(())
    }

    pub async fn json_from(id: &str, pool: &Pool) -> Result<String> {
        let json = sqlx::query_scalar!(
            r"
            SELECT `json`
            FROM `item`
            WHERE `id` = ?
            LIMIT 1;
            ",
            id
        )
        .fetch_one(&*pool)
        .await?;
        Ok(json)
    }

    pub async fn from_id(id: &str, pool: &Pool) -> Result<Option<Item>> {
        let raw_item = sqlx::query_as!(
            RawItem::<String>,
            r"
            SELECT `id`, `json`, `message_id`, `create_time`, `category`, `author`
            FROM `item`
            WHERE `id` = ?
            LIMIT 1;
            ",
            id
        )
        .fetch_optional(&*pool)
        .await?;

        match raw_item {
            Some(item) => Ok(Some(item.try_into()?)),
            None => Ok(None),
        }
    }

    pub async fn set_json(id: &str, json: &str, pool: &Pool) -> Result<()> {
        sqlx::query!(
            r"
            UPDATE `item`
            SET 
                `json` = ?
            WHERE
                `id` = ?
            ",
            json,
            id
        )
        .execute(&*pool)
        .await?;
        Ok(())
    }

    pub async fn set_category(id: &str, category: &str, pool: &Pool) -> Result<()> {
        let t = Utc::now();
        sqlx::query!(
            r"
            UPDATE `item`
            SET 
                `category` = ?,
                `mark_time` = ?
            WHERE
                `id` = ?
            ",
            category,
            t,
            id
        )
        .execute(&*pool)
        .await?;
        Ok(())
    }

    pub async fn all_item_json(message_id: &str, pool: &Pool) -> Result<Vec<String>> {
        let s = sqlx::query_scalar!(
            r"
            SELECT `json`
            FROM `item`
            WHERE 
                `message_id` = ?
            ",
            message_id
        )
        .fetch_all(&*pool)
        .await?;
        Ok(s)
    }

    pub async fn all_categorized_in_date(date: DateTime<Utc>, pool: &Pool) -> Result<Vec<Self>> {
        let date = date.with_timezone(&Shanghai).date();
        let start = date.and_hms(0, 0, 0).with_timezone(&Utc);
        let start = start.format("%Y-%m-%d %H:%M:%S").to_string();
        let end = date.and_hms(23, 59, 59).with_timezone(&Utc);
        let end = end.format("%Y-%m-%d %H:%M:%S").to_string();

        let items = sqlx::query_as!(
            RawItem::<String>,
            r#"
            SELECT `id`, `json`, `message_id`, `create_time`, `category`, `author`
            FROM `item`
            WHERE
                `create_time` BETWEEN ? AND ?
                AND `category` is not null
            ORDER BY `create_time` ASC;
            "#,
            start,
            end
        )
        .fetch_all(&*pool)
        .await?;

        let mut res = vec![];
        for item in items {
            res.push(item.try_into()?);
        }
        Ok(res)
    }
}

#[derive(Debug, Clone)]
pub struct Group {
    pub name: String,
    pub chat_id: String,
}
impl Group {
    pub async fn from_name(name: &str, pool: &Pool) -> Result<Option<Group>> {
        let group = sqlx::query_as!(
            Self,
            r#"
            SELECT  `name`, chat_id
            FROM    `group`
            WHERE   `name` = ?
            LIMIT 1;
            "#,
            name
        )
        .fetch_optional(&*pool)
        .await?;
        Ok(group)
    }
    pub async fn insert(name: &str, chat_id: &str, pool: &Pool) -> Result<Self> {
        sqlx::query!(
            r"
            INSERT INTO `group`
            (`name`, `chat_id`)
            VALUES
            (?, ?);
            ",
            name,
            chat_id
        )
        .execute(&*pool)
        .await?;
        Ok(Self {
            name: name.to_string(),
            chat_id: chat_id.to_string(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    async fn _test_serde_item(t: DateTime<Utc>) {
        let id = "1dkfjgndkfjg".to_string();
        let json = "[]".to_string();
        let item = Item {
            id: id.clone(),
            json: json.to_string(),
            message_id: "asds".to_string(),
            create_time: t,
            category: None,
            author: "test author".to_string(),
        };
        let pool = init("sqlite://:memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        item.insert(&pool).await.unwrap();
        let item = Item::from_id(&id, &pool).await.unwrap();

        assert_eq!(item.json, json);
        assert_eq!(item.create_time, t);
    }

    #[tokio::test]
    async fn test_serde_item() {
        _test_serde_item("2021-07-23 03:40:28Z".parse().unwrap()).await;
        _test_serde_item(Utc::now()).await;
    }

    #[test]
    fn test_parse_t() {
        use chrono::*;
        let t = NaiveDateTime::parse_from_str("2021-07-23 03:40:28", "%Y-%m-%d %H:%M:%S").unwrap();
        let t: DateTime<Utc> = DateTime::from_utc(t, Utc);
        dbg!(t);
        dbg!(t.to_string());
    }
}
