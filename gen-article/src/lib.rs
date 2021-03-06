#[macro_use]
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate log;

mod download_image;
mod fetch_dynamics;
mod login;
mod zhuanlan;

use anyhow::*;
use biliapi::Request;
use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use chrono_tz::Asia::Shanghai;
use log::*;
use std::{
    collections::{BTreeMap, HashMap},
    time::Duration,
};

pub use login::*;
use zhuanlan::{cards::Cards, items::Element, save_draft::*};

lazy_static::lazy_static! {
    static ref WEIGHT: HashMap<&'static str, i32> = maplit::hashmap!{
        "A-SOUL一周年庆祝" => -2,
        "乃琳50万粉祝贺" => -1,
        "音乐" => 0,
        "舞蹈" => 1,
        "手书" => 2,
        "手书/动画" => 2,
        "精剪混剪" => 3,
        "MMD" => 4,
        "发病" => 5,
        "鬼畜/整活" => 6,
        "炸厨房"=>7,
        "其他" => 8
    };
}

const MAX_SIZE: usize = 800;

fn strip(s: &str) -> String {
    let r = regex::Regex::new(r"\s*\n\s*").unwrap();
    r.replace_all(s, "").to_string()
}

async fn get_data(t: DateTime<Utc>) -> Result<BTreeMap<String, Vec<String>>> {
    let base_url = std::option_env!("ASOUL_WEEKLY_URL")
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            std::env::var("ASOUL_WEEKLY_URL")
                .expect("Missing environment variable `ASOUL_WEEKLY_URL`")
        });
    let url = format!(
        "{}/summary?t={}",
        base_url,
        t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    );
    debug!("获取归档 {}", url);
    let r = reqwest::get(url).await?.json().await?;
    Ok(r)
}

/// 返回版头，引言等
fn header(date: DateTime<Utc>) -> Vec<Element> {
    let mut ret = vec![
        Element::raw(strip(
            r#"
            <p style="text-align: center;">
                <span class="color-gray-01 font-size-12">
                一个魂们早上好呀！这里是枝江日报~本报旨在归纳前一天发生的A-SOUL相关各种资讯和二创内容，希望方便一个魂们快速浏览A-SOUL动态和二创内容。
                </span>
            </p>"#,
        )),
        Element::raw(strip(
            r#"
            <p style="text-align: center;">
                <span class="color-gray-01 font-size-12">
                B站功能提示：① 向左滑动/点击右下角菜单可以查看之前的日报；② 长按点赞按钮可以一键三连。
                    </span>
            </p>"#,
        )),
        Element::spacer(),
        Element::block_quote(strip(r#"【替换这里为版头】"#)),
    ];

    // 提醒二创参与
    let today = date.with_timezone(&Shanghai).date();
    let event_end = Shanghai.from_utc_date(&NaiveDate::from_ymd(2022, 1, 5));
    let event_remain = (event_end - today).num_days();
    info!("二创激励计划还有 {} 天", event_remain);
    if event_remain > 0 {
        ret.extend([
            Element::raw(strip(
                &format!(
                r#"
                <p style="text-align: center;">二创作者记得带上tag <strong><span class="color-yellow-04">#A-SOUL二创激励计划#</span></strong>&nbsp;参与活动</p>
                <p style="text-align: center;">活动将于&nbsp;2022年1月5日（{}天后）结束</p>
                <figure class="img-box" contenteditable="false"><img src="//article.biliimg.com/bfs/article/card/70584df3f930b1869475527a9e0fe347499dcb98.png" width="1320" height="224" data-size="33777" aid="13871002" class="article-card" type="normal"></figure><p><br></p>
                "#, event_remain)
            )),
        ]);
    }

    ret.extend([
        //
        // 成员动态
        Element::raw(strip(
            r#"
            <figure class="img-box img-seamless" contenteditable="false">
            <img src="//article.biliimg.com/bfs/article/48b650e9c9a23a594c674e595c94011b0e93af52.jpg" width="1280" height="600" data-size="86603" class="seamless" type="seamlessImage">
            </figure>
            "#,
        )),
        Element::spacer(),
        Element::block_quote(r#"【替换这里为成员动态】"#),
        Element::spacer(),
        Element::raw(strip(
            r#"
            <figure class="img-box" contenteditable="false">
            <img src="//i0.hdslb.com/bfs/article/02db465212d3c374a43c60fa2625cc1caeaab796.png" class="cut-off-6">
            </figure>"#,
        )),
        //
        // 直播动态
        Element::raw(strip(
            r#"
            <figure class="img-box img-seamless" contenteditable="false">
            <img src="//article.biliimg.com/bfs/article/843ea70a852bdea9a3536c456ff28376eaef9746.jpg" width="1280" height="600" data-size="86603" class="seamless" type="seamlessImage">
            </figure>
            "#,
        )),
        Element::spacer(),
        Element::block_quote(r#"【替换这里为直播动态，如切片GIF等，没有直播就删掉】"#),
        Element::spacer(),
        Element::raw(strip(
            r#"
            <figure class="img-box" contenteditable="false">
            <img src="//i0.hdslb.com/bfs/article/02db465212d3c374a43c60fa2625cc1caeaab796.png" class="cut-off-6">
            </figure>"#,
        )),
    ]);

    ret
}

/// 视频分版头
fn video_header() -> Vec<Element> {
    vec![Element::raw(strip(
        r#"
        <figure class="img-box" contenteditable="false">
            <img src="//article.biliimg.com/bfs/article/765b627af2487507cd4cb70db903ead4c6915f37.jpg" width="1280" height="600" data-size="127959">
            <figcaption class="caption" contenteditable=""></figcaption>
        </figure>"#,
    ))]
}

/// 视频结束分割线
fn video_end() -> Vec<Element> {
    vec![Element::raw(strip(
        r#"
        <figure class="img-box" contenteditable="false">
        <img src="//i0.hdslb.com/bfs/article/02db465212d3c374a43c60fa2625cc1caeaab796.png" class="cut-off-6">
        </figure>
        "#,
    ))]
}

/// 图片分版头
fn dynamic_header() -> Vec<Element> {
    vec![Element::raw(strip(
        r#"
        <figure class="img-box img-seamless" contenteditable="false">
            <img src="//article.biliimg.com/bfs/article/3f3eefbdeaf0d15f1bf5e37dee7462734263fb7d.jpg" width="1280" height="600" data-size="113238" class="seamless" type="seamlessImage">
        </figure>"#,
    ))]
}

fn ending() -> Vec<Element> {
    fn footnote(text: &str) -> Element {
        Element::raw(strip(&format!(
            r#"
            <p style="text-align: right;">
                <span class="color-gray-01 font-size-12">
                {}
                </span>
            </p>"#,
            text
        )))
    }

    vec![
        // 分割线
        Element::raw(strip(
            r#"
        <figure class="img-box" contenteditable="false">
            <img src="//i0.hdslb.com/bfs/article/02db465212d3c374a43c60fa2625cc1caeaab796.png" class="cut-off-6">
        </figure>"#,
        )),
        Element::Text {
            center: true,
            strong: false,
            classes: vec!["font-size-16".to_string()],
            text: "以上就是本期日报的全部内容啦！".to_string(),
        },
        Element::Text {
            center: false,
            strong: false,
            classes: vec!["font-size-16".to_string()],
            text: strip(
                r#"
                受B站专栏格式和人力限制，部分优秀二创内容无法展示完全。如果您看到有任何我们遗漏的内容，欢迎一个魂们踊跃向 @ASOUL周报 投稿内容！
                如果对枝江日报有什么好的意见和建议也可以通过私信直接向我们反馈，我们也深知目前还有很多不完善和需要改进的地方，会努力越做越好的！
            "#,
            ),
        },
        footnote(&format!("自动化：asoul-weekly {}", env!("BUILD_INFO"))),
        footnote("编辑：@大头大头大 | 二创筛选：@SkyBigBlack | 美工：@GhrotHHH"),
        footnote("日报文案：【】|  GIF制作：【】"),
    ]
}

/// 生成动态缩略图并写入本地
#[cfg(feature = "thumbnail")]
async fn generate_dynamic_thumbnail(images: Vec<bytes::Bytes>, date: DateTime<Utc>) -> Result<()> {
    use tokio::io::AsyncWriteExt;
    let image = merge_images::merge(&images)?;
    let f = format!("动态图片/{}-grid.jpg", date.format("%Y-%m-%d"));
    let mut f = tokio::fs::File::create(f).await?;
    f.write_all(&image).await?;

    let image = merge_images::waterfall(&images)?;
    let f = format!("动态图片/{}-waterfall.jpg", date.format("%Y-%m-%d"));
    let mut f = tokio::fs::File::create(f).await?;
    f.write_all(&image).await?;
    Ok(())
}

async fn gen_article_elements(
    client: &reqwest::Client,
    date: DateTime<Utc>,
    mut summary: BTreeMap<String, Vec<String>>,
) -> Result<Vec<Element>> {
    let mut interval = tokio::time::interval(Duration::from_secs(1));

    // 分类
    let dynamics = summary.remove("动态").unwrap_or_default();
    let mut videos = summary.into_iter().collect::<Vec<_>>();
    videos.sort_unstable_by_key(|(name, _)| WEIGHT.get(name.as_str()).unwrap_or(&99999));

    // 导言
    let mut elements = header(date);

    // 视频
    elements.extend(video_header());
    for (category, bvids) in videos {
        info!("处理分类 {} 视频", category);
        if category == "动态" {
            continue;
        }
        elements.push(Element::Text {
            center: false,
            strong: true,
            classes: vec!["color-blue-02".to_string(), "font-size-20".to_string()],
            text: category.to_string(),
        });
        for bvids in bvids.chunks(2) {
            info!("获取 {:?} 的 aid", bvids);
            interval.tick().await;
            let cards = Cards::request(client, bvids.to_vec()).await?;
            let aids: Vec<String> = bvids
                .iter()
                .filter_map(|b| cards.map.get(b))
                .map(|m| m.aid.to_string())
                .collect();
            info!("bvids {:?} => aid {:?}", bvids, aids);
            match aids.len() {
                0 => {
                    info!("这俩视频都被删除了，无语子，下一个");
                    continue;
                }
                1 => {
                    info!("视频被删了一个或者本身就只有一个，单列");
                }
                _ => {}
            }
            elements.push(Element::VideoLink {
                // 写死一个封面，发布的时候会自动替换
                cover: "https://i0.hdslb.com/bfs/article/card/fb4e1d78b966962a8b94037f4accb6a5ff5f3067.png".to_string(),
                width: 1320,
                height: 192,
                data_size: 61801,
                aids,
            });
        }
        elements.push(Element::spacer());
    }
    elements.extend(video_end());

    // 动态
    info!("{} 条动态", dynamics.len());
    elements.extend(dynamic_header());

    #[allow(unused)]
    let (dynamics_elements, dynamic_images) =
        fetch_dynamics::download_dynamics(dynamics, client, date).await?;
    elements.extend(dynamics_elements);

    // 结束
    elements.extend(ending());

    // 生成动态图片
    #[cfg(feature = "thumbnail")]
    generate_dynamic_thumbnail(dynamic_images, date).await?;

    Ok(elements)
}

fn date_string(t: DateTime<Utc>) -> String {
    let date_utc8 = t.with_timezone(&chrono_tz::Asia::Shanghai);
    let date = date_utc8.format("%m 月 %d 日");
    let weekday = match date_utc8.weekday() {
        chrono::Weekday::Mon => "星期一",
        chrono::Weekday::Tue => "星期二",
        chrono::Weekday::Wed => "星期三",
        chrono::Weekday::Thu => "星期四",
        chrono::Weekday::Fri => "星期五",
        chrono::Weekday::Sat => "星期六",
        chrono::Weekday::Sun => "星期日",
    };

    format!("{} {}", date, weekday)
}

pub async fn generate(client: reqwest::Client, csrf: String) -> Result<SaveDraft> {
    let date = Utc::now();

    let summary = get_data(date - chrono::Duration::days(1)).await?;
    let elements = gen_article_elements(&client, date, summary).await?;

    // 发送草稿
    let draft = Draft {
        title: format!("枝江日报（{}）", date_string(date)),
        banner_url: "".to_string(),
        content: elements,
        summary: "一个简单的总结，点开草稿会自动重新生成".to_string(),
        csrf,
    };
    let r = SaveDraft::request(&client, draft).await?;
    info!("saved draft aid = {}", r.aid);

    #[cfg(target_os = "windows")]
    notify_rust::Notification::new()
        .summary("枝江日报")
        .body(&format!("草稿生成完成, aid = {}", r.aid))
        .icon("firefox")
        .show()?;

    Ok(r)
}

#[test]
fn test_date() {
    let t = DateTime::parse_from_rfc3339("2021-10-13T11:25:00+08:00").unwrap();
    let t = t.with_timezone(&Utc);
    assert_eq!(date_string(t), "10 月 13 日 星期三")
}
