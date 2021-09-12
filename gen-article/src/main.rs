#[macro_use]
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate log;

mod login;
mod zhuanlan;

use anyhow::*;
use bilibili::tag_feed::*;
use biliapi::Request;
use chrono::{DateTime, Utc};
use log::*;
use std::{collections::BTreeMap, time::Duration};

use login::*;
use zhuanlan::{cards::Cards, dynamic_detail::DynamicDetail, items::Element, save_draft::*};

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

fn get_size(w: usize, h: usize) -> (usize, usize) {
    let (w, h) = (w as f64, h as f64);
    // 宽高都最高 MAX_SIZE
    let ratio = (MAX_SIZE as f64 / w).min(MAX_SIZE as f64 / h).min(1.0);
    let ans = ((ratio * w) as usize, (ratio * h) as usize);
    debug!("ratio = {:.2}, render size: {:?}", ratio, ans);
    ans
}

async fn content(
    client: &reqwest::Client,
    summary: BTreeMap<String, Vec<String>>,
) -> Result<Vec<Element>> {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    // 导言
    let mut elements = vec![
        Element::figure("https://i0.hdslb.com/bfs/article/e5dc2802adfc60c171735576f10f767939919207.jpg", 2560, 1080, 1497568, ""),
        Element::block_quote("一个魂们大家好，这里是A-SOUL周报的二创日报试行版，二创日报将收录整合有关tag中A-SOUL相关二创，尝试给大家的二创更多的曝光机会，让大家闲暇时间更方便地浏览二创，同时也记录下属于我们和A-SOUL的每一个美好的时刻。"),
        // 分割线
        Element::spacer(),
        Element::simple_figure("https://i0.hdslb.com/bfs/article/02db465212d3c374a43c60fa2625cc1caeaab796.png", "cut-off-6"),
    ];

    // 视频
    elements.push(Element::Text {
        center: true,
        strong: false,
        classes: vec!["color-pink-03".to_string(), "font-size-23".to_string()],
        text: "视频类".to_string(),
    });
    elements.push(Element::raw(strip(
        r#"<p>
        <span class="color-green-01 font-size-23">
            <span class="font-size-20">&nbsp;&nbsp;</span>
            <span class="color-blue-02 font-size-16">
                希望大家看到喜欢的
                <span class="color-lblue-02">二创作品</span>
                可以点击
                <span class="color-pink-02">作品详情</span>
                ，进入原视频评论区点赞评论一下，大家的支持是二创作者们的最大动力~
            </span>
        </span>
    </p>"#,
    )));
    for (category, bvids) in summary.iter() {
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
    // 分割线
    elements.push(Element::simple_figure(
        "https:////i0.hdslb.com/bfs/article/4adb9255ada5b97061e610b682b8636764fe50ed.png",
        "cut-off-5",
    ));
    // 动态图片
    elements.push(Element::Text {
        center: true,
        strong: false,
        classes: vec!["color-pink-03".to_string(), "font-size-23".to_string()],
        text: "美图类".to_string(),
    });
    elements.push(Element::raw(strip(
        r#"
    <p>
        <span class="color-pink-03 font-size-20">
            &nbsp;&nbsp;
            <span class="color-blue-02 font-size-16">
                希望大家看到喜欢的
                <span class="color-lblue-02">二创作品</span>
                可以点击下面的
                <span class="color-pink-02">作者ID</span>
                ，进入原动态评论区点赞评论一下，大家的支持是二创作者们的最大动力~
            </span>
        </span>
    </p>"#,
    )));
    let dynamics = summary.get("动态").cloned().unwrap_or_default();
    info!("{} 条动态", dynamics.len());

    for dynamic_url in dynamics {
        let dynamic_id = dynamic_url.replace("https://t.bilibili.com/", "");
        interval.tick().await;
        info!("获取动态信息 {}", dynamic_url);
        let info = match DynamicDetail::request(client, dynamic_id).await {
            Ok(info) => info,
            Err(e) => {
                error!(
                    "拉取动态信息失败，可能是已经删除了动态。动态链接: {}\n{:?}",
                    dynamic_url, e
                );
                continue;
            }
        };

        if info.card.desc.r#type != 2 {
            warn!("dynamic type != 2, but = {}", info.card.desc.r#type);
            continue;
        }
        let picture_dynamic = match serde_json::from_str::<PictureDynamic>(&info.card.inner) {
            Ok(picture_dynamic) => Dynamic::<PictureDynamic> {
                desc: info.card.desc,
                inner: picture_dynamic,
            },
            Err(e) => {
                warn!("type = 2，但是解析动态错误：{:?}", e);
                continue;
            }
        };
        info!(
            "获取动态信息完成 {}，开始下载图片以获取图片大小信息",
            dynamic_url
        );
        let uname = picture_dynamic.desc.user_profile.info.uname;
        let pic_src = picture_dynamic.inner.pictures[0].src.clone();
        debug!("图片链接：{}", pic_src);
        // 获取图片大小
        let r = client.get(&pic_src).send().await?;
        if let Some(size) = r.content_length() {
            if size > 1_000_000 {
                info!(
                    "图片大小：{:.2} MiB，可能需要下载一会儿",
                    size as f64 / 1024. / 1024.
                );
            }
        }
        let pic = r.bytes().await?;
        let (width, height) = match imagesize::blob_size(&pic) {
            Ok(dim) => {
                info!("动态 {} 图片大小：{:?}", dynamic_url, dim);
                get_size(dim.width, dim.height)
            }
            Err(e) => {
                warn!("无法获取图片大小: {:?}", e);
                (MAX_SIZE, MAX_SIZE)
            }
        };
        elements.push(Element::figure(
            pic_src,
            width,
            height,
            pic.len(),
            "".to_string(),
        ));
        let raw = format!(
            "<p style=\"text-align: center;\"><a href=\"{}\">↑ {}（{}图）↑</a></p>",
            dynamic_url,
            uname,
            picture_dynamic.inner.pictures.len()
        );
        elements.push(Element::raw(raw));
    }

    // 结束
    elements.push(Element::Text {
        center: true,
        strong: false,
        classes: vec!["font-size-16".to_string()],
        text: "以上就是本次日报的全部内容！".to_string(),
    });
    elements.push(Element::Text {
        center: false,
        strong: false,
        classes: vec!["font-size-16".to_string()],
        text: "由于专栏格式所限，部分优秀二创内容无法展示完全。欢迎一个魂们踊跃向周报组投稿自己的内容。如果对二创日报这个栏目有什么好的意见和建议可以通过私信直接向我们反馈，我们也深知目前还有很多不完善和需要改进的地方，会努力越做越好的！".to_string(),
    });

    // for el in elements.iter() {
    //     println!("{}", el);
    // }

    Ok(elements)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("{}", README);
    println!(
        "自动生成枝江晚报，版本：{}\n",
        std::env!("CARGO_PKG_VERSION")
    );
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
    let t = Utc::now() - chrono::Duration::days(1);

    let summary = data(t).await?;
    // 发送草稿
    let draft = Draft {
        title: "二创日报（自动生成）".to_string(),
        banner_url: "".to_string(),
        content: content(&client, summary).await?,
        summary: "一个简单的总结，点开草稿会自动重新生成".to_string(),
        csrf,
    };
    let r = SaveDraft::request(&client, draft).await?;
    info!("saved draft aid = {}", r.aid);

    Ok(())
}
