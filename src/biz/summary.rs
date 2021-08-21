use crate::db;
use anyhow::*;
use chrono::{DateTime, Utc};
use std::collections::BTreeMap as Map;

pub async fn categorized(date: DateTime<Utc>, pool: &db::Pool) -> Result<Map<String, Vec<String>>> {
    let items = db::Item::all_categorized_in_date(date, pool).await?;

    let mut map: Map<String, Vec<String>> = Map::new();
    for item in items {
        map.entry(item.category.unwrap()).or_default().push(item.id);
    }
    // rename ok => 动态
    if let Some(dynamics) = map.remove("ok") {
        for dynamic_id in dynamics {
            map.entry("动态".to_string())
                .or_default()
                .push(format!("https://t.bilibili.com/{}", dynamic_id));
        }
    }
    Ok(map)
}
