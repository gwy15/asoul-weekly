#[macro_use]
extern crate log;
use anyhow::*;
use biliapi::Request;
use gen_article as pkg;

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

#[tokio::main]
async fn main() -> Result<()> {
    println!("{}", README);
    println!("枝江日报生成程序 {}\n", std::env!("BUILD_INFO"));

    if log4rs::init_file("./log4rs.yml", Default::default()).is_err() {
        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "INFO");
        }
        pretty_env_logger::init_timed();
    }

    let (client, cookies) = pkg::persisted_client("./persisted_cookies.json").await?;

    match biliapi::requests::MyAccountInfo::request(&client, ()).await {
        Ok(data) => {
            info!("my account info: {:?}", data);
        }
        Err(e) => {
            warn!("not login: {:?}", e);
            info!("login now");
            println!("扫码登录时如果二维码很丑，换个等宽字体");
            pkg::login(&client, 240).await?;
            pkg::save_cookies(cookies.clone(), "./persisted_cookies.json").await?;
        }
    }

    let csrf = cookies
        .lock()
        .unwrap()
        .get("bilibili.com", "/", "bili_jct")
        .ok_or_else(|| anyhow!("missing csrf(bili_jct) cookie"))?
        .value()
        .to_string();

    pkg::generate(client, csrf).await?;

    Ok(())
}
