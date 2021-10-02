#[macro_use]
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate log;

mod fetch_dynamics;
mod login;
mod zhuanlan;

use anyhow::*;
use biliapi::Request;
use chrono::{DateTime, Utc};
use log::*;
use std::{
    collections::{BTreeMap, HashMap},
    time::Duration,
};

use login::*;
use zhuanlan::{cards::Cards, items::Element, save_draft::*};

lazy_static::lazy_static! {
    static ref WEIGHT: HashMap<&'static str, i32> = maplit::hashmap!{
        "音乐" => 0,
        "舞蹈" => 1,
        "手书" => 2,
        "精剪混剪" => 3,
        "MMD" => 4,
        "发病" => 5,
        "鬼畜/整活" => 6,
        "炸厨房"=>7,
        "其他" => 8
    };
}

const MAX_SIZE: usize = 800;
const README: &str = r#"
                               @@@@@@@@/                                                                
                            %@@@@@@@@@@/                                                                
                          @@@@@@@@@@@@@/    @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@*
                       @@@@@@@@@ @@@@@@/  @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@*
                     @@@@@@@@&   @@@@@@/ @@@@@@@.                                                @@@@@@*
                  @@@@@@@@@      @@@@@@/ @@@@@@*                                                 @@@@@@*
               .@@@@@@@@&        @@@@@@/ .@@@@@@@@@&             @@@@@@@      .@@@@@@    /@@@@@@ @@@@@@*
             @@@@@@@@@           @@@@@@/   @@@@@@@@@@@@@*     @@@@@@@@@@ @@   .@@@@@@    /@@@@@@ @@@@@@*
           @@@@@@@@(             @@@@@@/        %@@@@@@@@@  &@@@@@@@@@@ &@ @( .@@@@@@    /@@@@@@ @@@@@@*
        @@@@@@@@@                @@@@@@/            @@@@@@. @@@@@@     @ @@.@ .@@@@@@    /@@@@@@ @@@@@@*
     .@@@@@@@@*                  @@@@@@/          .@@@@@@@  @@@@@@@* /@@@@@@@  @@@@@@@. @@@@@@@* @@@@@@*
   @@@@@@@@@     &@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@    @@@@@@@@@@@@@@@    @@@@@@@@@@@@@@   @@@@@@*
*@@@@@@@@,        @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@        .@@@@@@@@@.        &@@@@@@@@@     @@@@@@*
"#;

fn strip(s: &str) -> String {
    let r = regex::Regex::new(r"\s*\n\s*").unwrap();
    r.replace_all(s, "").to_string()
}

async fn data(t: DateTime<Utc>) -> Result<BTreeMap<String, Vec<String>>> {
    let url = format!(
        "{}/summary?t={}",
        env!("ASOUL_WEEKLY_URL"),
        t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    );
    debug!("获取归档 {}", url);
    let r = reqwest::get(url).await?.json().await?;
    Ok(r)
}

/// 返回版头，引言等
fn header() -> Vec<Element> {
    vec![
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
    ]
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
            text: "以上就是本期日报的全部内容！".to_string(),
        },
        Element::Text {
            center: false,
            strong: false,
            classes: vec!["font-size-16".to_string()],
            text: strip(
                r#"
                由于专栏格式所限，部分优秀二创内容无法展示完全。欢迎一个魂们踊跃向周报组投稿自己的内容。
                如果对枝江日报这个栏目有什么好的意见和建议可以通过私信直接向我们反馈，我们也深知目前还有很多不完善和需要改进的地方，会努力越做越好的！
            "#,
            ),
        },
        Element::raw(strip(&format!(
            r#"
            <p style="text-align: right;">
                <span class="color-gray-01 font-size-12">
                {}
                </span>
            </p>"#,
            env!("BUILD_INFO")
        ))),
    ]
}

async fn content(
    client: &reqwest::Client,
    mut summary: BTreeMap<String, Vec<String>>,
) -> Result<Vec<Element>> {
    let mut interval = tokio::time::interval(Duration::from_secs(1));

    // 分类
    let dynamics = summary.remove("动态").unwrap_or_default();
    let mut videos = summary.into_iter().collect::<Vec<_>>();
    videos.sort_unstable_by_key(|(name, _)| WEIGHT.get(name.as_str()).unwrap_or(&99999));

    // 导言
    let mut elements = header();

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

    let dynamics_elements = fetch_dynamics::download_dynamics(dynamics, client).await?;
    elements.extend(dynamics_elements);

    // 结束
    elements.extend(ending());

    Ok(elements)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("{}", README);
    println!("枝江日报生成程序 {}\n", std::env!("BUILD_INFO"));
    println!("扫码登录时如果二维码很丑，换个等宽字体");

    if log4rs::init_file("./log4rs.yml", Default::default()).is_err() {
        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "INFO");
        }
        pretty_env_logger::init_timed();
    }

    let (client, cookies) = persisted_client("./persisted_cookies.json").await?;

    match biliapi::requests::MyAccountInfo::request(&client, ()).await {
        Ok(data) => {
            info!("my account info: {:?}", data);
        }
        Err(e) => {
            warn!("not login: {:?}", e);
            info!("login now");
            login(&client, 240).await?;
            save_cookies(cookies.clone(), "./persisted_cookies.json").await?;
        }
    }

    let csrf = cookies
        .lock()
        .unwrap()
        .get("bilibili.com", "/", "bili_jct")
        .ok_or_else(|| anyhow!("missing csrf(bili_jct) cookie"))?
        .value()
        .to_string();

    // 询问
    let date_utc8 = Utc::now()
        .with_timezone(&chrono_tz::Asia::Shanghai)
        .format("%m 月 %d 日");

    let summary = data(Utc::now() - chrono::Duration::days(1)).await?;
    // 发送草稿
    let draft = Draft {
        title: format!("枝江日报（{}）", date_utc8),
        banner_url: "".to_string(),
        content: content(&client, summary).await?,
        summary: "一个简单的总结，点开草稿会自动重新生成".to_string(),
        csrf,
    };
    let r = SaveDraft::request(&client, draft).await?;
    info!("saved draft aid = {}", r.aid);

    Ok(())
}
